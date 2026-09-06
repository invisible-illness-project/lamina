use lamina::error::SignalError;
use lamina::hrv::intervals::peaks_to_intervals;
use lamina::hrv::time::hrv_rmssd;
use lamina::ppg::{
    PpgPeakDetectionConfig, ppg_clean, ppg_findpeaks, ppg_findpeaks_config, ppg_findpeaks_mask,
};
use ndarray::Array1;
use serde::Deserialize;
use std::f64::consts::PI;
use std::fs::File;
use std::io::BufReader;

#[allow(dead_code)]
#[derive(Deserialize)]
struct GoldenPpgCase {
    name: String,
    sampling_rate: f64,
    heart_rate: f64,
    signal: Vec<f64>,
    expected_systolic_peaks: Vec<usize>,
}

#[test]
fn test_ppg_clean_basic() {
    let signal = Array1::<f64>::ones(100);
    let cleaned = ppg_clean(&signal, 100.0).expect("PPG clean failed");
    assert_eq!(cleaned.len(), signal.len());
}

#[test]
fn test_ppg_input_validation() {
    let empty_sig = Array1::<f64>::zeros(0);
    assert!(matches!(
        ppg_findpeaks(&empty_sig, 100.0),
        Err(SignalError::EmptySignal)
    ));

    let nan_sig = Array1::from_vec(vec![1.0, f64::NAN, 2.0]);
    assert!(matches!(
        ppg_findpeaks(&nan_sig, 100.0),
        Err(SignalError::NonFiniteInput)
    ));

    let inf_sig = Array1::from_vec(vec![1.0, f64::INFINITY, 2.0]);
    assert!(matches!(
        ppg_findpeaks(&inf_sig, 100.0),
        Err(SignalError::NonFiniteInput)
    ));

    let valid_sig = Array1::<f64>::ones(500);
    assert!(matches!(
        ppg_findpeaks(&valid_sig, 0.0),
        Err(SignalError::InvalidSamplingRate(_))
    ));
    assert!(matches!(
        ppg_findpeaks(&valid_sig, -100.0),
        Err(SignalError::InvalidSamplingRate(_))
    ));

    // Insufficient samples check
    let short_sig = Array1::<f64>::ones(5);
    assert!(matches!(
        ppg_findpeaks(&short_sig, 100.0),
        Err(SignalError::InsufficientSamples { .. })
    ));

    let inv_cfg = PpgPeakDetectionConfig::new()
        .with_w_peak_sec(0.800)
        .with_w_beat_sec(0.200);
    assert!(matches!(
        ppg_findpeaks_config(&valid_sig, 100.0, &inv_cfg),
        Err(SignalError::InvalidWindowSize(_))
    ));

    // Invalid highcut >= Nyquist
    let nyq_cfg = PpgPeakDetectionConfig::new().with_highcut(60.0);
    assert!(matches!(
        ppg_findpeaks_config(&valid_sig, 100.0, &nyq_cfg),
        Err(SignalError::InvalidCutoffFrequency(_))
    ));
}

#[test]
fn test_ppg_synthetic_waveforms_multi_fs() {
    let sampling_rates: Vec<f64> = vec![100.0, 128.0, 250.0, 500.0, 1000.0];

    for &fs in &sampling_rates {
        let duration = 10.0;
        let n = (fs * duration) as usize;
        let t = Array1::linspace(0.0, duration, n);

        // 1. Synthetic PPG pulse train (1 pulse/sec = 60 bpm)
        let mut clean_ppg = Array1::<f64>::zeros(n);
        let mut expected_peaks = Vec::new();
        let hw = (0.05 * fs).round() as usize; // ~50ms halfwidth

        for sec in 1..9 {
            let center = (sec as f64 * fs) as usize;
            if center > hw && center + hw < n {
                expected_peaks.push(center);
                let hw_i = hw as i64;
                for offset in -hw_i..=hw_i {
                    let idx = (center as i64 + offset) as usize;
                    if idx < n {
                        let dist = offset as f64 / (hw as f64 / 2.0).max(1.0);
                        clean_ppg[idx] = 2.0 * (-dist * dist).exp();
                    }
                }
            }
        }

        let detected = ppg_findpeaks_config(&clean_ppg, fs, &PpgPeakDetectionConfig::default())
            .unwrap_or_else(|e| panic!("Clean PPG detection failed at Fs={}Hz: {:?}", fs, e));

        assert!(
            !detected.is_empty(),
            "Should detect systolic peaks in clean PPG at Fs={}Hz",
            fs
        );

        let tol_samples = (0.150 * fs).round() as i64;
        for &exp in &expected_peaks {
            let matched = detected
                .iter()
                .any(|&det| (det as i64 - exp as i64).abs() <= tol_samples);
            assert!(
                matched,
                "Expected peak around sample {} matched at Fs={}Hz",
                exp, fs
            );
        }

        // 2. Noisy PPG with baseline wander + DC offset
        let baseline = t.mapv(|tv| 0.3 * (2.0 * PI * 0.15 * tv).sin());
        let noisy_ppg = &clean_ppg + &baseline + 1.0;
        let noisy_detected = ppg_findpeaks(&noisy_ppg, fs)
            .unwrap_or_else(|e| panic!("Noisy PPG detection failed at Fs={}Hz: {:?}", fs, e));
        assert!(
            !noisy_detected.is_empty(),
            "Should detect peaks in noisy PPG with baseline wander at Fs={}Hz",
            fs
        );
    }
}

