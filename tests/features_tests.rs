use lamina::ecg::ecg_findpeaks;
use lamina::eda::{
    EdaDecompositionConfig, EdaPeakDetectionConfig, ScrEvent, eda_decompose, eda_findpeaks_events,
};
use lamina::error::SignalError;
use lamina::features::{
    FeatureConfig, MultimodalInput, WindowConfig, cardiac_features, coupling_features,
    eda_features, extract_features, extract_features_naive, generate_windows, respiration_features,
    time_range_to_sample_range,
};
use lamina::ppg::ppg_findpeaks;
use lamina::rsp::{RespirationCycle, rsp_clean, rsp_cycles};
use ndarray::Array1;
use std::f64::consts::PI;

fn mask_to_indices(mask: &Array1<bool>) -> Vec<usize> {
    mask.iter()
        .enumerate()
        .filter_map(|(i, &p)| if p { Some(i) } else { None })
        .collect()
}

// ============================================================================
// Group A — Windowing & Time-Range Conversion Tests
// ============================================================================

#[test]
fn test_generate_windows_fixed_and_overlapping() {
    let cfg = WindowConfig {
        window_duration_sec: 60.0,
        step_sec: 30.0,
        min_coverage: 0.80,
    };

    let wins = generate_windows(0.0, 120.0, &cfg).unwrap();
    assert_eq!(wins.len(), 3);
    assert_eq!(wins[0].start_time_sec, 0.0);
    assert_eq!(wins[0].end_time_sec, 60.0);
    assert_eq!(wins[1].start_time_sec, 30.0);
    assert_eq!(wins[1].end_time_sec, 90.0);
    assert_eq!(wins[2].start_time_sec, 60.0);
    assert_eq!(wins[2].end_time_sec, 120.0);
}

#[test]
fn test_generate_windows_invalid_configs() {
    let valid_cfg = WindowConfig::default();

    assert!(matches!(
        generate_windows(
            0.0,
            10.0,
            &WindowConfig {
                window_duration_sec: -10.0,
                ..valid_cfg.clone()
            }
        ),
        Err(SignalError::InvalidWindowSize(_))
    ));
    assert!(matches!(
        generate_windows(
            0.0,
            10.0,
            &WindowConfig {
                step_sec: 0.0,
                ..valid_cfg.clone()
            }
        ),
        Err(SignalError::InvalidWindowSize(_))
    ));
    assert!(matches!(
        generate_windows(100.0, 10.0, &valid_cfg),
        Err(SignalError::InvalidWindowSize(_))
    ));
    assert!(matches!(
        generate_windows(f64::NAN, 10.0, &valid_cfg),
        Err(SignalError::NonFiniteInput)
    ));
}

#[test]
fn test_time_range_to_sample_range_boundaries() {
    let fs = 100.0;
    let len = 1000;

    // Integer aligned: [0.0, 1.0) -> sample indices 0 to 100
    let range = time_range_to_sample_range(0.0, 1.0, fs, 0.0, len)
        .unwrap()
        .unwrap();
    assert_eq!(range, (0, 100));

    // Fractional boundary: [0.005, 1.005) -> sample indices 1 to 101
    let range2 = time_range_to_sample_range(0.005, 1.005, fs, 0.0, len)
        .unwrap()
        .unwrap();
    assert_eq!(range2, (1, 101));

    // Outside bounds before data -> None
    assert!(
        time_range_to_sample_range(-10.0, -1.0, fs, 0.0, len)
            .unwrap()
            .is_none()
    );

    // Outside bounds after data -> None
    assert!(
        time_range_to_sample_range(20.0, 30.0, fs, 0.0, len)
            .unwrap()
            .is_none()
    );
}

// ============================================================================
// Group B — Cardiac Feature & HRV Hand-Calculation Tests
// ============================================================================

