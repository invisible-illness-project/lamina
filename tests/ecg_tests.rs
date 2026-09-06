use lamina::ecg::{
    EcgPeakDetectionConfig, ecg_clean, ecg_findpeaks, ecg_findpeaks_config, ecg_findpeaks_mask,
};
use lamina::error::SignalError;
use lamina::hrv::intervals::peaks_to_intervals;
use lamina::hrv::time::hrv_rmssd;
use ndarray::Array1;
use serde::Deserialize;
use std::f64::consts::PI;
use std::fs::File;
use std::io::BufReader;

#[allow(dead_code)]
#[derive(Deserialize)]
struct GoldenEcgCase {
    name: String,
    sampling_rate: f64,
    heart_rate: f64,
    noise: f64,
    signal: Vec<f64>,
    expected_r_peaks: Vec<usize>,
}

#[test]
fn test_ecg_clean_basic() {
    let signal = Array1::<f64>::ones(100);
    let cleaned = ecg_clean(&signal, 100.0, "neurokit").expect("ECG clean failed");
    assert_eq!(cleaned.len(), signal.len());
}

#[test]
fn test_ecg_input_validation() {
    let empty_sig = Array1::<f64>::zeros(0);
    assert!(matches!(
        ecg_findpeaks(&empty_sig, 100.0),
        Err(SignalError::EmptySignal)
    ));

    let nan_sig = Array1::from_vec(vec![1.0, f64::NAN, 2.0]);
    assert!(matches!(
        ecg_findpeaks(&nan_sig, 100.0),
        Err(SignalError::NonFiniteInput)
    ));

    let inf_sig = Array1::from_vec(vec![1.0, f64::INFINITY, 2.0]);
    assert!(matches!(
        ecg_findpeaks(&inf_sig, 100.0),
        Err(SignalError::NonFiniteInput)
    ));

    let valid_sig = Array1::<f64>::ones(100);
    assert!(matches!(
        ecg_findpeaks(&valid_sig, 0.0),
        Err(SignalError::InvalidSamplingRate(_))
    ));
    assert!(matches!(
        ecg_findpeaks(&valid_sig, -100.0),
        Err(SignalError::InvalidSamplingRate(_))
    ));

    // Invalid config validation
    let inv_cfg = EcgPeakDetectionConfig::new()
        .with_lowcut(20.0)
        .with_highcut(10.0);
    assert!(matches!(
        ecg_findpeaks_config(&valid_sig, 100.0, &inv_cfg),
        Err(SignalError::InvalidCutoffFrequency(_))
    ));
}

#[test]
fn test_ecg_synthetic_waveforms() {
    let fs = 100.0;
    let duration = 10.0;
    let n = (fs * duration) as usize;
    let t = Array1::linspace(0.0, duration, n);

    // 1. Synthetic ECG-like pulse train (approx 60 bpm = 1 peak/sec)
    let mut clean_ecg = Array1::<f64>::zeros(n);
    let mut expected_peaks = Vec::new();
    for sec in 1..9 {
        let idx = (sec as f64 * fs) as usize;
        expected_peaks.push(idx);
        clean_ecg[idx - 1] = -0.5;
        clean_ecg[idx] = 4.0;
        clean_ecg[idx + 1] = -1.0;
    }

    let detected = ecg_findpeaks_config(&clean_ecg, fs, &EcgPeakDetectionConfig::default())
        .expect("Clean ECG detection failed");

    assert!(
        !detected.is_empty(),
        "Should detect peaks in clean synthetic ECG"
    );
    for &exp in &expected_peaks {
        let matched = detected
            .iter()
            .any(|&det| (det as i64 - exp as i64).abs() <= 15);
        assert!(
            matched,
            "Expected peak around sample {} matched in detected peaks",
            exp
        );
    }

    // 2. Tachycardia (150 bpm = 2.5 Hz = peak every 40 samples at 100 Hz)
    let mut tachy_ecg = Array1::<f64>::zeros(n);
    let mut tachy_expected = Vec::new();
    for i in (40..n - 40).step_by(40) {
        tachy_expected.push(i);
        tachy_ecg[i - 1] = -0.3;
        tachy_ecg[i] = 3.5;
        tachy_ecg[i + 1] = -0.8;
    }

    let tachy_cfg = EcgPeakDetectionConfig::new().with_refractory_period_sec(0.150);
    let tachy_detected =
        ecg_findpeaks_config(&tachy_ecg, fs, &tachy_cfg).expect("Tachycardia detection failed");
    assert!(
        !tachy_detected.is_empty(),
        "Should detect peaks in tachycardia ECG"
    );

    // 3. Bradycardia (45 bpm = 0.75 Hz = peak every 133 samples at 100 Hz)
    let mut brady_ecg = Array1::<f64>::zeros(n);
    for i in (133..n - 133).step_by(133) {
        brady_ecg[i] = 4.0;
    }
    let brady_detected = ecg_findpeaks(&brady_ecg, fs).expect("Bradycardia detection failed");
    assert!(
        !brady_detected.is_empty(),
        "Should detect peaks in bradycardia ECG"
    );

    // 4. Baseline wander + noise
    let baseline_wander = t.mapv(|tv| 0.5 * (2.0 * PI * 0.2 * tv).sin());
    let noisy_ecg = &clean_ecg + &baseline_wander;
    let noisy_detected = ecg_findpeaks(&noisy_ecg, fs).expect("Noisy ECG detection failed");
    assert!(
        !noisy_detected.is_empty(),
        "Should detect peaks in ECG with baseline wander"
    );
}

