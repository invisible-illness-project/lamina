use lamina::ecg::clean::ecg_clean;
use lamina::ecg::peaks::ecg_findpeaks;
use ndarray::Array1;
use serde::Deserialize;
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
fn test_ecg_against_golden_dataset() {
    let file = File::open("tests/golden_ecg.json").expect("Failed to open golden dataset");
    let reader = BufReader::new(file);
    let cases: Vec<GoldenEcgCase> = serde_json::from_reader(reader).expect("Failed to parse JSON");

    for case in cases {
        let signal = Array1::from_vec(case.signal);

        // 1. Clean the signal
        let cleaned =
            ecg_clean(&signal, case.sampling_rate, "pantompkins").expect("ECG clean failed");
        assert_eq!(
            cleaned.len(),
            signal.len(),
            "Signal length should not change after cleaning"
        );

        // 2. Find peaks
        let rust_peaks = ecg_findpeaks(&cleaned, case.sampling_rate).expect("ECG findpeaks failed");
        let rust_peak_indices: Vec<usize> = rust_peaks
            .iter()
            .enumerate()
            .filter_map(|(i, &p)| if p { Some(i) } else { None })
            .collect();

        // Verification tolerance (150ms)
        let tolerance = (0.150 * case.sampling_rate).round() as isize;

        let mut tp = 0;
        for expected in &case.expected_r_peaks {
            let found = rust_peak_indices
                .iter()
                .any(|&r| (r as isize - *expected as isize).abs() <= tolerance);
            if found {
                tp += 1;
            }
        }

        let recall = tp as f64 / case.expected_r_peaks.len() as f64;
        assert!(
            recall >= 0.75,
            "Case {} failed recall check (recall = {:.2})",
            case.name,
            recall
        );
    }
}