#[test]
fn test_cardiac_features_synthetic_r_peaks() {
    let fs = 100.0;
    let r_peaks: Vec<usize> = (0..10).map(|i| i * 100).collect();

    let win = generate_windows(
        0.0,
        10.0,
        &WindowConfig {
            window_duration_sec: 10.0,
            step_sec: 10.0,
            min_coverage: 0.8,
        },
    )
    .unwrap()[0]
        .clone();

    let cardiac = cardiac_features(&r_peaks, fs, 0.0, &win).unwrap();

    assert_eq!(cardiac.beat_count, 10);
    assert!((cardiac.mean_hr_bpm.unwrap() - 60.0).abs() < 1e-5);
    assert!((cardiac.median_hr_bpm.unwrap() - 60.0).abs() < 1e-5);
    assert!((cardiac.rr_mean_ms.unwrap() - 1000.0).abs() < 1e-5);
    assert!((cardiac.sdnn_ms.unwrap() - 0.0).abs() < 1e-5);
    assert!((cardiac.rmssd_ms.unwrap() - 0.0).abs() < 1e-5);
    assert!((cardiac.pnn50.unwrap() - 0.0).abs() < 1e-5);
}

#[test]
fn test_cardiac_sdnn_known_value() {
    let fs = 1000.0; // 1 ms resolution
    // R-peaks at times 0.0, 0.8, 1.8, 3.0 seconds -> RR intervals: 800 ms, 1000 ms, 1200 ms
    // Mean RR = 1000 ms
    // Deviations: -200, 0, 200 ms -> Squared: 40000, 0, 40000 -> Sum = 80000
    // Population variance = 80000 / 3 = 26666.6667
    // SDNN = sqrt(26666.6667) = 163.299316 ms
    let r_peaks = vec![0, 800, 1800, 3000];

    let win = generate_windows(
        0.0,
        5.0,
        &WindowConfig {
            window_duration_sec: 5.0,
            step_sec: 5.0,
            min_coverage: 0.8,
        },
    )
    .unwrap()[0]
        .clone();

    let cardiac = cardiac_features(&r_peaks, fs, 0.0, &win).unwrap();
    assert!((cardiac.rr_mean_ms.unwrap() - 1000.0).abs() < 1e-3);
    assert!((cardiac.sdnn_ms.unwrap() - 163.2993).abs() < 1e-3);
    assert_eq!(cardiac.rr_std_ms, cardiac.sdnn_ms);
}

#[test]
fn test_cardiac_boundary_crossing_rr_interval() {
    let fs = 10.0;
    // Peak at 5.5s (index 55), peak at 6.5s (index 65)
    // Window is [6.0, 12.0)
    let r_peaks = vec![55, 65, 75];

    let win = generate_windows(
        6.0,
        12.0,
        &WindowConfig {
            window_duration_sec: 6.0,
            step_sec: 6.0,
            min_coverage: 0.8,
        },
    )
    .unwrap()[0]
        .clone();

    let cardiac = cardiac_features(&r_peaks, fs, 0.0, &win).unwrap();
    // Terminating R-peak at 6.5s falls in [6.0, 12.0). Preceding peak is 5.5s.
    // RR interval (5.5, 6.5) = 1.0s = 1000 ms is included!
    assert!(cardiac.rr_mean_ms.is_some());
    assert!((cardiac.rr_mean_ms.unwrap() - 1000.0).abs() < 1e-3);
}

// ============================================================================
// Group C — EDA Feature & Safety Tests
// ============================================================================

