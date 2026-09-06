use lamina::error::SignalError;
use lamina::ppg::{ppg_clean, ppg_findpeaks};
use lamina::rppg::{
    ChromAlgorithm, GreenAlgorithm, OpticalSignal, PosAlgorithm, Roi, RoiSample, RppgAlgorithm,
    RppgAlgorithmId, RppgConfig, RppgWindowConfig, StaticRoi, TrackedRoiSeries, VideoFrame,
    VideoStream, extract_roi_sample, extract_rppg,
};

/// Helper to generate a deterministic synthetic RGB video stream with pulsatile modulation.
fn create_synthetic_video_stream(
    duration_sec: f64,
    fps: f64,
    pulse_freq_hz: f64,
    pulse_amp: f64,
    dark_bias: f64,
    noise_amp: f64,
) -> (VideoStream, StaticRoi) {
    let width = 40;
    let height = 40;
    let num_frames = (duration_sec * fps).round() as usize;

    let mut frames = Vec::with_capacity(num_frames);

    for i in 0..num_frames {
        let t = i as f64 / fps;
        let pulse = (2.0 * std::f64::consts::PI * pulse_freq_hz * t).sin();

        // Base color levels: Red=120, Green=150, Blue=100
        let base_g = (150.0 - dark_bias + pulse_amp * pulse).clamp(0.0, 255.0);
        let base_r = (120.0 - dark_bias + 0.3 * pulse_amp * pulse).clamp(0.0, 255.0);
        let base_b = (100.0 - dark_bias - 0.2 * pulse_amp * pulse).clamp(0.0, 255.0);

        let mut data = vec![0u8; width * height * 3];
        for pixel_idx in 0..(width * height) {
            let offset = pixel_idx * 3;
            // Simple deterministic noise based on pixel index
            let noise = if noise_amp > 0.0 {
                ((pixel_idx % 7) as f64 - 3.0) * noise_amp
            } else {
                0.0
            };
            data[offset] = (base_r + noise).clamp(0.0, 255.0) as u8;
            data[offset + 1] = (base_g + noise).clamp(0.0, 255.0) as u8;
            data[offset + 2] = (base_b + noise).clamp(0.0, 255.0) as u8;
        }

        let frame = VideoFrame::new(t, width, height, data).unwrap();
        frames.push(frame);
    }

    let stream = VideoStream::new(frames, Some(fps)).unwrap();
    let roi = StaticRoi::new(Roi::new(10, 10, 20, 20).unwrap());

    (stream, roi)
}

// ============================================================================
// Group A — Video & Timing Validation Tests
// ============================================================================

#[test]
fn test_video_frame_validation_and_malformed_buffers() {
    // Valid frame
    let data = vec![128u8; 10 * 10 * 3];
    let frame = VideoFrame::new(0.0, 10, 10, data);
    assert!(frame.is_ok());

    // Mismatched buffer length -> Err(DimensionMismatch)
    let bad_data = vec![128u8; 50];
    let frame_bad_len = VideoFrame::new(0.0, 10, 10, bad_data);
    assert!(matches!(frame_bad_len, Err(SignalError::DimensionMismatch)));

    // Zero width/height -> Err(InvalidWindowSize)
    let empty_data: Vec<u8> = Vec::new();
    let frame_zero = VideoFrame::new(0.0, 0, 10, empty_data);
    assert!(matches!(frame_zero, Err(SignalError::InvalidWindowSize(_))));

    // Non-finite timestamp -> Err(NonFiniteInput)
    let frame_nan = VideoFrame::new(f64::NAN, 10, 10, vec![128; 300]);
    assert!(matches!(frame_nan, Err(SignalError::NonFiniteInput)));
}

