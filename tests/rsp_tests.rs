use lamina::error::SignalError;
use lamina::rsp::{
    RspCleaningConfig, RspProcessingConfig, rsp_clean, rsp_clean_config, rsp_cycles,
    rsp_cycles_config, rsp_findpeaks, rsp_findpeaks_config, rsp_findpeaks_mask, rsp_rate,
};
use ndarray::Array1;
use serde::Deserialize;
use std::f64::consts::PI;
use std::fs::File;
use std::io::BufReader;

#[allow(dead_code)]
#[derive(Deserialize)]
struct GoldenRspCase {
    name: String,
    sampling_rate: f64,
    respiratory_rate: f64,
    raw_signal: Vec<f64>,
    expected_peaks: Vec<usize>,
}

#[test]
fn test_rsp_clean_basic() {
    let signal = Array1::<f64>::ones(500);
    let cleaned = rsp_clean(&signal, 100.0).expect("RSP clean failed");
    assert_eq!(cleaned.len(), signal.len());
}

// Group A — Mathematical / Structural Invariants
#[test]
fn test_rsp_cycle_invariants() {
    let fs = 100.0;
    let duration = 20.0;
    let n = (fs * duration) as usize;
    let t = Array1::linspace(0.0, duration, n);
    // 12 breaths/min = 0.2 Hz sinusoidal signal
    let raw = t.mapv(|tv| (2.0 * PI * 0.2 * tv).sin());

    let cycles = rsp_cycles(&raw, fs).expect("rsp_cycles failed");
    assert!(!cycles.is_empty());

    for cycle in &cycles {
        // Invariant 1: Ordering inspiration < expiration < next_inspiration
        assert!(
            cycle.inspiration_index < cycle.expiration_index,
            "Inspiration index ({}) must be < Expiration index ({})",
            cycle.inspiration_index,
            cycle.expiration_index
        );
        assert!(
            cycle.expiration_index < cycle.next_inspiration_index,
            "Expiration index ({}) must be < Next inspiration index ({})",
            cycle.expiration_index,
            cycle.next_inspiration_index
        );

        // Invariant 2: Positive duration
        assert!(cycle.duration_sec > 0.0);

        // Invariant 3: Rate consistency (BPM = 60 / duration_sec)
        let expected_bpm = 60.0 / cycle.duration_sec;
        let diff = (cycle.respiratory_rate_bpm - expected_bpm).abs();
        assert!(
            diff < 1e-6,
            "Rate consistency diff exceeds tolerance: {}",
            diff
        );
    }
}

// Group B — Synthetic Known-Answer Tests
#[test]
fn test_rsp_synthetic_known_answer_rates() {
    let fs = 100.0;
    let duration = 30.0;
    let n = (fs * duration) as usize;
    let t = Array1::linspace(0.0, duration, n);

    // Test frequencies corresponding to 6, 12, 20, and 30 breaths/min
    let test_rates_bpm = vec![6.0, 12.0, 20.0, 30.0];

    for &known_bpm in &test_rates_bpm {
        let freq_hz = known_bpm / 60.0;
        let raw = t.mapv(|tv| (2.0 * PI * freq_hz * tv).sin());

        let config = RspProcessingConfig::default()
            .with_min_breath_interval_sec(60.0 / (known_bpm + 10.0))
            .with_max_breath_interval_sec(60.0 / (known_bpm - 4.0).max(1.0));

        let cycles = rsp_cycles_config(&raw, fs, &config)
            .unwrap_or_else(|e| panic!("Cycles extraction failed at {} bpm: {:?}", known_bpm, e));

        assert!(
            !cycles.is_empty(),
            "Expected detected cycles at {} bpm",
            known_bpm
        );

        let mean_bpm =
            cycles.iter().map(|c| c.respiratory_rate_bpm).sum::<f64>() / cycles.len() as f64;
        let rate_diff = (mean_bpm - known_bpm).abs();

        assert!(
            rate_diff <= 0.5,
            "Mean BPM error ({:.2}) exceeds tolerance at target {} bpm",
            mean_bpm,
            known_bpm
        );
    }
}

// Group C — Reference Implementation Comparison (NeuroKit2)
#[test]
fn test_rsp_golden_parity_neurokit() {
    let file = match File::open("tests/golden_rsp.json") {
        Ok(f) => f,
        Err(_) => {
            println!("Skipping golden RSP reference test: golden_rsp.json not found");
            return;
        }
    };
    let reader = BufReader::new(file);
    let cases: Vec<GoldenRspCase> =
        serde_json::from_reader(reader).expect("Failed to parse golden_rsp.json");

    assert!(!cases.is_empty(), "Golden RSP dataset is empty");

    let mut total_tp = 0;
    let mut total_fp = 0;
    let mut total_fn = 0;

    for case in &cases {
        let signal = Array1::from_vec(case.raw_signal.clone());
        let fs = case.sampling_rate;
        let expected = &case.expected_peaks;

        let cleaned = rsp_clean(&signal, fs).expect("rsp_clean failed");
        let detected = rsp_findpeaks_config(&cleaned, fs, &RspProcessingConfig::default())
            .expect("rsp_findpeaks_config failed");

        let tol_samples = (0.500 * fs).round() as i64; // 500ms timing tolerance

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
            "RSP Case '{}' (Fs={}Hz, Target={}bpm): TP={}, FP={}, FN={}",
            case.name, fs, case.respiratory_rate, tp, fp, fn_count
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
        "Overall Golden RSP Performance: Precision={:.4}, Recall={:.4}, F1={:.4}",
        precision, recall, f1
    );

    assert!(
        f1 >= 0.75,
        "RSP F1 score ({:.4}) must be >= 0.75 against NeuroKit2 reference",
        f1
    );
}