#[test]
fn test_eda_features_synthetic_signals_and_scrs() {
    let fs = 100.0;
    let n_samples = 1000; // 10s
    let tonic = Array1::from_elem(n_samples, 3.5);
    let phasic = Array1::from_elem(n_samples, 0.2);

    let scrs = vec![
        ScrEvent {
            onset_index: 100,
            peak_index: 200,
            amplitude: 1.5,
            rise_time_sec: 1.0,
        },
        ScrEvent {
            onset_index: 500,
            peak_index: 600,
            amplitude: 2.5,
            rise_time_sec: 1.0,
        },
    ];

    let win = generate_windows(
        0.0,
        10.0,
        &WindowConfig {
            window_duration_sec: 10.0,
            step_sec: 10.0,
            min_coverage: 0.8,
        },
    )
    .unwrap()[0]
        .clone();

    let eda = eda_features(&tonic, &phasic, &scrs, fs, 0.0, &win).unwrap();

    assert_eq!(eda.scr_count, 2);
    assert!((eda.scr_rate_per_min.unwrap() - 12.0).abs() < 1e-5);
    assert!((eda.mean_tonic_us.unwrap() - 3.5).abs() < 1e-5);
    assert!((eda.mean_phasic_us.unwrap() - 0.2).abs() < 1e-5);
    assert!((eda.mean_scr_amplitude_us.unwrap() - 2.0).abs() < 1e-5);
}

#[test]
fn test_eda_mismatched_lengths_returns_error() {
    let fs = 100.0;
    let tonic = Array1::from_elem(1000, 3.5);
    let phasic = Array1::from_elem(500, 0.2); // Mismatched length

    let win = generate_windows(
        0.0,
        10.0,
        &WindowConfig {
            window_duration_sec: 10.0,
            step_sec: 10.0,
            min_coverage: 0.8,
        },
    )
    .unwrap()[0]
        .clone();

    assert!(matches!(
        eda_features(&tonic, &phasic, &[], fs, 0.0, &win),
        Err(SignalError::DimensionMismatch)
    ));
}

// ============================================================================
// Group D — Respiration Feature Tests
// ============================================================================

#[test]
fn test_respiration_features_synthetic_cycles() {
    let fs = 100.0;
    let cycles = vec![
        RespirationCycle {
            inspiration_index: 100,
            expiration_index: 300,
            next_inspiration_index: 500,
            duration_sec: 4.0,
            respiratory_rate_bpm: 15.0,
            amplitude: 1.0,
        },
        RespirationCycle {
            inspiration_index: 500,
            expiration_index: 700,
            next_inspiration_index: 900,
            duration_sec: 4.0,
            respiratory_rate_bpm: 15.0,
            amplitude: 1.0,
        },
    ];

    let win = generate_windows(
        0.0,
        10.0,
        &WindowConfig {
            window_duration_sec: 10.0,
            step_sec: 10.0,
            min_coverage: 0.8,
        },
    )
    .unwrap()[0]
        .clone();

    let resp = respiration_features(&cycles, fs, 0.0, &win).unwrap();

    assert_eq!(resp.cycle_count, 2);
    assert!((resp.mean_rate_bpm.unwrap() - 15.0).abs() < 1e-5);
    assert!((resp.mean_cycle_duration_sec.unwrap() - 4.0).abs() < 1e-5);
}

// ============================================================================
// Group E — Coupling Feature Tests
// ============================================================================

#[test]
fn test_coupling_features_multimodal() {
    let fs = 100.0;
    let r_peaks = vec![100, 200, 300, 400, 500, 600, 700, 800, 900];
    let ppg_peaks = vec![120, 220, 320, 420, 520, 620, 720, 820, 920];
    let cycles = vec![RespirationCycle {
        inspiration_index: 100,
        expiration_index: 500,
        next_inspiration_index: 900,
        duration_sec: 8.0,
        respiratory_rate_bpm: 7.5,
        amplitude: 1.0,
    }];

    let win = generate_windows(
        0.0,
        10.0,
        &WindowConfig {
            window_duration_sec: 10.0,
            step_sec: 10.0,
            min_coverage: 0.8,
        },
    )
    .unwrap()[0]
        .clone();

    let coupling = coupling_features(
        Some(&r_peaks),
        fs,
        0.0,
        Some(&ppg_peaks),
        fs,
        0.0,
        Some(&cycles),
        fs,
        0.0,
        None,
        fs,
        0.0,
        &win,
    )
    .unwrap();

    assert!(coupling.mean_pulse_delay_sec.is_some());
    assert!((coupling.mean_pulse_delay_sec.unwrap() - 0.20).abs() < 1e-5);
}

