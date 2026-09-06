use lamina::eda::{
    EdaCleaningConfig, EdaDecompositionConfig, EdaPeakDetectionConfig, ScrEvent, eda_clean,
    eda_clean_config, eda_decompose, eda_findpeaks, eda_findpeaks_config, eda_findpeaks_events,
    eda_findpeaks_mask, eda_phasic,
};
use lamina::error::SignalError;
use ndarray::Array1;
use serde::Deserialize;
use std::f64::consts::PI;
use std::fs::File;
use std::io::BufReader;

#[allow(dead_code)]
#[derive(Deserialize)]
struct GoldenEdaCase {
    name: String,
    sampling_rate: f64,
    scr_number: usize,
    raw_signal: Vec<f64>,
    expected_scr_peaks: Vec<usize>,
}

#[test]
fn test_eda_clean_basic() {
    let signal = Array1::<f64>::ones(100);
    let cleaned = eda_clean(&signal, 100.0).expect("EDA clean failed");
    assert_eq!(cleaned.len(), signal.len());
}

// Group A — Mathematical / Structural Invariants
#[test]
fn test_eda_reconstruction_invariant() {
    let fs = 100.0;
    let n = 1000;
    let t = Array1::linspace(0.0, 10.0, n);
    let raw = t.mapv(|tv| 2.0 + 0.1 * tv + 0.5 * (2.0 * PI * 1.0 * tv).sin());

    let cleaned = eda_clean(&raw, fs).expect("Clean failed");
    let components =
        eda_decompose(&cleaned, fs, &EdaDecompositionConfig::default()).expect("Decompose failed");

    assert_eq!(components.tonic.len(), n);
    assert_eq!(components.phasic.len(), n);

    // Verify exact reconstruction: tonic + phasic = cleaned
    for i in 0..n {
        let reconstructed = components.tonic[i] + components.phasic[i];
        let diff = (reconstructed - cleaned[i]).abs();
        assert!(
            diff < 1e-12,
            "Reconstruction error at sample {} exceeds tolerance: {}",
            i,
            diff
        );
    }
}

// Group B — Synthetic Known-Answer Tests
#[test]
fn test_eda_synthetic_known_answer_scrs() {
    let fs = 100.0;
    let duration = 10.0;
    let n = (fs * duration) as usize;

    let mut phasic_sig = Array1::<f64>::zeros(n);
    let mut known_events = Vec::new();

    // Generate 3 known SCR events with known onsets, peaks, amplitudes, and rise times
    let event_specs = vec![
        (2.0, 0.5, 0.2), // sec 2.0: onset=200, peak=250 (rise=0.5s), amp=0.2
        (5.0, 1.0, 0.4), // sec 5.0: onset=500, peak=600 (rise=1.0s), amp=0.4
        (8.0, 0.8, 0.3), // sec 8.0: onset=800, peak=880 (rise=0.8s), amp=0.3
    ];

    for &(start_sec, rise_sec, amp) in &event_specs {
        let onset_idx = (start_sec * fs) as usize;
        let rise_samples = (rise_sec * fs) as usize;
        let peak_idx = onset_idx + rise_samples;
        known_events.push((onset_idx, peak_idx, amp, rise_sec));

        for i in 0..rise_samples {
            let idx = onset_idx + i;
            let progress = i as f64 / rise_samples as f64;
            phasic_sig[idx] = amp * (PI * progress / 2.0).sin();
        }
        // Decay phase
        let decay_samples = (1.5 * fs) as usize;
        for i in 0..decay_samples {
            let idx = peak_idx + i;
            if idx < n {
                let progress = i as f64 / decay_samples as f64;
                phasic_sig[idx] = amp * (-2.0 * progress).exp();
            }
        }
    }

    let config = EdaPeakDetectionConfig::default()
        .with_min_amplitude(0.05)
        .with_min_prominence(0.01);
    let events = eda_findpeaks_events(&phasic_sig, fs, &config).expect("Finding SCR events failed");

    assert_eq!(events.len(), 3, "Expected exactly 3 SCR events detected");

    for (i, &(exp_onset, exp_peak, exp_amp, exp_rise)) in known_events.iter().enumerate() {
        let ev = &events[i];
        let onset_diff = (ev.onset_index as i64 - exp_onset as i64).abs();
        let peak_diff = (ev.peak_index as i64 - exp_peak as i64).abs();
        let amp_diff = (ev.amplitude - exp_amp).abs();
        let rise_diff = (ev.rise_time_sec - exp_rise).abs();

        assert!(
            onset_diff <= 5,
            "Onset index diff at event {}: {}",
            i,
            onset_diff
        );
        assert!(
            peak_diff <= 5,
            "Peak index diff at event {}: {}",
            i,
            peak_diff
        );
        assert!(
            amp_diff < 0.05,
            "Amplitude diff at event {}: {}",
            i,
            amp_diff
        );
        assert!(
            rise_diff < 0.1,
            "Rise time diff at event {}: {}",
            i,
            rise_diff
        );
    }
}

