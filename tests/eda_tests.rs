use lamina::eda::clean::eda_clean;
use lamina::eda::peaks::eda_findpeaks;
use lamina::eda::phasic::eda_phasic;
use ndarray::Array1;

#[test]
fn test_eda_pipeline_mock() {
    let mut signal_vec = vec![0.0; 200];
    for i in 0..200 {
        signal_vec[i] = (i as f64) * 0.01;
    }

    for i in 80..100 {
        signal_vec[i] += (i - 80) as f64 * 0.2;
    }
    for i in 100..120 {
        signal_vec[i] += (120 - i) as f64 * 0.2;
    }

    let signal = Array1::from_vec(signal_vec);

    let cleaned = eda_clean(&signal, 100.0).expect("EDA clean failed");
    assert_eq!(cleaned.len(), signal.len());

    let phasic = eda_phasic(&cleaned, 100.0).expect("EDA phasic failed");
    assert_eq!(phasic.len(), signal.len());

    let peaks = eda_findpeaks(&phasic).expect("EDA findpeaks failed");

    let true_peaks: Vec<usize> = peaks
        .iter()
        .enumerate()
        .filter_map(|(i, &p)| if p { Some(i) } else { None })
        .collect();

    assert!(!true_peaks.is_empty(), "Expected to extract the SCR peak");
    let has_peak = true_peaks.iter().any(|&idx| idx >= 90 && idx <= 110);
    assert!(
        has_peak,
        "Expected to find SCR peak aligned with the rapid rise constraint"
    );
}
