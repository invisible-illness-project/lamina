use lamina::error::SignalError;
use lamina::ppg::{ppg_clean, ppg_findpeaks};
use lamina::rppg::{
    ChromAlgorithm, GreenAlgorithm, OpticalSignal, PosAlgorithm, Roi, RoiSample, RppgAlgorithm,
    RppgAlgorithmId, RppgConfig, RppgPreprocessingConfig, RppgWindowConfig, StaticRoi,
    TrackedRoiSeries, VideoFrame, VideoStream, extract_roi_sample, extract_rppg,
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
        window: RppgWindowConfig {
            window_sec: 3.0,
            step_sec: 0.5,
            min_window_fraction: 0.8,
        },
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
        window: RppgWindowConfig {
            window_sec: 3.0,
            step_sec: 0.5,
            min_window_fraction: 0.8,
        },
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
        window: RppgWindowConfig {
            window_sec: 2.0,
            step_sec: 0.5,
            min_window_fraction: 0.8,
        },
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
        window: RppgWindowConfig {
            window_sec: 2.0,
            step_sec: 0.5,
            min_window_fraction: 0.8,
        },
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
        window: RppgWindowConfig {
            window_sec: 2.0,
            step_sec: 0.5,
            min_window_fraction: 0.8,
        },
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
// Group F — Downstream Integration & Task 1.1 Hardening Tests
// ============================================================================

#[test]
fn test_timestamp_window_selection_irregular_frames() {
    let irregular_timestamps = vec![
        0.000, 0.033, 0.071, 0.110, 0.160, 0.210, 0.260, 0.310, 0.360,
    ];
    let samples: Vec<RoiSample> = irregular_timestamps
        .iter()
        .map(|&t| RoiSample {
            timestamp_sec: t,
            red: 100.0,
            green: 150.0,
            blue: 120.0,
            valid_pixels: 500,
        })
        .collect();

    let optical = OpticalSignal::from_samples(&samples).unwrap();
    let (start_idx, end_idx) = optical.timestamp_range(0.05, 0.25).unwrap();
    assert_eq!(start_idx, 2); // 0.071 is first >= 0.05
    assert_eq!(end_idx, 6); // 0.260 is first >= 0.25
}

#[test]
fn test_window_local_preprocessing() {
    let n = 30;
    let samples: Vec<RoiSample> = (0..n)
        .map(|i| {
            let t = i as f64 / 10.0;
            // Introduce linear ramp trend: 2.0 * t
            RoiSample {
                timestamp_sec: t,
                red: 100.0 + 2.0 * t,
                green: 150.0 + 2.0 * t,
                blue: 120.0 + 2.0 * t,
                valid_pixels: 500,
            }
        })
        .collect();

    let optical = OpticalSignal::from_samples(&samples).unwrap();

    let config_detrend = RppgPreprocessingConfig {
        normalize_channels: false,
        detrend: true,
    };
    let preprocessed = optical.preprocess(&config_detrend).unwrap();

    // Detrending linear ramp leaves red channel nearly flat around mean
    let mean_orig = optical.red.iter().sum::<f64>() / n as f64;
    let mean_detrend = preprocessed.red.iter().sum::<f64>() / n as f64;
    let var_orig = optical
        .red
        .iter()
        .map(|&x| (x - mean_orig).powi(2))
        .sum::<f64>()
        / n as f64;
    let var_detrend = preprocessed
        .red
        .iter()
        .map(|&x| (x - mean_detrend).powi(2))
        .sum::<f64>()
        / n as f64;
    assert!(var_detrend < var_orig * 0.01);
}

#[test]
fn test_gap_aware_valid_segments() {
    let timestamps = vec![
        0.0, 0.1, 0.2, 0.3, // Segment 1 (0.3s)
        2.5, 2.6, 2.7, 2.8, // Segment 2 (gap 2.2s > max_gap 1.0s)
    ];
    let waveform = vec![1.0, 2.0, 1.0, 0.0, 3.0, 4.0, 3.0, 2.0];

    let signal = lamina::rppg::RppgSignal {
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

    let segments = signal.valid_segments(1.0).unwrap();
    assert_eq!(segments.len(), 2);
    assert_eq!(segments[0].waveform.len(), 4);
    assert_eq!(segments[1].waveform.len(), 4);
    assert_eq!(segments[0].start_sec, 0.0);
    assert_eq!(segments[1].start_sec, 2.5);
}

#[test]
fn test_valid_duration_fraction_unequal_lengths() {
    let (stream, roi) = create_synthetic_video_stream(10.0, 30.0, 1.2, 10.0, 0.0, 0.0);
    let config = RppgConfig {
        algorithm: RppgAlgorithmId::GreenChannel,
        min_quality: 0.0,
        window: RppgWindowConfig {
            window_sec: 3.0,
            step_sec: 1.0,
            min_window_fraction: 0.8,
        },
        ..RppgConfig::default()
    };

    let signal = extract_rppg(&stream, &roi, &config).unwrap();
    assert!(signal.quality.valid_fraction > 0.8);
    assert!(signal.quality.valid_fraction <= 1.0);
}

#[test]
fn test_downstream_ppg_integration_via_valid_segments() {
    let (stream, roi) = create_synthetic_video_stream(10.0, 30.0, 1.2, 10.0, 0.0, 0.0);
    let config = RppgConfig {
        algorithm: RppgAlgorithmId::Pos,
        min_quality: 0.0,
        window: RppgWindowConfig {
            window_sec: 4.0,
            step_sec: 1.0,
            min_window_fraction: 0.8,
        },
        ..RppgConfig::default()
    };

    let rppg_signal = extract_rppg(&stream, &roi, &config).unwrap();
    let valid_segs = rppg_signal.valid_segments(1.0).unwrap();
    assert!(!valid_segs.is_empty());

    for seg in valid_segs {
        let arr = seg.to_ndarray();
        let cleaned = ppg_clean(&arr, seg.sampling_rate_hz).unwrap();
        let peaks = ppg_findpeaks(&cleaned, seg.sampling_rate_hz).unwrap();
        assert_eq!(cleaned.len(), arr.len());
        assert_eq!(peaks.len(), arr.len());
    }
}

#[test]
fn test_valid_duration_fraction_overlapping_windows_no_double_counting() {
    let (stream, roi) = create_synthetic_video_stream(4.0, 30.0, 1.2, 10.0, 0.0, 0.0);
    let config = RppgConfig {
        algorithm: RppgAlgorithmId::Pos,
        min_quality: 0.0,
        window: RppgWindowConfig {
            window_sec: 3.0,
            step_sec: 0.5, // 83% overlap
            min_window_fraction: 0.8,
        },
        ..RppgConfig::default()
    };

    let signal = extract_rppg(&stream, &roi, &config).unwrap();
    // Overlapping windows covering 4s recording duration must yield valid_fraction <= 1.0 without unmerged sum overflow
    assert!(signal.quality.valid_fraction <= 1.0);
    assert!((signal.quality.valid_fraction - 1.0).abs() < 1e-3);
}

#[test]
fn test_signal_quality_custom_band_lag_bounds() {
    let fs = 30.0;
    let band = (0.5, 3.0); // 30-180 BPM
    let waveform: Vec<f64> = (0..100)
        .map(|i| (2.0 * std::f64::consts::PI * 1.5 * (i as f64 / fs)).sin())
        .collect();

    let q = lamina::rppg::quality::assess_signal_quality(&waveform, fs, band);
    assert!(q > 0.5);

    // Invalid Nyquist band returns 0.0
    let q_invalid = lamina::rppg::quality::assess_signal_quality(&waveform, fs, (0.5, 20.0));
    assert_eq!(q_invalid, 0.0);
}

#[test]
fn test_valid_segments_invalid_max_gap_returns_error() {
    let signal = lamina::rppg::RppgSignal {
        timestamps_sec: vec![0.0, 0.1, 0.2, 0.3],
        waveform: vec![1.0, 2.0, 1.0, 0.0],
        sampling_rate_hz: 10.0,
        quality: lamina::rppg::RppgQualitySummary {
            overall: 1.0,
            valid_fraction: 1.0,
            segments: Vec::new(),
        },
        algorithm: RppgAlgorithmId::Pos,
    };

    assert!(matches!(
        signal.valid_segments(-0.5),
        Err(SignalError::InvalidWindowSize(_))
    ));
    assert!(matches!(
        signal.valid_segments(f64::NAN),
        Err(SignalError::InvalidWindowSize(_))
    ));
}

#[test]
fn test_internal_frame_gap_invalidates_window() {
    let (mut stream, roi) = create_synthetic_video_stream(10.0, 30.0, 1.2, 10.0, 0.0, 0.0);
    // Introduce a large 2.0s internal gap at frame 150 (t = 5.0s -> t = 7.033s)
    for frame in stream.frames.iter_mut().skip(151) {
        frame.timestamp_sec += 2.0;
    }

    let config = RppgConfig {
        algorithm: RppgAlgorithmId::Pos,
        min_quality: 0.4,
        max_gap_sec: 0.5, // 0.5s max gap threshold
        window: RppgWindowConfig {
            window_sec: 3.0,
            step_sec: 0.5,
            min_window_fraction: 0.8,
        },
        ..RppgConfig::default()
    };

    let signal = extract_rppg(&stream, &roi, &config).unwrap();

    // 1. Verify quality metadata: windows spanning the 2.0s gap must be invalidated (overall == 0.0)
    let gap_quality_windows = signal
        .quality
        .segments
        .iter()
        .filter(|q| q.start_sec <= 5.0 && q.end_sec >= 7.0);
    for q in gap_quality_windows {
        assert_eq!(
            q.overall, 0.0,
            "Quality window spanning internal gap > max_gap_sec must have 0.0 quality"
        );
    }

    // 2. Check boundary waveform samples before (<= 4.0s) and after (>= 8.0s) the gap are finite
    let before_gap_samples = signal
        .timestamps_sec
        .iter()
        .zip(signal.waveform.iter())
        .filter(|&(t, _)| *t >= 0.0 && *t <= 4.0);
    for (_, w) in before_gap_samples {
        assert!(!w.is_nan(), "Samples before gap must be finite");
    }

    let after_gap_samples = signal
        .timestamps_sec
        .iter()
        .zip(signal.waveform.iter())
        .filter(|&(t, _)| *t >= 8.0 && *t <= 11.5);
    for (_, w) in after_gap_samples {
        assert!(!w.is_nan(), "Samples after gap must be finite");
    }

    // 3. Verify valid_segments() extracts exactly two distinct contiguous temporal segments
    let valid_segs = signal.valid_segments(config.max_gap_sec).unwrap();
    assert_eq!(
        valid_segs.len(),
        2,
        "Timestamp gap must split signal into two contiguous valid segments"
    );
    assert!(valid_segs[0].end_sec <= 5.1);
    assert!(valid_segs[1].start_sec >= 6.9);
    for seg in &valid_segs {
        for w in &seg.waveform {
            assert!(
                !w.is_nan(),
                "Valid segments must contain non-NaN waveform samples"
            );
        }
    }
}

#[test]
fn test_piecewise_elementary_quality_aggregation_no_multiplicity_bias() {
    use lamina::rppg::RppgSegmentQuality;

    // Two overlapping windows with conflicting quality (0.2 vs 0.8)
    let quality_segs = vec![
        RppgSegmentQuality {
            start_sec: 0.0,
            end_sec: 3.0,
            overall: 0.2,
            roi_quality: 0.2,
            motion_quality: 0.2,
            illumination_quality: 0.2,
            signal_quality: 0.2,
            valid_fraction: 1.0,
        },
        RppgSegmentQuality {
            start_sec: 0.5,
            end_sec: 3.5,
            overall: 0.8,
            roi_quality: 0.8,
            motion_quality: 0.8,
            illumination_quality: 0.8,
            signal_quality: 0.8,
            valid_fraction: 1.0,
        },
    ];

    let signal = lamina::rppg::RppgSignal {
        timestamps_sec: vec![0.0, 1.0, 2.0, 3.0],
        waveform: vec![1.0, 2.0, 1.0, 0.0],
        sampling_rate_hz: 1.0,
        quality: lamina::rppg::RppgQualitySummary {
            overall: 0.5,
            valid_fraction: 1.0,
            segments: quality_segs,
        },
        algorithm: RppgAlgorithmId::Pos,
    };

    let segs = signal.valid_segments(1.0).unwrap();
    assert_eq!(segs.len(), 1);

    // Over segment [0.0, 3.0]:
    // Interval [0.0, 0.5): length 0.5, active {W1 (0.2)} => mean 0.2, contrib 0.1
    // Interval [0.5, 3.0): length 2.5, active {W1 (0.2), W2 (0.8)} => mean 0.5, contrib 1.25
    // Integrated quality = (0.1 + 1.25) / 3.0 = 0.45 (vs naive overlap weighting 0.4727)
    assert!(
        (segs[0].quality.overall - 0.45).abs() < 1e-3,
        "Piecewise quality integration must produce 0.45, got {}",
        segs[0].quality.overall
    );

    // Also test uncovered sub-interval quality fallback (active_cnt == 0 => 0.0 quality contribution)
    let uncovered_quality_segs = vec![RppgSegmentQuality {
        start_sec: 0.0,
        end_sec: 2.0,
        overall: 0.8,
        roi_quality: 0.8,
        motion_quality: 0.8,
        illumination_quality: 0.8,
        signal_quality: 0.8,
        valid_fraction: 1.0,
    }];

    let uncovered_signal = lamina::rppg::RppgSignal {
        timestamps_sec: vec![0.0, 1.0, 2.0, 3.0],
        waveform: vec![1.0, 2.0, 1.0, 0.0],
        sampling_rate_hz: 1.0,
        quality: lamina::rppg::RppgQualitySummary {
            overall: 0.8,
            valid_fraction: 1.0,
            segments: uncovered_quality_segs,
        },
        algorithm: RppgAlgorithmId::Pos,
    };

    let uncovered_segs = uncovered_signal.valid_segments(1.0).unwrap();
    assert_eq!(uncovered_segs.len(), 1);

    // Over segment [0.0, 3.0]:
    // Interval [0.0, 2.0): length 2.0, active {W1 (0.8)} => contrib 1.6
    // Interval [2.0, 3.0): length 1.0, active {} => contrib 0.0
    // Integrated quality = 1.6 / 3.0 = 0.5333...
    assert!(
        (uncovered_segs[0].quality.overall - 1.6 / 3.0).abs() < 1e-3,
        "Uncovered sub-interval must contribute 0.0 quality, got {}",
        uncovered_segs[0].quality.overall
    );
    assert!((uncovered_segs[0].quality.roi_quality - 1.6 / 3.0).abs() < 1e-3);
}

#[test]
fn test_signal_polarity_and_bvp_waveform() {
    use lamina::rppg::SignalPolarity;

    let timestamps = vec![0.0, 0.1, 0.2, 0.3, 0.4];
    let raw_waveform = vec![1.0, 2.0, 0.5, 3.0, 0.0];

    let rppg = lamina::rppg::RppgSignal {
        timestamps_sec: timestamps.clone(),
        waveform: raw_waveform.clone(),
        sampling_rate_hz: 10.0,
        quality: lamina::rppg::RppgQualitySummary {
            overall: 1.0,
            valid_fraction: 1.0,
            segments: Vec::new(),
        },
        algorithm: RppgAlgorithmId::Pos,
    };

    // Normal polarity keeps waveform unchanged
    let bvp_normal = rppg.to_bvp_waveform(SignalPolarity::Normal);
    assert_eq!(bvp_normal.waveform, raw_waveform);

    // Inverted polarity negates waveform
    let bvp_inv = rppg.to_bvp_waveform(SignalPolarity::Inverted);
    let expected_inv: Vec<f64> = raw_waveform.iter().map(|v| -v).collect();
    assert_eq!(bvp_inv.waveform, expected_inv);

    // BvpWaveform to_ndarray conversion
    let arr = bvp_normal.to_ndarray();
    assert_eq!(arr.len(), 5);
}

#[test]
fn test_rppg_polarity_contract_and_autodetect_boundaries() {
    use lamina::rppg::{RppgAlgorithmId, RppgQualitySummary, RppgSignal, SignalPolarity};

    let timestamps = vec![0.0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7];
    let dummy_quality = RppgQualitySummary {
        overall: 1.0,
        valid_fraction: 1.0,
        segments: Vec::new(),
    };

    // 1. Explicit Normal & Explicit Inverted (Deterministic Physical Contracts)
    let raw_optical = vec![0.0, 0.1, -1.5, 0.2, 0.0, 0.1, -1.4, 0.2];
    let rppg_abs = RppgSignal {
        timestamps_sec: timestamps.clone(),
        waveform: raw_optical.clone(),
        sampling_rate_hz: 10.0,
        quality: dummy_quality.clone(),
        algorithm: RppgAlgorithmId::Chrom,
    };

    let bvp_norm = rppg_abs.to_bvp_waveform(SignalPolarity::Normal);
    assert_eq!(
        bvp_norm.waveform, raw_optical,
        "Explicit Normal must preserve waveform exactly"
    );

    let bvp_inv = rppg_abs.to_bvp_waveform(SignalPolarity::Inverted);
    let expected_inv: Vec<f64> = raw_optical.iter().map(|v| -v).collect();
    assert_eq!(
        bvp_inv.waveform, expected_inv,
        "Explicit Inverted must negate waveform exactly"
    );

    // 2. Right-skewed positive pulse waveform
    // Baseline = 0.0 with positive peaks = 5.0 (skew > 0.3)
    let pos_pulse = vec![0.0, 0.0, 0.0, 5.0, 0.0, 0.0, 0.0, 5.0];
    let rppg_pos = RppgSignal {
        timestamps_sec: timestamps.clone(),
        waveform: pos_pulse.clone(),
        sampling_rate_hz: 10.0,
        quality: dummy_quality.clone(),
        algorithm: RppgAlgorithmId::Pos,
    };

    // Explicit Normal preserves positive pulse waveform
    let bvp_pos_norm = rppg_pos.to_bvp_waveform(SignalPolarity::Normal);
    assert_eq!(bvp_pos_norm.waveform, pos_pulse);

    // AutoDetect evaluates skew > 0.3 and flips signal (documenting heuristic boundary)
    let bvp_pos_auto = rppg_pos.to_bvp_waveform(SignalPolarity::AutoDetect);
    let expected_pos_flipped: Vec<f64> = pos_pulse.iter().map(|v| -v).collect();
    assert_eq!(
        bvp_pos_auto.waveform, expected_pos_flipped,
        "AutoDetect skewness heuristic flips positive pulse due to positive skewness"
    );

    // 3. Degenerate / near-zero variance input (std <= 1e-6)
    let constant_wave = vec![1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0];
    let rppg_const = RppgSignal {
        timestamps_sec: timestamps.clone(),
        waveform: constant_wave.clone(),
        sampling_rate_hz: 10.0,
        quality: dummy_quality,
        algorithm: RppgAlgorithmId::GreenChannel,
    };

    let bvp_const_auto = rppg_const.to_bvp_waveform(SignalPolarity::AutoDetect);
    assert_eq!(
        bvp_const_auto.waveform, constant_wave,
        "Degenerate constant input must not be flipped by AutoDetect"
    );
}