#[test]
fn test_ecg_golden_parity_neurokit() {
    let file = match File::open("tests/golden_ecg.json") {
        Ok(f) => f,
        Err(_) => {
            println!("Skipping golden ECG reference test: golden_ecg.json not found");
            return;
        }
    };
    let reader = BufReader::new(file);
    let cases: Vec<GoldenEcgCase> =
        serde_json::from_reader(reader).expect("Failed to parse golden_ecg.json");

    assert!(!cases.is_empty(), "Golden ECG dataset is empty");

    let mut total_tp = 0;
    let mut total_fp = 0;
    let mut total_fn = 0;

    for case in &cases {
        let signal = Array1::from_vec(case.signal.clone());
        let fs = case.sampling_rate;
        let expected = &case.expected_r_peaks;

        let detected = ecg_findpeaks_config(&signal, fs, &EcgPeakDetectionConfig::default())
            .expect("ecg_findpeaks_config failed");

        // Time tolerance: +-150ms converted to samples
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
            "ECG Case '{}' (Fs={}Hz, HR={}bpm): TP={}, FP={}, FN={}",
            case.name, fs, case.heart_rate, tp, fp, fn_count
        );
    }

    let precision = total_tp as f64 / (total_tp + total_fp) as f64;
    let recall = total_tp as f64 / (total_tp + total_fn) as f64;
    let f1 = 2.0 * precision * recall / (precision + recall);

    println!(
        "Overall Golden ECG Pan-Tompkins Performance: Precision={:.4}, Recall={:.4}, F1={:.4}",
        precision, recall, f1
    );

    assert!(
        f1 >= 0.85,
        "Pan-Tompkins F1 score ({:.4}) must be >= 0.85 against NeuroKit2 reference",
        f1
    );
}

#[test]
fn test_ecg_hrv_pipeline_integration() {
    let fs = 100.0;
    let duration = 10.0;
    let n = (fs * duration) as usize;
    let mut clean_ecg = Array1::<f64>::zeros(n);
    for sec in 1..9 {
        let idx = (sec as f64 * fs) as usize;
        clean_ecg[idx] = 4.0;
    }

    let mask = ecg_findpeaks_mask(&clean_ecg, fs, &EcgPeakDetectionConfig::default())
        .expect("ecg_findpeaks_mask failed");

    let intervals = peaks_to_intervals(&mask, fs).expect("peaks_to_intervals failed");
    assert!(intervals.len() >= 2);

    let rmssd = hrv_rmssd(&intervals).expect("hrv_rmssd failed");
    assert!(rmssd.is_finite());
}
