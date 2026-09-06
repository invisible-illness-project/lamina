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
fn test_ecg_clean_method_dispatch() {
    let signal = Array1::<f64>::ones(100);
    for m in ["neurokit", "pantompkins", "biosppy", "", "  NEUROKIT  "] {
        assert!(
            ecg_clean(&signal, 100.0, m).is_ok(),
            "Method '{}' should be supported",
            m
        );
    }
    assert!(
        matches!(
            ecg_clean(&signal, 100.0, "unsupported_method"),
            Err(SignalError::InvalidCutoffFrequency(_))
        ),
        "Unsupported method should return InvalidCutoffFrequency error"
    );
}

#[test]
fn test_ecg_peaks_invalid_threshold_multiplier_error() {
    let signal = Array1::<f64>::ones(500);
    let inv_cfg = EcgPeakDetectionConfig::new().with_threshold_multiplier(1.0);
    let err = ecg_findpeaks_config(&signal, 100.0, &inv_cfg).unwrap_err();
    assert!(
        matches!(err, SignalError::InvalidCutoffFrequency(_)),
        "threshold_multiplier >= 1.0 must return InvalidCutoffFrequency parameter error, got {:?}",
        err
    );
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

    let valid_sig = Array1::<f64>::ones(500);
    assert!(matches!(
        ecg_findpeaks(&valid_sig, 0.0),
        Err(SignalError::InvalidSamplingRate(_))
    ));
    assert!(matches!(
        ecg_findpeaks(&valid_sig, -100.0),
        Err(SignalError::InvalidSamplingRate(_))
    ));

    // Insufficient samples check
    let short_sig = Array1::<f64>::ones(5);
    assert!(matches!(
        ecg_findpeaks(&short_sig, 100.0),
        Err(SignalError::InsufficientSamples { .. })
    ));

    // Invalid config validation (lowcut >= highcut)
    let inv_cfg = EcgPeakDetectionConfig::new()
        .with_lowcut(20.0)
        .with_highcut(10.0);
    assert!(matches!(
        ecg_findpeaks_config(&valid_sig, 100.0, &inv_cfg),
        Err(SignalError::InvalidCutoffFrequency(_))
    ));

    // Invalid highcut >= Nyquist
    let nyq_cfg = EcgPeakDetectionConfig::new().with_highcut(60.0);
    assert!(matches!(
        ecg_findpeaks_config(&valid_sig, 100.0, &nyq_cfg),
        Err(SignalError::InvalidCutoffFrequency(_))
    ));
}