// Group C — Reference Implementation Comparison (NeuroKit2)
#[test]
fn test_eda_golden_parity_neurokit() {
    let file = match File::open("tests/golden_eda.json") {
        Ok(f) => f,
        Err(_) => {
            println!("Skipping golden EDA reference test: golden_eda.json not found");
            return;
        }
    };
    let reader = BufReader::new(file);
    let cases: Vec<GoldenEdaCase> =
        serde_json::from_reader(reader).expect("Failed to parse golden_eda.json");

    assert!(!cases.is_empty(), "Golden EDA dataset is empty");

    let mut total_tp = 0;
    let mut total_fp = 0;
    let mut total_fn = 0;

    for case in &cases {
        let signal = Array1::from_vec(case.raw_signal.clone());
        let fs = case.sampling_rate;
        let expected = &case.expected_scr_peaks;

        let cleaned = eda_clean(&signal, fs).expect("eda_clean failed");
        let phasic = eda_phasic(&cleaned, fs).expect("eda_phasic failed");
        let detected = eda_findpeaks_config(&phasic, fs, &EdaPeakDetectionConfig::default())
            .expect("eda_findpeaks_config failed");

        let tol_samples = (0.250 * fs).round() as i64; // 250ms tolerance

        let mut matched_exp = vec![false; expected.len()];
        let mut matched_det = vec![false; detected.len()];

        for (d_i, &det) in detected.iter().enumerate() {
            for (e_i, &exp) in expected.iter().enumerate() {
                if !matched_exp[e_i] && (det as i64 - exp as i64).abs() <= tol_samples {
                    matched_exp[e_i] = true;
                    matched_det[d_i] = true;
                    break;
                }
            }
        }

        let tp = matched_exp.iter().filter(|&&m| m).count();
        let fn_count = expected.len() - tp;
        let fp = detected.len() - tp;

        total_tp += tp;
        total_fp += fp;
        total_fn += fn_count;

        println!(
            "EDA Case '{}' (Fs={}Hz, SCRs={}): TP={}, FP={}, FN={}",
            case.name, fs, case.scr_number, tp, fp, fn_count
        );
    }

    let precision = if total_tp + total_fp > 0 {
        total_tp as f64 / (total_tp + total_fp) as f64
    } else {
        1.0
    };
    let recall = if total_tp + total_fn > 0 {
        total_tp as f64 / (total_tp + total_fn) as f64
    } else {
        1.0
    };
    let f1 = if precision + recall > 0.0 {
        2.0 * precision * recall / (precision + recall)
    } else {
        0.0
    };

    println!(
        "Overall Golden EDA Performance: Precision={:.4}, Recall={:.4}, F1={:.4}",
        precision, recall, f1
    );

    assert!(
        f1 >= 0.75,
        "EDA F1 score ({:.4}) must be >= 0.75 against NeuroKit2 reference",
        f1
    );
}

// Group D — Robustness & Multi-Fs Suite
#[test]
fn test_eda_multi_sampling_rate_suite() {
    let sampling_rates: Vec<f64> = vec![32.0, 64.0, 100.0, 128.0, 250.0, 500.0];

    for &fs in &sampling_rates {
        let duration = 10.0;
        let n = (fs * duration) as usize;
        let t = Array1::linspace(0.0, duration, n);

        let mut raw = t.mapv(|tv| 3.0 + 0.05 * tv);
        let onset_idx = (3.0 * fs) as usize;
        let peak_idx = (4.0 * fs) as usize;
        for i in onset_idx..peak_idx {
            raw[i] += 0.5 * ((i - onset_idx) as f64 / (peak_idx - onset_idx) as f64);
        }

        let cleaned = eda_clean(&raw, fs).expect("Clean failed");
        let components = eda_decompose(&cleaned, fs, &EdaDecompositionConfig::default())
            .expect("Decompose failed");

        let events =
            eda_findpeaks_events(&components.phasic, fs, &EdaPeakDetectionConfig::default())
                .expect("SCR detection failed");

        assert!(
            !events.is_empty(),
            "Expected SCR event detected at Fs={}Hz",
            fs
        );
    }
}

#[test]
fn test_eda_edge_case_robustness() {
    let fs = 100.0;
    let n = 1000;

    // Constant signal noise floor protection
    let const_sig = Array1::<f64>::ones(n) * 4.2;
    let cleaned = eda_clean(&const_sig, fs).expect("Clean constant signal failed");
    let components = eda_decompose(&cleaned, fs, &EdaDecompositionConfig::default())
        .expect("Decompose constant signal failed");
    let events = eda_findpeaks_events(&components.phasic, fs, &EdaPeakDetectionConfig::default())
        .expect("Findpeaks constant signal failed");
    assert_eq!(events.len(), 0, "Constant signal must yield 0 SCR events");

    // Extreme amplitude scaling
    let mut high_amp = Array1::<f64>::ones(n);
    high_amp[500] = 500.0;
    assert!(eda_clean(&high_amp, fs).is_ok());
}

