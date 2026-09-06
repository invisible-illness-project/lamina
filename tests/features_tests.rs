use lamina::ecg::ecg_findpeaks;
use lamina::eda::{
    EdaDecompositionConfig, EdaPeakDetectionConfig, ScrEvent, eda_decompose, eda_findpeaks_events,
};
use lamina::error::SignalError;
use lamina::features::{
    FeatureConfig, MultimodalInput, WindowConfig, cardiac_features, coupling_features,
    eda_features, extract_features, generate_windows, respiration_features,
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
// Group A — Windowing Tests
// ============================================================================

#[test]
fn test_generate_windows_fixed_and_overlapping() {
    let cfg = WindowConfig {
        window_duration_sec: 60.0,
        step_sec: 30.0,
        min_coverage: 0.80,
    };

    // 0 to 120 seconds -> 3 windows: [0, 60), [30, 90), [60, 120)
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

// ============================================================================
// Group B — Cardiac Feature Tests
// ============================================================================

#[test]
fn test_cardiac_features_synthetic_r_peaks() {
    let fs = 100.0;
    // R-peaks every 1.0s (60 BPM, 1000 ms RR) over 10 seconds: indices 0, 100, 200, ..., 900
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

// ============================================================================
// Group C — EDA Feature Tests
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
    assert!((eda.scr_rate_per_min.unwrap() - 12.0).abs() < 1e-5); // 2 events / 10s = 12 events/min
    assert!((eda.mean_tonic_us.unwrap() - 3.5).abs() < 1e-5);
    assert!((eda.mean_phasic_us.unwrap() - 0.2).abs() < 1e-5);
    assert!((eda.mean_scr_amplitude_us.unwrap() - 2.0).abs() < 1e-5);
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
// Group F — Missing Modality Tests
// ============================================================================

#[test]
fn test_missing_modalities_returns_none_statistics() {
    let input = MultimodalInput {
        ecg_r_peaks: Some(vec![0, 100, 200, 300, 400, 500, 600, 700, 800, 900, 1000]),
        ecg_sampling_rate: 100.0,
        ecg_offset_sec: 0.0,
        eda_tonic: None,
        eda_phasic: None,
        eda_scr_events: None,
        eda_sampling_rate: 100.0,
        eda_offset_sec: 0.0,
        rsp_cycles: None,
        rsp_sampling_rate: 100.0,
        rsp_offset_sec: 0.0,
        ppg_peaks: None,
        ppg_sampling_rate: 100.0,
        ppg_offset_sec: 0.0,
    };

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

    let fv = &fvs[0];
    assert!(fv.cardiac.mean_hr_bpm.is_some());
    assert!(fv.eda.mean_tonic_us.is_none());
    assert!(fv.respiration.mean_rate_bpm.is_none());
    assert!(fv.coupling.rsa_amplitude_bpm.is_none());
}

// ============================================================================
// Group G — Sampling Rate Invariance Tests
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
// Group H — Numerical Robustness & Fallible Inputs
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
// Group I — End-to-End Integration Pipeline Test
// ============================================================================

#[test]
fn test_features_end_to_end_multimodal_pipeline() {
    let fs = 100.0;
    let n_samples = 3000; // 30 seconds

    // Synthetic ECG
    let mut ecg_signal = Array1::<f64>::zeros(n_samples);
    for i in (50..n_samples).step_by(100) {
        ecg_signal[i] = 4.0;
    }
    let r_peaks_mask = ecg_findpeaks(&ecg_signal, fs).unwrap();
    let r_peaks = mask_to_indices(&r_peaks_mask);

    // Synthetic RSP
    let mut rsp_signal = Array1::<f64>::zeros(n_samples);
    for i in 0..n_samples {
        let t = i as f64 / fs;
        rsp_signal[i] = (2.0 * PI * 0.25 * t).sin();
    }
    let cleaned_rsp = rsp_clean(&rsp_signal, fs).unwrap();
    let rsp_cycle_list = rsp_cycles(&cleaned_rsp, fs).unwrap();

    // Synthetic PPG
    let mut ppg_signal = Array1::<f64>::zeros(n_samples);
    for &r_idx in &r_peaks {
        let p_idx = r_idx + 20;
        if p_idx < n_samples {
            ppg_signal[p_idx] = 2.0;
        }
    }
    let ppg_peaks_mask = ppg_findpeaks(&ppg_signal, fs).unwrap();
    let ppg_peaks_list = mask_to_indices(&ppg_peaks_mask);

    // Synthetic EDA
    let mut eda_signal = Array1::<f64>::zeros(n_samples);
    for i in 0..n_samples {
        eda_signal[i] = 2.0 + 0.001 * i as f64;
    }
    eda_signal[500] += 1.0;
    let decomp = eda_decompose(&eda_signal, fs, &EdaDecompositionConfig::default()).unwrap();
    let scr_events =
        eda_findpeaks_events(&decomp.phasic, fs, &EdaPeakDetectionConfig::default()).unwrap();

    let input = MultimodalInput {
        ecg_r_peaks: Some(r_peaks),
        ecg_sampling_rate: fs,
        ecg_offset_sec: 0.0,
        eda_tonic: Some(decomp.tonic),
        eda_phasic: Some(decomp.phasic),
        eda_scr_events: Some(scr_events),
        eda_sampling_rate: fs,
        eda_offset_sec: 0.0,
        rsp_cycles: Some(rsp_cycle_list),
        rsp_sampling_rate: fs,
        rsp_offset_sec: 0.0,
        ppg_peaks: Some(ppg_peaks_list),
        ppg_sampling_rate: fs,
        ppg_offset_sec: 0.0,
    };

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
