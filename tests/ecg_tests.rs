use lamina::ecg::clean::ecg_clean;
use lamina::ecg::peaks::ecg_findpeaks;
use ndarray::Array1;

#[test]
fn test_ecg_clean_mock() {
    let signal = Array1::<f64>::ones(100);
    let cleaned = ecg_clean(&signal, 1000.0, "neurokit").expect("ECG clean failed");
    assert_eq!(cleaned.len(), signal.len());
}

#[test]
fn test_ecg_findpeaks_mock() {
    let mut signal_vec = vec![0.0; 200];

    // Create spike at index 50
    signal_vec[49] = 1.0;
    signal_vec[50] = 5.0;
    signal_vec[51] = -1.0;

    // Create spike at index 150
    signal_vec[149] = 1.0;
    signal_vec[150] = 6.0;
    signal_vec[151] = -2.0;

    let signal = Array1::from_vec(signal_vec);
    let peaks = ecg_findpeaks(&signal, 10.0).expect("ECG findpeaks failed");

    let true_peaks: Vec<usize> = peaks
        .iter()
        .enumerate()
        .filter_map(|(i, &p)| if p { Some(i) } else { None })
        .collect();

    assert!(!true_peaks.is_empty(), "Expected to find some peaks");

    let has_first = true_peaks.iter().any(|&idx| idx >= 45 && idx <= 55);
    let has_second = true_peaks.iter().any(|&idx| idx >= 145 && idx <= 155);

    assert!(has_first, "Expected to find first simulated QRS peak");
    assert!(has_second, "Expected to find second simulated QRS peak");
}