// Group E — API & Input Validation
#[test]
fn test_eda_input_validation() {
    let empty_sig = Array1::<f64>::zeros(0);
    assert!(matches!(
        eda_clean(&empty_sig, 100.0),
        Err(SignalError::EmptySignal)
    ));
    assert!(matches!(
        eda_phasic(&empty_sig, 100.0),
        Err(SignalError::EmptySignal)
    ));

    let nan_sig = Array1::from_vec(vec![1.0, f64::NAN, 2.0]);
    assert!(matches!(
        eda_clean(&nan_sig, 100.0),
        Err(SignalError::NonFiniteInput)
    ));
    assert!(matches!(
        eda_phasic(&nan_sig, 100.0),
        Err(SignalError::NonFiniteInput)
    ));

    let inf_sig = Array1::from_vec(vec![1.0, f64::INFINITY, 2.0]);
    assert!(matches!(
        eda_clean(&inf_sig, 100.0),
        Err(SignalError::NonFiniteInput)
    ));

    let valid_sig = Array1::<f64>::ones(500);
    assert!(matches!(
        eda_clean(&valid_sig, 0.0),
        Err(SignalError::InvalidSamplingRate(_))
    ));
    assert!(matches!(
        eda_clean(&valid_sig, -100.0),
        Err(SignalError::InvalidSamplingRate(_))
    ));

    // Insufficient samples
    let short_sig = Array1::<f64>::ones(5);
    assert!(matches!(
        eda_clean(&short_sig, 100.0),
        Err(SignalError::InsufficientSamples { .. })
    ));

    // Invalid Cutoff (exceeding Nyquist)
    let inv_cfg = EdaDecompositionConfig::new().with_tonic_cutoff_hz(60.0);
    assert!(matches!(
        eda_decompose(&valid_sig, 100.0, &inv_cfg),
        Err(SignalError::InvalidCutoffFrequency(_))
    ));
}

#[test]
fn test_eda_convenience_apis() {
    let fs = 100.0;
    let n = 500;
    let mut phasic = Array1::<f64>::zeros(n);
    for i in 189..=199 {
        phasic[i] = 0.05 * (i - 189) as f64;
    }

    let config = EdaPeakDetectionConfig::default();
    let events: Vec<ScrEvent> =
        eda_findpeaks_events(&phasic, fs, &config).expect("eda_findpeaks_events failed");
    assert!(!events.is_empty());
    assert_eq!(events[0].peak_index, 199);

    let mask = eda_findpeaks_mask(&phasic, fs, &config).expect("eda_findpeaks_mask failed");
    assert_eq!(mask.len(), n);
    assert!(mask[199]);

    let compat_mask = eda_findpeaks(&phasic, fs).expect("eda_findpeaks failed");
    assert_eq!(compat_mask.len(), n);
    assert!(compat_mask[199]);
}

#[test]
fn test_eda_4hz_wearable_passband_and_nyquist_handling() {
    // Empatica E4 wearable EDA (fs = 4.0 Hz, Nyquist = 2.0 Hz)
    let fs = 4.0;
    let duration = 60.0;
    let n = (fs * duration) as usize;
    let raw_eda = Array1::from_elem(n, 2.5);

    // Default 5.0 Hz cutoff >= 2.0 Hz Nyquist -> pass-through enabled by default
    let cleaned_default = eda_clean(&raw_eda, fs).expect("eda_clean at 4 Hz failed");
    assert_eq!(cleaned_default, raw_eda);

    // Explicit 1.5 Hz lowpass cutoff < 2.0 Hz Nyquist -> applies Butterworth lowpass filter
    let cfg_1_5 = EdaCleaningConfig::new().with_lowpass_cutoff_hz(1.5);
    let cleaned_1_5 =
        eda_clean_config(&raw_eda, fs, &cfg_1_5).expect("eda_clean_config at 1.5 Hz failed");
    assert_eq!(cleaned_1_5.len(), n);

    // Explicit pass_through_if_nyquist_violated = false -> returns InvalidCutoffFrequency error
    let cfg_strict = EdaCleaningConfig::new()
        .with_lowpass_cutoff_hz(5.0)
        .with_pass_through_if_nyquist_violated(false);
    assert!(eda_clean_config(&raw_eda, fs, &cfg_strict).is_err());
}

#[test]
fn test_eda_multi_sampling_rates() {
    let rates = [4.0, 16.0, 100.0, 700.0];
    for &fs in &rates {
        let duration = 30.0;
        let n = (fs * duration) as usize;
        let t = Array1::linspace(0.0, duration, n);
        let raw = t.mapv(|tv| 1.0 + 0.1 * (2.0 * std::f64::consts::PI * 0.05 * tv).sin());
        let cleaned = eda_clean(&raw, fs).expect("eda_clean failed");
        assert_eq!(cleaned.len(), n);
        let decomp = eda_decompose(&cleaned, fs, &EdaDecompositionConfig::default())
            .expect("eda_decompose failed");
        assert_eq!(decomp.tonic.len(), n);
        assert_eq!(decomp.phasic.len(), n);
    }
}