#[test]
fn test_ecg_synthetic_waveforms_multi_fs() {
    let sampling_rates: Vec<f64> = vec![100.0, 128.0, 250.0, 500.0, 1000.0];

    for &fs in &sampling_rates {
        let duration = 10.0;
        let n = (fs * duration) as usize;
        let t = Array1::linspace(0.0, duration, n);

        // 1. Synthetic ECG-like pulse train (1 peak/sec = 60 bpm)
        let mut clean_ecg = Array1::<f64>::zeros(n);
        let mut expected_peaks = Vec::new();
        for sec in 1..9 {
            let idx = (sec as f64 * fs) as usize;
            if idx > 2 && idx + 2 < n {
                expected_peaks.push(idx);
                clean_ecg[idx - 1] = -0.5;
                clean_ecg[idx] = 4.0;
                clean_ecg[idx + 1] = -1.0;
            }
        }

        let detected = ecg_findpeaks_config(&clean_ecg, fs, &EcgPeakDetectionConfig::default())
            .unwrap_or_else(|e| panic!("Clean ECG detection failed at Fs={}Hz: {:?}", fs, e));

        assert!(
            !detected.is_empty(),
            "Should detect peaks in clean synthetic ECG at Fs={}Hz",
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

        // 2. Baseline wander + High-frequency noise + DC Offset
        let baseline = t.mapv(|tv| 0.5 * (2.0 * PI * 0.2 * tv).sin());
        let hf_noise = t.mapv(|tv| 0.05 * (2.0 * PI * 35.0 * tv).cos());
        let noisy_ecg = &clean_ecg + &baseline + &hf_noise + 2.5;

        let noisy_detected = ecg_findpeaks(&noisy_ecg, fs)
            .unwrap_or_else(|e| panic!("Noisy ECG detection failed at Fs={}Hz: {:?}", fs, e));
        assert!(
            !noisy_detected.is_empty(),
            "Should detect peaks in noisy ECG at Fs={}Hz",
            fs
        );
    }
}

#[test]
fn test_ecg_edge_case_robustness() {
    let fs = 100.0;
    let n = 1000;

    // Constant signal (no peaks should panic; returns empty or validated peaks)
    let const_sig = Array1::<f64>::ones(n);
    let const_peaks = ecg_findpeaks(&const_sig, fs).expect("Constant signal should not panic");
    let count = const_peaks.iter().filter(|&&p| p).count();
    assert_eq!(count, 0, "Constant signal should produce 0 peaks");

    // Extreme amplitude signal (scale invariance check)
    let mut high_amp_sig = Array1::<f64>::zeros(n);
    for sec in 1..9 {
        let idx = sec * 100;
        high_amp_sig[idx] = 1000.0;
    }
    let high_amp_peaks = ecg_findpeaks(&high_amp_sig, fs).expect("High amplitude should succeed");
    assert!(high_amp_peaks.iter().any(|&p| p));
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

#[test]
fn test_ecg_6case_regression_matrix() {
    let fs = 250.0;
    let duration = 10.0;
    let n = (fs * duration) as usize;

    // Helper to generate synthetic QRS pulse
    let make_qrs = |amp: f64, width_sec: f64| -> Vec<f64> {
        let len = (width_sec * fs) as usize;
        let mut pulse = Vec::with_capacity(len);
        for i in 0..len {
            let t = (i as f64 - len as f64 / 2.0) / (len as f64 / 4.0);
            pulse.push(amp * (-0.5 * t * t).exp());
        }
        pulse
    };

    // Case 1: Normal sinus QRS
    let mut sig1 = Array1::<f64>::zeros(n);
    for k in 1..9 {
        let pos = (k as f64 * 1.0 * fs) as usize;
        let qrs = make_qrs(1.0, 0.08);
        for (i, &v) in qrs.iter().enumerate() {
            if pos + i < n {
                sig1[pos + i] += v;
            }
        }
    }
    let peaks1 = ecg_findpeaks(&sig1, fs).expect("Case 1 failed");
    assert!(
        peaks1.iter().filter(|&&p| p).count() >= 7,
        "Case 1: Normal QRS should be detected"
    );

    // Case 2: PVCs with 3:1 amplitude disparity (MIT-BIH 228 model)
    let mut sig2 = Array1::<f64>::zeros(n);
    for k in 1..9 {
        let pos = (k as f64 * 1.0 * fs) as usize;
        let amp = if k % 3 == 0 { 3.5 } else { 1.0 }; // 3.5:1 amplitude disparity
        let qrs = make_qrs(amp, 0.08);
        for (i, &v) in qrs.iter().enumerate() {
            if pos + i < n {
                sig2[pos + i] += v;
            }
        }
    }
    let peaks2 = ecg_findpeaks(&sig2, fs).expect("Case 2 failed");
    let count2 = peaks2.iter().filter(|&&p| p).count();
    assert_eq!(
        count2, 8,
        "Case 2: PVC with 3:1 amplitude disparity should detect all 8 beats without blackout"
    );

    // Case 3: Continuous bigeminy (normal - PVC - normal - PVC)
    let mut sig3 = Array1::<f64>::zeros(n);
    for k in 0..6 {
        let pos_norm = ((1.0 + k as f64 * 1.4) * fs) as usize;
        let pos_pvc = ((1.5 + k as f64 * 1.4) * fs) as usize;
        let qrs_norm = make_qrs(1.0, 0.08);
        let qrs_pvc = make_qrs(2.5, 0.12);
        for (i, &v) in qrs_norm.iter().enumerate() {
            if pos_norm + i < n {
                sig3[pos_norm + i] += v;
            }
        }
        for (i, &v) in qrs_pvc.iter().enumerate() {
            if pos_pvc + i < n {
                sig3[pos_pvc + i] += v;
            }
        }
    }
    let peaks3 = ecg_findpeaks(&sig3, fs).expect("Case 3 failed");
    assert!(
        peaks3.iter().filter(|&&p| p).count() >= 10,
        "Case 3: Bigeminy beats should be detected"
    );

    // Case 4: Narrow / biphasic QRS complexes
    let mut sig4 = Array1::<f64>::zeros(n);
    for k in 1..9 {
        let pos = (k as f64 * 1.0 * fs) as usize;
        // Biphasic: positive spike followed immediately by negative trough
        for i in 0..10 {
            if pos + i < n {
                sig4[pos + i] = 1.0 * (i as f64 / 5.0);
            }
            if pos + 10 + i < n {
                sig4[pos + 10 + i] = -1.0 * (1.0 - i as f64 / 5.0);
            }
        }
    }
    let peaks4 = ecg_findpeaks(&sig4, fs).expect("Case 4 failed");
    assert!(
        peaks4.iter().filter(|&&p| p).count() >= 7,
        "Case 4: Biphasic QRS should be detected"
    );

    // Case 5: Paced ECG with sharp pacing spikes
    let mut sig5 = sig1.clone();
    for k in 1..9 {
        let pos = (k as f64 * 1.0 * fs) as usize - 5;
        if pos < n {
            sig5[pos] = 5.0;
        } // narrow sharp pacing spike 5 samples before QRS
    }
    let peaks5 = ecg_findpeaks(&sig5, fs).expect("Case 5 failed");
    assert!(
        peaks5.iter().filter(|&&p| p).count() >= 7,
        "Case 5: Paced ECG should detect R-peaks"
    );

    // Case 6: High-frequency EMG / motion noise bursts
    let mut sig6 = sig1.clone();
    for i in (3.0 * fs) as usize..(4.0 * fs) as usize {
        if i < n {
            sig6[i] += 0.2 * ((i as f64 * 50.0).sin());
        } // 50 Hz noise burst
    }
    let peaks6 = ecg_findpeaks(&sig6, fs).expect("Case 6 failed");
    assert!(
        peaks6.iter().filter(|&&p| p).count() >= 7,
        "Case 6: EMG noise burst should not prevent detection of R-peaks"
    );
}