// ============================================================================
// Group F — Coverage & Quality Semantics Tests
// ============================================================================

#[test]
fn test_coverage_semantics_usable_window_duration() {
    let input = MultimodalInput::new()
        .with_ecg(
            (0..=60).map(|i| i * 100).collect(), // 0 to 60s
            100.0,
            0.0,
        )
        .unwrap();

    let cfg = FeatureConfig {
        window: WindowConfig {
            window_duration_sec: 60.0,
            step_sec: 60.0,
            min_coverage: 0.80,
        },
        ..FeatureConfig::default()
    };

    let fvs = extract_features(&input, &cfg).unwrap();
    assert!(!fvs.is_empty());
    let quality = &fvs[0].quality;

    // Full 60s window fully populated -> coverage is ~1.0 (100%), NOT 1/recording_len
    assert!((quality.coverage - 1.0).abs() < 1e-3);
    assert!((quality.modality_coverage.overall - 1.0).abs() < 1e-3);
    assert_eq!(quality.modality_coverage.ecg, Some(1.0));
    assert_eq!(quality.modality_coverage.eda, None); // Absent modality represented as None
}

// ============================================================================
// Group G — Naive Reference Equivalence Oracle & Large Event Scaling
// ============================================================================

#[test]
fn test_naive_reference_equivalence_oracle() {
    let fs = 100.0;
    let r_peaks: Vec<usize> = (0..100).map(|i| i * 100).collect();
    let input = MultimodalInput::new().with_ecg(r_peaks, fs, 0.0).unwrap();

    let cfg = FeatureConfig {
        window: WindowConfig {
            window_duration_sec: 10.0,
            step_sec: 5.0,
            min_coverage: 0.8,
        },
        ..FeatureConfig::default()
    };

    let fvs_prod = extract_features(&input, &cfg).unwrap();
    let fvs_naive = extract_features_naive(&input, &cfg).unwrap();

    assert_eq!(fvs_prod.len(), fvs_naive.len());
    for (p, n) in fvs_prod.iter().zip(fvs_naive.iter()) {
        assert_eq!(p.cardiac, n.cardiac);
        assert_eq!(p.quality, n.quality);
    }
}

#[test]
fn test_large_event_collection_scaling() {
    let fs = 100.0;
    let n_events = 100_000;
    let r_peaks: Vec<usize> = (0..n_events).map(|i| i * 100).collect();
    let input = MultimodalInput::new().with_ecg(r_peaks, fs, 0.0).unwrap();

    let cfg = FeatureConfig {
        window: WindowConfig {
            window_duration_sec: 60.0,
            step_sec: 30.0,
            min_coverage: 0.8,
        },
        ..FeatureConfig::default()
    };

    let fvs = extract_features(&input, &cfg).unwrap();
    assert!(!fvs.is_empty());
}

// ============================================================================
// Group H — Sampling Rate Invariance Tests
// ============================================================================

#[test]
fn test_multi_sampling_rate_feature_invariance() {
    let sampling_rates = [32.0, 64.0, 100.0, 128.0, 250.0, 500.0, 1000.0];

    for &fs in &sampling_rates {
        let r_peaks: Vec<usize> = (0..10)
            .map(|i| (i as f64 * 1.0 * fs).round() as usize)
            .collect();
        let win = generate_windows(
            0.0,
            10.0,
            &WindowConfig {
                window_duration_sec: 10.0,
                step_sec: 10.0,
                min_coverage: 0.8,
            },
        )
        .unwrap()[0]
            .clone();

        let cardiac = cardiac_features(&r_peaks, fs, 0.0, &win).unwrap();
        assert!((cardiac.mean_hr_bpm.unwrap() - 60.0).abs() < 1e-3);
    }
}

