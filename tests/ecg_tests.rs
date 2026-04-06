use ndarray::Array1;
use lamina::ecg::clean::ecg_clean;
use lamina::ecg::peaks::ecg_findpeaks;

#[test]
fn test_ecg_clean_mock() {
    let signal = Array1::<f64>::ones(100);
    // ecg_clean is currently a passthrough based on our scaffold logic. It should return same length.
    let cleaned = ecg_clean(&signal, 1000.0, "neurokit");
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
    let peaks = ecg_findpeaks(&signal, 10.0);

    // We expect the integrated differential signal to have peaks around indices 50 and 150
    // Because of the windowing (150ms @ 10Hz = 1.5 samples), the moving average 
    // spreads out the difference square. For this coarse threshold heuristic, we
    // mostly want to see that at least some peaks are detected.
    let true_peaks: Vec<usize> = peaks.iter()
        .enumerate()
        .filter_map(|(i, &p)| if p { Some(i) } else { None })
        .collect();

    assert!(!true_peaks.is_empty(), "Expected to find some peaks");
    
    // Print/verify finding the specific region peaks
    let has_first = true_peaks.iter().any(|&idx| idx >= 45 && idx <= 55);
    let has_second = true_peaks.iter().any(|&idx| idx >= 145 && idx <= 155);

    assert!(has_first, "Expected to find first simulated QRS peak");
    assert!(has_second, "Expected to find second simulated QRS peak");
}
