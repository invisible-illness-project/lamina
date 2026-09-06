use lamina::error::SignalError;
use lamina::signal::peaks::{PeakDetectionConfig, signal_findpeaks, signal_findpeaks_config};
use ndarray::Array1;
use serde::Deserialize;
use std::fs::File;
use std::io::BufReader;

#[derive(Deserialize)]
struct TestCase {
    name: String,
    min_height: Option<f64>,
    min_distance: Option<usize>,
    min_prominence: Option<f64>,
    expected_peaks: Vec<usize>,
}

#[derive(Deserialize)]
struct GoldenPeaks {
    signal: Vec<f64>,
    ripple_signal: Vec<f64>,
    test_cases: Vec<TestCase>,
}

#[test]
fn test_scipy_peak_detection_parity() {
    let file = File::open("tests/golden_peaks.json").expect("Failed to open golden peaks JSON");
    let reader = BufReader::new(file);
    let data: GoldenPeaks =
        serde_json::from_reader(reader).expect("Failed to parse golden peaks JSON");

    let sig_multi = Array1::from_vec(data.signal);
    let sig_ripple = Array1::from_vec(data.ripple_signal);

    for tc in &data.test_cases {
        let input = if tc.name == "ripple_filter" {
            &sig_ripple
        } else {
            &sig_multi
        };

        let mut config = PeakDetectionConfig::new();
        if let Some(h) = tc.min_height {
            config = config.with_min_height(h);
        }
        if let Some(d) = tc.min_distance {
            config = config.with_min_distance(d);
        }
        if let Some(p) = tc.min_prominence {
            config = config.with_min_prominence(p);
        }

        let actual_indices =
            signal_findpeaks_config(input, &config).expect("signal_findpeaks_config failed");

        println!(
            "Test Case '{:18}': Expected {:?}, Got {:?}",
            tc.name, tc.expected_peaks, actual_indices
        );

        assert_eq!(
            actual_indices, tc.expected_peaks,
            "SciPy peak index mismatch for test case '{}'",
            tc.name
        );
    }
}

#[test]
fn test_deterministic_peak_constraints() {
    // Height filtering
    let sig_ht = Array1::from_vec(vec![0.0, 1.0, 0.0, 5.0, 0.0, 2.0, 0.0]);
    let cfg_ht = PeakDetectionConfig::new().with_min_height(3.0);
    let peaks_ht = signal_findpeaks_config(&sig_ht, &cfg_ht).expect("Findpeaks failed");
    assert_eq!(peaks_ht, vec![3]);

    // Distance filtering
    let sig_dist = Array1::from_vec(vec![0.0, 5.0, 0.0, 4.0, 0.0, 5.0, 0.0]);
    let cfg_dist = PeakDetectionConfig::new().with_min_distance(3);
    let peaks_dist = signal_findpeaks_config(&sig_dist, &cfg_dist).expect("Findpeaks failed");
    assert_eq!(peaks_dist.len(), 2);

    // Constant signal (no peaks)
    let sig_const = Array1::<f64>::from_elem(100, 2.5);
    let peaks_const = signal_findpeaks(&sig_const).expect("Findpeaks failed");
    assert!(!peaks_const.iter().any(|&p| p));

    // Monotonic increasing signal (no peaks)
    let sig_mono = Array1::from_vec((0..100).map(|i| i as f64).collect());
    let peaks_mono = signal_findpeaks(&sig_mono).expect("Findpeaks failed");
    assert!(!peaks_mono.iter().any(|&p| p));

    // Threshold filtering
    let sig_thresh = Array1::from_vec(vec![0.0, 2.0, 1.9, 5.0, 1.0, 0.0]);
    let cfg_thresh = PeakDetectionConfig::new().with_threshold(1.0);
    let peaks_thresh = signal_findpeaks_config(&sig_thresh, &cfg_thresh).expect("Findpeaks failed");
    assert_eq!(peaks_thresh, vec![3]);
}

#[test]
fn test_invalid_peak_configs() {
    let sig = Array1::from_vec(vec![0.0, 1.0, 0.0]);

    // Distance 0
    let cfg_d0 = PeakDetectionConfig::new().with_min_distance(0);
    assert!(matches!(
        signal_findpeaks_config(&sig, &cfg_d0),
        Err(SignalError::InvalidWindowSize(0))
    ));

    // Non-finite height
    let cfg_nan = PeakDetectionConfig::new().with_min_height(f64::NAN);
    assert!(matches!(
        signal_findpeaks_config(&sig, &cfg_nan),
        Err(SignalError::NonFiniteInput)
    ));

    // Empty signal
    let empty = Array1::<f64>::zeros(0);
    assert!(matches!(
        signal_findpeaks(&empty),
        Err(SignalError::EmptySignal)
    ));
}