#[test]
fn test_ppg_edge_case_robustness() {
    let fs = 100.0;
    let n = 1000;

    // Constant signal (no peaks should panic; returns empty)
    let const_sig = Array1::<f64>::ones(n);
    let const_peaks = ppg_findpeaks(&const_sig, fs).expect("Constant signal should not panic");
    let count = const_peaks.iter().filter(|&&p| p).count();
    assert_eq!(count, 0, "Constant signal should produce 0 peaks");

    // High amplitude signal
    let mut high_amp_sig = Array1::<f64>::zeros(n);
    for sec in 1..9 {
        let center = sec * 100;
        for offset in -5..=5 {
            let idx = (center as i64 + offset) as usize;
            if idx < n {
                high_amp_sig[idx] = 500.0;
            }
        }
    }
    let high_amp_peaks = ppg_findpeaks(&high_amp_sig, fs).expect("High amplitude should succeed");
    assert!(high_amp_peaks.iter().any(|&p| p));
}

#[test]
fn test_ppg_golden_parity_neurokit() {
    let file = match File::open("tests/golden_ppg.json") {
        Ok(f) => f,
        Err(_) => {
            println!("Skipping golden PPG reference test: golden_ppg.json not found");
            return;
        }
    };
    let reader = BufReader::new(file);
    let cases: Vec<GoldenPpgCase> =
        serde_json::from_reader(reader).expect("Failed to parse golden_ppg.json");

    assert!(!cases.is_empty(), "Golden PPG dataset is empty");

    let mut total_tp = 0;
    let mut total_fp = 0;
    let mut total_fn = 0;

    for case in &cases {
        let signal = Array1::from_vec(case.signal.clone());
        let fs = case.sampling_rate;
        let expected = &case.expected_systolic_peaks;

        let detected = ppg_findpeaks_config(&signal, fs, &PpgPeakDetectionConfig::default())
            .expect("ppg_findpeaks_config failed");

        let tol_samples = (0.150 * fs).round() as i64;

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
            "PPG Case '{}' (Fs={}Hz, HR={}bpm): TP={}, FP={}, FN={}",
            case.name, fs, case.heart_rate, tp, fp, fn_count
        );
    }

    let precision = total_tp as f64 / (total_tp + total_fp) as f64;
    let recall = total_tp as f64 / (total_tp + total_fn) as f64;
    let f1 = 2.0 * precision * recall / (precision + recall);

    println!(
        "Overall Golden PPG Elgendi Performance: Precision={:.4}, Recall={:.4}, F1={:.4}",
        precision, recall, f1
    );

    assert!(
        f1 >= 0.85,
        "Elgendi F1 score ({:.4}) must be >= 0.85 against NeuroKit2 reference",
        f1
    );
}

#[test]
fn test_ppg_hrv_pipeline_integration() {
    let fs = 100.0;
    let duration = 10.0;
    let n = (fs * duration) as usize;
    let mut clean_ppg = Array1::<f64>::zeros(n);
    for sec in 1..9 {
        let center = (sec as f64 * fs) as usize;
        for offset in -5..=5 {
            let idx = (center as i64 + offset) as usize;
            if idx < n {
                clean_ppg[idx] = 2.0;
            }
        }
    }

    let mask = ppg_findpeaks_mask(&clean_ppg, fs, &PpgPeakDetectionConfig::default())
        .expect("ppg_findpeaks_mask failed");

    let intervals = peaks_to_intervals(&mask, fs).expect("peaks_to_intervals failed");
    assert!(intervals.len() >= 2);

    let rmssd = hrv_rmssd(&intervals).expect("hrv_rmssd failed");
    assert!(rmssd.is_finite());
}