// ============================================================================
// Group I — Numerical Robustness & Fallible Inputs
// ============================================================================

#[test]
fn test_numerical_robustness_and_fallible_inputs() {
    let win = generate_windows(
        0.0,
        10.0,
        &WindowConfig {
            window_duration_sec: 10.0,
            step_sec: 10.0,
            min_coverage: 0.8,
        },
    )
    .unwrap()[0]
        .clone();

    assert!(matches!(
        cardiac_features(&[100], -100.0, 0.0, &win),
        Err(SignalError::InvalidSamplingRate(_))
    ));
    assert!(matches!(
        cardiac_features(&[100], 100.0, f64::NAN, &win),
        Err(SignalError::NonFiniteInput)
    ));
}

// ============================================================================
// Group J — End-to-End Integration Pipeline Test
// ============================================================================

#[test]
fn test_features_end_to_end_multimodal_pipeline() {
    let fs = 100.0;
    let n_samples = 3000; // 30 seconds

    let mut ecg_signal = Array1::<f64>::zeros(n_samples);
    for i in (50..n_samples).step_by(100) {
        ecg_signal[i] = 4.0;
    }
    let r_peaks_mask = ecg_findpeaks(&ecg_signal, fs).unwrap();
    let r_peaks = mask_to_indices(&r_peaks_mask);

    let mut rsp_signal = Array1::<f64>::zeros(n_samples);
    for i in 0..n_samples {
        let t = i as f64 / fs;
        rsp_signal[i] = (2.0 * PI * 0.25 * t).sin();
    }
    let cleaned_rsp = rsp_clean(&rsp_signal, fs).unwrap();
    let rsp_cycle_list = rsp_cycles(&cleaned_rsp, fs).unwrap();

    let mut ppg_signal = Array1::<f64>::zeros(n_samples);
    for &r_idx in &r_peaks {
        let p_idx = r_idx + 20;
        if p_idx < n_samples {
            ppg_signal[p_idx] = 2.0;
        }
    }
    let ppg_peaks_mask = ppg_findpeaks(&ppg_signal, fs).unwrap();
    let ppg_peaks_list = mask_to_indices(&ppg_peaks_mask);

    let mut eda_signal = Array1::<f64>::zeros(n_samples);
    for i in 0..n_samples {
        eda_signal[i] = 2.0 + 0.001 * i as f64;
    }
    eda_signal[500] += 1.0;
    let decomp = eda_decompose(&eda_signal, fs, &EdaDecompositionConfig::default()).unwrap();
    let scr_events =
        eda_findpeaks_events(&decomp.phasic, fs, &EdaPeakDetectionConfig::default()).unwrap();

    let input = MultimodalInput::new()
        .with_ecg(r_peaks, fs, 0.0)
        .unwrap()
        .with_ppg(ppg_peaks_list, fs, 0.0)
        .unwrap()
        .with_eda(decomp.tonic, decomp.phasic, scr_events, fs, 0.0)
        .unwrap()
        .with_rsp(rsp_cycle_list, fs, 0.0)
        .unwrap();

    let cfg = FeatureConfig {
        window: WindowConfig {
            window_duration_sec: 10.0,
            step_sec: 10.0,
            min_coverage: 0.8,
        },
        ..FeatureConfig::default()
    };

    let fvs = extract_features(&input, &cfg).unwrap();
    assert!(!fvs.is_empty());

    for fv in &fvs {
        assert!(fv.cardiac.mean_hr_bpm.is_some());
        assert!(fv.eda.mean_tonic_us.is_some());
        assert!(fv.respiration.mean_rate_bpm.is_some());
        assert!(fv.quality.usable_feature_count > 0);
    }
}

// ============================================================================
// Group K — Ordering Invariant & Oracle Parity Hardening Tests
// ============================================================================

