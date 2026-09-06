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
pub use signal::{OpticalSignal, RoiSample, RppgSignal};
pub use video::{VideoFrame, VideoMetadata, VideoStream};

use crate::error::{Result, SignalError};

/// Primary entry point for extracting a quality-characterized rPPG pulse signal from a video stream.
///
/// # Pipeline Workflow
/// 1. Validates video stream frame timing, dimensions, and rPPG configuration parameters.
/// 2. Extracts spatial mean RGB intensity samples (`RoiSample`) from frame ROIs supplied by `roi_provider`.
/// 3. Assembles a temporal `OpticalSignal` series.
/// 4. Slices the optical signal into sliding temporal windows (`RppgWindowConfig`).
/// 5. Applies the selected classical algorithm (`GreenChannel`, `Chrom`, `Pos`) to extract windowed pulse traces.
/// 6. Evaluates multi-tiered segment-level quality (`RppgSegmentQuality`) for ROI, motion, illumination, and periodicity.
/// 7. Combines windowed pulse traces using quality-weighted overlap-add stitching.
/// 8. Returns a standardized `RppgSignal` paired with an `RppgQualitySummary`.
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

    // 2. Determine windowing sample bounds
    let win_samples = (config.window.window_sec * fs).round() as usize;
    let step_samples = (config.window.step_sec * fs).round() as usize;

    let win_samples = win_samples.max(4).min(total_samples);
    let step_samples = step_samples.max(1).min(win_samples);

    // Instantiate selected rPPG extraction algorithm
    let algo: Box<dyn RppgAlgorithm> = match config.algorithm {
        RppgAlgorithmId::GreenChannel => Box::new(GreenAlgorithm),
        RppgAlgorithmId::Chrom => Box::new(ChromAlgorithm),
        RppgAlgorithmId::Pos => Box::new(PosAlgorithm),
    };

    // 3. Sliding window extraction and quality evaluation
    let mut stitched_waveform = vec![0.0f64; total_samples];
    let mut weight_accumulator = vec![0.0f64; total_samples];
    let mut segment_qualities = Vec::new();

    let mut start_idx = 0usize;
    while start_idx < total_samples {
        let end_idx = (start_idx + win_samples).min(total_samples);
        let current_len = end_idx - start_idx;
        if current_len < 4 {
            break;
        }

        let win_optical = optical.slice(start_idx, current_len)?;
        let win_displacements = &displacements[start_idx..end_idx];

        // Extract window pulse waveform
        let win_pulse_res = algo.extract_window(&win_optical, config);

        let (win_pulse, seg_q) = match win_pulse_res {
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
                    start_sec: win_optical.timestamps_sec[0],
                    end_sec: *win_optical.timestamps_sec.last().unwrap(),
                    overall: 0.0,
                    roi_quality: 0.0,
                    motion_quality: 0.0,
                    illumination_quality: 0.0,
                    signal_quality: 0.0,
                    valid_fraction: 0.0,
                };
                (vec![0.0; current_len], q)
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

        if end_idx == total_samples {
            break;
        }
        start_idx += step_samples;
    }

    // Normalize overlap-add waveform or mark gap with NaN
    for i in 0..total_samples {
        if weight_accumulator[i] > 0.0 {
            stitched_waveform[i] /= weight_accumulator[i];
        } else {
            stitched_waveform[i] = f64::NAN;
        }
    }

    // 4. Build recording-wide quality summary
    let overall_quality = if !segment_qualities.is_empty() {
        segment_qualities.iter().map(|q| q.overall).sum::<f64>() / segment_qualities.len() as f64
    } else {
        0.0
    };

    let valid_count = segment_qualities
        .iter()
        .filter(|q| q.overall >= config.min_quality)
        .count();
    let valid_fraction = if !segment_qualities.is_empty() {
        valid_count as f64 / segment_qualities.len() as f64
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
