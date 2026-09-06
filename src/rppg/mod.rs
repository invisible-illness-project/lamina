pub mod algorithms;
pub mod chrom;
pub mod config;
pub mod pos;
pub mod quality;
pub mod roi;
pub mod signal;
pub mod video;

pub use algorithms::{GreenAlgorithm, RppgAlgorithm};
pub use chrom::ChromAlgorithm;
pub use config::{RppgAlgorithmId, RppgConfig, RppgPreprocessingConfig, RppgWindowConfig};
pub use pos::PosAlgorithm;
pub use quality::{
    RppgQualitySummary, RppgSegmentQuality, assess_illumination_quality, assess_motion_quality,
    assess_roi_quality, assess_signal_quality, evaluate_segment_quality,
};
pub use roi::{Roi, RoiProvider, StaticRoi, TrackedRoiSeries, extract_roi_sample};
pub use signal::{OpticalSignal, RoiSample, RppgSegment, RppgSignal};
pub use video::{VideoFrame, VideoMetadata, VideoStream};

use crate::error::{Result, SignalError};

/// Primary entry point for extracting a quality-characterized rPPG pulse signal from a video stream.
///
/// # Pipeline Workflow
/// 1. Validates video stream frame timing, dimensions, and rPPG configuration parameters.
/// 2. Extracts spatial mean RGB intensity samples (`RoiSample`) from frame ROIs supplied by `roi_provider`.
/// 3. Assembles a temporal `OpticalSignal` series.
/// 4. Slices the optical signal into physical-time sliding windows (`RppgWindowConfig`) via zero-copy index lookup (`timestamp_range`).
/// 5. Applies window-local preprocessing (linear detrending and channel mean normalization).
/// 6. Applies the selected classical algorithm (`GreenChannel`, `Chrom`, `Pos`) to extract windowed pulse traces.
/// 7. Evaluates multi-tiered segment-level quality (`RppgSegmentQuality`) for ROI, motion, illumination, and periodicity.
/// 8. Combines windowed pulse traces using quality-weighted overlap-add stitching.
/// 9. Returns a standardized `RppgSignal` paired with an `RppgQualitySummary`.
pub fn extract_rppg(
    video: &VideoStream,
    roi_provider: &dyn RoiProvider,
    config: &RppgConfig,
) -> Result<RppgSignal> {
    config.validate()?;
    video.validate_timing()?;

    // 1. Extract ROI RGB samples across valid frames
    let mut roi_samples = Vec::with_capacity(video.frames.len());
    let mut displacements = Vec::with_capacity(video.frames.len());

    for (i, frame) in video.frames.iter().enumerate() {
        if let Some(roi) = roi_provider.roi_for_frame(i, frame.timestamp_sec) {
            let sample = extract_roi_sample(frame, &roi)?;
            let disp = roi_provider.displacement_at_frame(i);
            roi_samples.push(sample);
            displacements.push(disp);
        }
    }

    if roi_samples.is_empty() {
        return Err(SignalError::EmptySignal);
    }

    let optical = OpticalSignal::from_samples(&roi_samples)?;
    let total_samples = optical.timestamps_sec.len();
    let fs = optical.mean_sampling_rate()?;

    // Instantiate selected rPPG extraction algorithm
    let algo: Box<dyn RppgAlgorithm> = match config.algorithm {
        RppgAlgorithmId::GreenChannel => Box::new(GreenAlgorithm),
        RppgAlgorithmId::Chrom => Box::new(ChromAlgorithm),
        RppgAlgorithmId::Pos => Box::new(PosAlgorithm),
    };

    // 2. Physical-time sliding window extraction
    let mut stitched_waveform = vec![0.0f64; total_samples];
    let mut weight_accumulator = vec![0.0f64; total_samples];
    let mut segment_qualities = Vec::new();

    let t_first = optical.timestamps_sec[0];
    let t_last = *optical.timestamps_sec.last().unwrap();
    let total_duration = (t_last - t_first).max(0.0);

    let mut win_start_t = t_first;
    while win_start_t < t_last {
        let win_end_t = (win_start_t + config.window.window_sec).min(t_last + 1e-6);
        let (start_idx, end_idx) = optical.timestamp_range(win_start_t, win_end_t)?;
        let current_len = end_idx.saturating_sub(start_idx);

        if current_len < 4 {
            if end_idx >= total_samples {
                break;
            }
            win_start_t += config.window.step_sec;
            continue;
        }

        // Coverage duration check
        let observed_span = optical.timestamps_sec[end_idx - 1] - optical.timestamps_sec[start_idx];
        let dt_sample = if current_len > 1 {
            observed_span / (current_len - 1) as f64
        } else {
            0.0
        };
        let coverage_sec = observed_span + dt_sample;
        let coverage_ratio = coverage_sec / config.window.window_sec;

        let win_optical_raw = optical.slice(start_idx, current_len)?;
        let win_displacements = &displacements[start_idx..end_idx];

        let (win_pulse, seg_q) = if coverage_ratio < config.window.min_window_fraction {
            // Window duration insufficient to meet min_window_fraction coverage criteria
            let q = RppgSegmentQuality {
                start_sec: win_optical_raw.timestamps_sec[0],
                end_sec: *win_optical_raw.timestamps_sec.last().unwrap(),
                overall: 0.0,
                roi_quality: 0.0,
                motion_quality: 0.0,
                illumination_quality: 0.0,
                signal_quality: 0.0,
                valid_fraction: 0.0,
            };
            (vec![0.0; current_len], q)
        } else {
            // Apply window-local preprocessing
            match win_optical_raw.preprocess(&config.preprocessing) {
                Ok(win_optical) => match algo.extract_window(&win_optical, config) {
                    Ok(p) => {
                        let q = evaluate_segment_quality(
                            &win_optical,
                            &p,
                            win_displacements,
                            config.minimum_roi_pixels,
                            config.signal_band_hz,
                        );
                        (p, q)
                    }
                    Err(_) => {
                        let q = RppgSegmentQuality {
                            start_sec: win_optical_raw.timestamps_sec[0],
                            end_sec: *win_optical_raw.timestamps_sec.last().unwrap(),
                            overall: 0.0,
                            roi_quality: 0.0,
                            motion_quality: 0.0,
                            illumination_quality: 0.0,
                            signal_quality: 0.0,
                            valid_fraction: 0.0,
                        };
                        (vec![0.0; current_len], q)
                    }
                },
                Err(_) => {
                    let q = RppgSegmentQuality {
                        start_sec: win_optical_raw.timestamps_sec[0],
                        end_sec: *win_optical_raw.timestamps_sec.last().unwrap(),
                        overall: 0.0,
                        roi_quality: 0.0,
                        motion_quality: 0.0,
                        illumination_quality: 0.0,
                        signal_quality: 0.0,
                        valid_fraction: 0.0,
                    };
                    (vec![0.0; current_len], q)
                }
            }
        };

        segment_qualities.push(seg_q.clone());

        // Quality gating check: if segment meets min_quality, contribute to overlap-add
        if seg_q.overall >= config.min_quality {
            let weight = seg_q.overall;
            for (k, &pulse_val) in win_pulse.iter().enumerate().take(current_len) {
                let idx = start_idx + k;
                stitched_waveform[idx] += weight * pulse_val;
                weight_accumulator[idx] += weight;
            }
        }

        if end_idx >= total_samples {
            break;
        }
        win_start_t += config.window.step_sec;
    }

    // Normalize overlap-add waveform or mark gap with NaN
    for i in 0..total_samples {
        if weight_accumulator[i] > 0.0 {
            stitched_waveform[i] /= weight_accumulator[i];
        } else {
            stitched_waveform[i] = f64::NAN;
        }
    }

    // 3. Build recording-wide quality summary
    let overall_quality = if !segment_qualities.is_empty() {
        segment_qualities.iter().map(|q| q.overall).sum::<f64>() / segment_qualities.len() as f64
    } else {
        0.0
    };

    // Single-pass forward merging of chronologically generated valid intervals [start_sec, end_sec)
    let mut merged_intervals: Vec<(f64, f64)> = Vec::new();
    for q in segment_qualities
        .iter()
        .filter(|q| q.overall >= config.min_quality)
    {
        if let Some(last) = merged_intervals.last_mut() {
            if q.start_sec <= last.1 + 1e-6 {
                last.1 = last.1.max(q.end_sec);
            } else {
                merged_intervals.push((q.start_sec, q.end_sec));
            }
        } else {
            merged_intervals.push((q.start_sec, q.end_sec));
        }
    }

    let valid_duration: f64 = merged_intervals.iter().map(|(a, b)| (b - a).max(0.0)).sum();

    let valid_fraction = if total_duration > 0.0 {
        (valid_duration / total_duration).clamp(0.0, 1.0)
    } else {
        0.0
    };

    let summary = RppgQualitySummary {
        overall: overall_quality,
        valid_fraction,
        segments: segment_qualities,
    };

    Ok(RppgSignal {
        timestamps_sec: optical.timestamps_sec,
        waveform: stitched_waveform,
        sampling_rate_hz: fs,
        quality: summary,
        algorithm: config.algorithm,
    })
}
