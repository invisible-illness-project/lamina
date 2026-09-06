use lamina::ecg::clean::ecg_clean;
use lamina::ecg::peaks::ecg_findpeaks;
use ndarray::Array1;
use serde::Deserialize;
use std::fs::File;
use std::io::BufReader;

#[allow(dead_code)]
#[derive(Deserialize)]
struct GoldenData {
    sampling_rate: f64,
    raw_signal: Vec<f64>,
    cleaned_signal: Vec<f64>,
    expected_r_peaks: Vec<usize>,
}

#[test]
fn test_ecg_against_golden_dataset() {
    let file = File::open("tests/golden_ecg.json").expect("Failed to open golden dataset");
    let reader = BufReader::new(file);
    let data: GoldenData = serde_json::from_reader(reader).expect("Failed to parse JSON");

    let signal = Array1::from_vec(data.raw_signal);

    // 1. Clean the signal
    let cleaned = ecg_clean(&signal, data.sampling_rate, "neurokit").expect("ECG clean failed");
    assert_eq!(
        cleaned.len(),
        signal.len(),
        "Signal length should not change after cleaning"
    );

    // 2. Find peaks
    let rust_peaks = ecg_findpeaks(&cleaned, data.sampling_rate).expect("ECG findpeaks failed");
    let rust_peak_indices: Vec<usize> = rust_peaks
        .iter()
        .enumerate()
        .filter_map(|(i, &p)| if p { Some(i) } else { None })
        .collect();

    println!("Expected peaks (NeuroKit2): {:?}", data.expected_r_peaks);
    println!("Rust peaks detected:     {:?}", rust_peak_indices);

    // Category 3 Event Detection Parity: Verify each expected peak has a Rust peak within +/- 15 samples (150ms at 100Hz)
    let tolerance = 15;

    for expected in &data.expected_r_peaks {
        let found = rust_peak_indices
            .iter()
            .any(|&r| (r as isize - *expected as isize).abs() <= tolerance);
        assert!(
            found,
            "Failed to find R-peak near index {} within a tolerance of {} samples",
            expected, tolerance
        );
    }
}