// Group D — Robustness & Multi-Fs Suite
#[test]
fn test_rsp_multi_sampling_rate_suite() {
    let sampling_rates: Vec<f64> = vec![32.0, 64.0, 100.0, 128.0, 250.0, 500.0];

    for &fs in &sampling_rates {
        let duration = 20.0;
        let n = (fs * duration) as usize;
        let t = Array1::linspace(0.0, duration, n);

        // 15 breaths/min (0.25 Hz) + baseline wander + DC offset
        let raw =
            t.mapv(|tv| 5.0 + 0.3 * (2.0 * PI * 0.02 * tv).sin() + (2.0 * PI * 0.25 * tv).sin());

        let cleaned = rsp_clean(&raw, fs).expect("Clean failed");
        let cycles = rsp_cycles(&cleaned, fs).expect("Cycles failed");
        let rate_array = rsp_rate(&raw, fs).expect("Rsp rate array failed");

        assert!(
            !cycles.is_empty(),
            "Expected detected breath cycles at Fs={}Hz",
            fs
        );
        assert_eq!(rate_array.len(), n);
        assert!(rate_array.iter().all(|&r| r.is_finite()));
    }
}

#[test]
fn test_rsp_edge_case_robustness() {
    let fs = 100.0;
    let n = 1000;

    // Constant signal noise floor protection
    let const_sig = Array1::<f64>::ones(n) * PI;
    let cycles = rsp_cycles(&const_sig, fs).expect("Constant signal cycles failed");
    assert_eq!(cycles.len(), 0, "Constant signal must produce 0 cycles");

    // Extreme amplitude scaling check
    let mut high_amp = Array1::<f64>::zeros(n);
    for sec in 1..9 {
        let idx = sec * 100;
        high_amp[idx] = 1000.0;
    }
    assert!(rsp_clean(&high_amp, fs).is_ok());
}

// Group E — API & Input Validation
#[test]
fn test_rsp_input_validation() {
    let empty_sig = Array1::<f64>::zeros(0);
    assert!(matches!(
        rsp_clean(&empty_sig, 100.0),
        Err(SignalError::EmptySignal)
    ));

    let nan_sig = Array1::from_vec(vec![1.0, f64::NAN, 2.0]);
    assert!(matches!(
        rsp_clean(&nan_sig, 100.0),
        Err(SignalError::NonFiniteInput)
    ));

    let inf_sig = Array1::from_vec(vec![1.0, f64::INFINITY, 2.0]);
    assert!(matches!(
        rsp_clean(&inf_sig, 100.0),
        Err(SignalError::NonFiniteInput)
    ));

    let valid_sig = Array1::<f64>::ones(500);
    assert!(matches!(
        rsp_clean(&valid_sig, 0.0),
        Err(SignalError::InvalidSamplingRate(_))
    ));
    assert!(matches!(
        rsp_clean(&valid_sig, -100.0),
        Err(SignalError::InvalidSamplingRate(_))
    ));

    // Insufficient samples
    let short_sig = Array1::<f64>::ones(5);
    assert!(matches!(
        rsp_clean(&short_sig, 100.0),
        Err(SignalError::InsufficientSamples { .. })
    ));

    // Invalid Cutoff (lowcut >= highcut)
    let inv_cfg = RspCleaningConfig::new()
        .with_lowcut(0.60)
        .with_highcut(0.30);
    assert!(matches!(
        rsp_clean_config(&valid_sig, 100.0, &inv_cfg),
        Err(SignalError::InvalidCutoffFrequency(_))
    ));

    // Invalid Highcut exceeding Nyquist
    let nyq_cfg = RspCleaningConfig::new().with_highcut(60.0);
    assert!(matches!(
        rsp_clean_config(&valid_sig, 100.0, &nyq_cfg),
        Err(SignalError::InvalidCutoffFrequency(_))
    ));
}

#[test]
fn test_rsp_convenience_apis() {
    let fs = 100.0;
    let duration = 20.0;
    let n = (fs * duration) as usize;
    let t = Array1::linspace(0.0, duration, n);
    let raw = t.mapv(|tv| (2.0 * PI * 0.25 * tv).sin());

    let mask = rsp_findpeaks_mask(&raw, fs, &RspProcessingConfig::default())
        .expect("rsp_findpeaks_mask failed");
    assert_eq!(mask.len(), n);

    let compat_mask = rsp_findpeaks(&raw, fs).expect("rsp_findpeaks failed");
    assert_eq!(compat_mask.len(), n);
}

#[test]
fn test_rsp_precleaned_and_slow_breathing() {
    let fs = 50.0;
    // 1 breath every 15 seconds (4 breaths/min, duration 60s -> 4 cycles)
    let duration = 60.0;
    let n = (fs * duration) as usize;
    let t = Array1::linspace(0.0, duration, n);
    let precleaned_sig = t.mapv(|tv| (2.0 * PI * (1.0 / 15.0) * tv).sin());

    let cfg = RspProcessingConfig::default()
        .with_precleaned(true)
        .with_max_breath_interval_sec(20.0);

    let cycles =
        rsp_cycles_config(&precleaned_sig, fs, &cfg).expect("rsp_cycles_config precleaned failed");
    assert!(
        !cycles.is_empty(),
        "Should detect slow breathing cycles (15s interval)"
    );
}