#[test]
fn test_video_stream_timing_validation() {
    let (mut stream, _) = create_synthetic_video_stream(2.0, 30.0, 1.2, 5.0, 0.0, 0.0);
    assert!(stream.validate_timing().is_ok());

    // Non-monotonic timestamps -> Err(UnsortedEvents)
    stream.frames[2].timestamp_sec = 0.01; // Decreasing
    assert!(matches!(
        stream.validate_timing(),
        Err(SignalError::UnsortedEvents)
    ));

    // Duplicate timestamps -> Err(UnsortedEvents)
    stream.frames[2].timestamp_sec = stream.frames[1].timestamp_sec;
    assert!(matches!(
        stream.validate_timing(),
        Err(SignalError::UnsortedEvents)
    ));
}

// ============================================================================
// Group B — ROI & Optical Signal Tests
// ============================================================================

#[test]
fn test_roi_boundary_and_validation() {
    let roi = Roi::new(5, 5, 20, 20).unwrap();
    assert!(roi.validate_for_frame(30, 30).is_ok());

    // Out of bounds -> Err(DimensionMismatch)
    assert!(matches!(
        roi.validate_for_frame(20, 20),
        Err(SignalError::DimensionMismatch)
    ));

    // Zero dimension -> Err(InvalidWindowSize)
    assert!(matches!(
        Roi::new(0, 0, 0, 10),
        Err(SignalError::InvalidWindowSize(_))
    ));
}

#[test]
fn test_roi_extraction_deterministic_mean_rgb() {
    let width = 4;
    let height = 4;
    let mut data = vec![0u8; width * height * 3];

    // Set all pixels to R=100, G=150, B=200
    for i in 0..(width * height) {
        data[i * 3] = 100;
        data[i * 3 + 1] = 150;
        data[i * 3 + 2] = 200;
    }

    let frame = VideoFrame::new(0.5, width, height, data).unwrap();
    let roi = Roi::new(1, 1, 2, 2).unwrap();

    let sample = extract_roi_sample(&frame, &roi).unwrap();
    assert_eq!(sample.timestamp_sec, 0.5);
    assert_eq!(sample.red, 100.0);
    assert_eq!(sample.green, 150.0);
    assert_eq!(sample.blue, 200.0);
    assert_eq!(sample.valid_pixels, 4);
}

#[test]
fn test_optical_signal_validation_and_subslice() {
    let samples = vec![
        RoiSample {
            timestamp_sec: 0.0,
            red: 100.0,
            green: 150.0,
            blue: 120.0,
            valid_pixels: 100,
        },
        RoiSample {
            timestamp_sec: 0.1,
            red: 101.0,
            green: 151.0,
            blue: 121.0,
            valid_pixels: 100,
        },
        RoiSample {
            timestamp_sec: 0.2,
            red: 102.0,
            green: 152.0,
            blue: 122.0,
            valid_pixels: 100,
        },
    ];

    let optical = OpticalSignal::from_samples(&samples).unwrap();
    assert_eq!(optical.timestamps_sec.len(), 3);

    let fs = optical.mean_sampling_rate().unwrap();
    assert!((fs - 10.0).abs() < 1e-6);

    let slice = optical.slice(1, 2).unwrap();
    assert_eq!(slice.timestamps_sec.len(), 2);
    assert_eq!(slice.timestamps_sec[0], 0.1);
}

// ============================================================================
// Group C — Classical rPPG Algorithms & Determinism Tests
// ============================================================================

#[test]
fn test_green_algorithm_sinusoidal_recovery() {
    let (stream, roi) = create_synthetic_video_stream(4.0, 30.0, 1.2, 10.0, 0.0, 0.0);
    let config = RppgConfig {
        algorithm: RppgAlgorithmId::GreenChannel,
        min_quality: 0.0,
        ..RppgConfig::default()
    };

    let signal = extract_rppg(&stream, &roi, &config).unwrap();
    assert_eq!(signal.algorithm, RppgAlgorithmId::GreenChannel);
    assert_eq!(signal.waveform.len(), stream.frames.len());

    // Waveform must be non-empty and finite
    for &w in &signal.waveform {
        assert!(w.is_finite());
    }
}

