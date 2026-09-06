use lamina::ppg::clean::ppg_clean;
use lamina::ppg::peaks::ppg_findpeaks;
use ndarray::Array1;

#[test]
fn test_ppg_clean_mock() {
    let signal = Array1::<f64>::ones(100);
    let cleaned = ppg_clean(&signal, 1000.0).expect("PPG clean failed");
    assert_eq!(cleaned.len(), signal.len());
}

#[test]
fn test_ppg_findpeaks_mock() {
    let mut signal_vec = vec![0.0; 100];

    for i in 20..30 {
        signal_vec[i] = (i - 20) as f64 * 0.1;
    }
    for i in 30..40 {
        signal_vec[i] = (40 - i) as f64 * 0.1;
    }

    for i in 70..80 {
        signal_vec[i] = (i - 70) as f64 * 0.1;
    }
    for i in 80..90 {
        signal_vec[i] = (90 - i) as f64 * 0.1;
    }

    let signal = Array1::from_vec(signal_vec);
    let peaks = ppg_findpeaks(&signal, 100.0).expect("PPG findpeaks failed");

    let true_peaks: Vec<usize> = peaks
        .iter()
        .enumerate()
        .filter_map(|(i, &p)| if p { Some(i) } else { None })
        .collect();

    assert!(!true_peaks.is_empty(), "Expected to find some peaks");
    let has_first = true_peaks.iter().any(|&idx| idx >= 25 && idx <= 35);
    let has_second = true_peaks.iter().any(|&idx| idx >= 75 && idx <= 85);

    assert!(has_first, "Found first PPG peak");
    assert!(has_second, "Found second PPG peak");
}