#[test]
fn test_unsorted_events_returns_error() {
    let cfg = FeatureConfig::default();

    // Unsorted ECG peaks
    let input_ecg = MultimodalInput::new()
        .with_ecg(vec![500, 100, 300], 100.0, 0.0)
        .unwrap();
    assert!(matches!(
        extract_features(&input_ecg, &cfg),
        Err(SignalError::UnsortedEvents)
    ));

    // Unsorted PPG peaks
    let input_ppg = MultimodalInput::new()
        .with_ppg(vec![500, 100, 300], 100.0, 0.0)
        .unwrap();
    assert!(matches!(
        extract_features(&input_ppg, &cfg),
        Err(SignalError::UnsortedEvents)
    ));

    // Unsorted RSP cycles
    let cycles = vec![
        RespirationCycle {
            inspiration_index: 200,
            expiration_index: 250,
            next_inspiration_index: 300,
            duration_sec: 1.0,
            respiratory_rate_bpm: 60.0,
            amplitude: 1.0,
        },
        RespirationCycle {
            inspiration_index: 50,
            expiration_index: 100,
            next_inspiration_index: 150,
            duration_sec: 1.0,
            respiratory_rate_bpm: 60.0,
            amplitude: 1.0,
        },
    ];
    let input_rsp = MultimodalInput::new().with_rsp(cycles, 100.0, 0.0).unwrap();
    assert!(matches!(
        extract_features(&input_rsp, &cfg),
        Err(SignalError::UnsortedEvents)
    ));
}

#[test]
fn test_oracle_equivalence_small_hop_dense_events() {
    let fs = 100.0;
    // 60 seconds of signal with high beat density (approx 2 beats/sec)
    let r_peaks: Vec<usize> = (0..120).map(|i| (i as f64 * 0.5 * fs) as usize).collect();
    let ppg_peaks: Vec<usize> = r_peaks.iter().map(|&r| r + 20).collect();

    let input = MultimodalInput::new()
        .with_ecg(r_peaks, fs, 0.0)
        .unwrap()
        .with_ppg(ppg_peaks, fs, 0.0)
        .unwrap();

    let cfg = FeatureConfig {
        window: WindowConfig {
            window_duration_sec: 10.0,
            step_sec: 1.0, // Highly overlapping 1s hop
            min_coverage: 0.8,
        },
        ..FeatureConfig::default()
    };

    let fvs_opt = extract_features(&input, &cfg).unwrap();
    let fvs_naive = extract_features_naive(&input, &cfg).unwrap();

    assert_eq!(fvs_opt.len(), fvs_naive.len());
    assert!(!fvs_opt.is_empty());

    for (opt, naive) in fvs_opt.iter().zip(fvs_naive.iter()) {
        assert!((opt.window.start_time_sec - naive.window.start_time_sec).abs() < 1e-6);
        assert_eq!(opt.cardiac.beat_count, naive.cardiac.beat_count);

        if let (Some(a), Some(b)) = (opt.cardiac.mean_hr_bpm, naive.cardiac.mean_hr_bpm) {
            assert!((a - b).abs() < 1e-6);
        } else {
            assert_eq!(
                opt.cardiac.mean_hr_bpm.is_some(),
                naive.cardiac.mean_hr_bpm.is_some()
            );
        }

        if let (Some(a), Some(b)) = (opt.cardiac.sdnn_ms, naive.cardiac.sdnn_ms) {
            assert!((a - b).abs() < 1e-6);
        } else {
            assert_eq!(
                opt.cardiac.sdnn_ms.is_some(),
                naive.cardiac.sdnn_ms.is_some()
            );
        }

        if let (Some(a), Some(b)) = (
            opt.coupling.mean_pulse_delay_sec,
            naive.coupling.mean_pulse_delay_sec,
        ) {
            assert!((a - b).abs() < 1e-6);
        } else {
            assert_eq!(
                opt.coupling.mean_pulse_delay_sec.is_some(),
                naive.coupling.mean_pulse_delay_sec.is_some()
            );
        }
    }
}