#[test]
fn test_chrom_algorithm_analytical_projection() {
    // Construct analytical OpticalSignal with known R_n, G_n, B_n
    let n = 100;
    let mut samples = Vec::with_capacity(n);

    for i in 0..n {
        let t = i as f64 / 30.0;
        let pulse = (2.0 * std::f64::consts::PI * 1.2 * t).sin();
        samples.push(RoiSample {
            timestamp_sec: t,
            red: 100.0 + 1.0 * pulse,
            green: 150.0 + 3.0 * pulse,
            blue: 120.0 - 0.5 * pulse,
            valid_pixels: 500,
        });
    }

    let optical = OpticalSignal::from_samples(&samples).unwrap();
    let config = RppgConfig::default();
    let algo = ChromAlgorithm;

    let pulse = algo.extract_window(&optical, &config).unwrap();
    assert_eq!(pulse.len(), n);
    for &p in &pulse {
        assert!(p.is_finite());
    }
}

#[test]
fn test_pos_algorithm_analytical_projection() {
    let n = 100;
    let mut samples = Vec::with_capacity(n);

    for i in 0..n {
        let t = i as f64 / 30.0;
        let pulse = (2.0 * std::f64::consts::PI * 1.2 * t).sin();
        samples.push(RoiSample {
            timestamp_sec: t,
            red: 100.0 + 1.0 * pulse,
            green: 150.0 + 3.0 * pulse,
            blue: 120.0 - 0.5 * pulse,
            valid_pixels: 500,
        });
    }

    let optical = OpticalSignal::from_samples(&samples).unwrap();
    let config = RppgConfig::default();
    let algo = PosAlgorithm;

    let pulse = algo.extract_window(&optical, &config).unwrap();
    assert_eq!(pulse.len(), n);
    for &p in &pulse {
        assert!(p.is_finite());
    }
}

#[test]
fn test_algorithm_determinism() {
    let (stream, roi) = create_synthetic_video_stream(5.0, 30.0, 1.2, 8.0, 0.0, 0.0);
    let config = RppgConfig {
        algorithm: RppgAlgorithmId::Pos,
        min_quality: 0.0,
        ..RppgConfig::default()
    };

    let run1 = extract_rppg(&stream, &roi, &config).unwrap();
    let run2 = extract_rppg(&stream, &roi, &config).unwrap();

    assert_eq!(run1.timestamps_sec, run2.timestamps_sec);
    assert_eq!(run1.waveform, run2.waveform);
    assert_eq!(run1.quality, run2.quality);
}

// ============================================================================
// Group D — Quality Assessment & Quality Gating Tests
// ============================================================================

#[test]
fn test_quality_dark_and_saturated_roi() {
    // Dark ROI (mean lum < 15)
    let (dark_stream, roi) = create_synthetic_video_stream(3.0, 30.0, 1.2, 1.0, 140.0, 0.0);
    let config = RppgConfig {
        min_quality: 0.0,
        ..RppgConfig::default()
    };

    let dark_signal = extract_rppg(&dark_stream, &roi, &config).unwrap();
    assert!(dark_signal.quality.overall < 0.8);

    // Saturated ROI (mean lum > 240)
    let (sat_stream, _) = create_synthetic_video_stream(3.0, 30.0, 1.2, 1.0, -140.0, 0.0);
    let sat_signal = extract_rppg(&sat_stream, &roi, &config).unwrap();
    assert!(sat_signal.quality.overall < 0.8);
}

#[test]
fn test_quality_motion_degradation() {
    let (stream, _) = create_synthetic_video_stream(3.0, 30.0, 1.2, 5.0, 0.0, 0.0);
    let num_frames = stream.frames.len();

    // Tracked ROI with large movement steps
    let mut rois = Vec::new();
    for i in 0..num_frames {
        let step = (i * 15) % 30;
        rois.push(Roi::new(step, step, 10, 10).ok());
    }

    let tracked_roi = TrackedRoiSeries::new(rois);
    let config = RppgConfig {
        min_quality: 0.0,
        ..RppgConfig::default()
    };

    let signal = extract_rppg(&stream, &tracked_roi, &config).unwrap();
    assert!(
        signal.quality.overall < 0.9,
        "Large motion displacement must reduce quality score"
    );
}

#[test]
fn test_quality_gating_and_unusable_rejection() {
    // Dark stream with strict min_quality threshold = 0.95
    let (dark_stream, roi) = create_synthetic_video_stream(3.0, 30.0, 1.2, 1.0, 140.0, 0.0);
    let config = RppgConfig {
        min_quality: 0.95, // High threshold
        ..RppgConfig::default()
    };

    let signal = extract_rppg(&dark_stream, &roi, &config).unwrap();

    // Waveform samples falling in rejected segments must be NaN (unusable)
    for &w in &signal.waveform {
        assert!(w.is_nan());
    }
    assert_eq!(signal.quality.valid_fraction, 0.0);
}

// ============================================================================
// Group E — Resampling & Stitching Tests
// ============================================================================

#[test]
fn test_rppg_signal_uniform_resampling_and_max_gap() {
    let timestamps = vec![0.0, 0.1, 0.2, 0.3, 1.5, 1.6, 1.7]; // Gap between 0.3 and 1.5 (1.2s gap > 0.5s max_gap)
    let waveform = vec![0.0, 1.0, 0.0, -1.0, 0.0, 1.0, 0.0];

    let rppg_signal = lamina::rppg::RppgSignal {
        timestamps_sec: timestamps,
        waveform,
        sampling_rate_hz: 10.0,
        quality: lamina::rppg::RppgQualitySummary {
            overall: 1.0,
            valid_fraction: 1.0,
            segments: Vec::new(),
        },
        algorithm: RppgAlgorithmId::Pos,
    };

    let resampled = rppg_signal.resample_uniform(20.0, 0.5).unwrap();
    assert_eq!(resampled.sampling_rate_hz, 20.0);

    // Points inside the 1.2s gap (> 0.5s max_gap) must be NaN
    let gap_sample = resampled
        .timestamps_sec
        .iter()
        .zip(resampled.waveform.iter())
        .find(|&(t, _)| *t > 0.5 && *t < 1.3);

    assert!(gap_sample.is_some());
    assert!(gap_sample.unwrap().1.is_nan());

    // Test direct GreenAlgorithm trait method
    assert_eq!(GreenAlgorithm.id(), RppgAlgorithmId::GreenChannel);
}

// ============================================================================
// Group F — Downstream Integration with Lamina PPG Processing
// ============================================================================

#[test]
fn test_downstream_lamina_ppg_integration() {
    let (stream, roi) = create_synthetic_video_stream(10.0, 30.0, 1.2, 10.0, 0.0, 0.0); // 1.2 Hz pulse (~72 BPM)
    let config = RppgConfig {
        algorithm: RppgAlgorithmId::Pos,
        min_quality: 0.0,
        window: RppgWindowConfig {
            window_sec: 4.0,
            step_sec: 1.0,
        },
        ..RppgConfig::default()
    };

    let rppg_signal = extract_rppg(&stream, &roi, &config).unwrap();
    let resampled = rppg_signal.resample_uniform(30.0, 1.0).unwrap();

    // 1. Convert to Array1<f64>
    let raw_arr = resampled.to_ndarray();

    // 2. Pass into lamina::ppg::ppg_clean
    let cleaned = ppg_clean(&raw_arr, resampled.sampling_rate_hz).unwrap();
    assert_eq!(cleaned.len(), raw_arr.len());

    // 3. Pass into lamina::ppg::ppg_findpeaks
    let peaks_mask = ppg_findpeaks(&cleaned, resampled.sampling_rate_hz).unwrap();
    let peak_count = peaks_mask.iter().filter(|&&p| p).count();

    // Over 10s at 1.2 Hz (~72 BPM), expected ~12 peaks
    assert!(
        (8..=15).contains(&peak_count),
        "rPPG signal passed through lamina::ppg must recover ~12 pulse peaks, got {}",
        peak_count
    );
}
