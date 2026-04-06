use ndarray::Array1;
use lamina::eda::clean::eda_clean;
use lamina::eda::phasic::eda_phasic;
use lamina::eda::peaks::eda_findpeaks;

#[test]
fn test_eda_pipeline_mock() {
    // Generate a long linear signal (tonic baseline)
    let mut signal_vec = vec![0.0; 200];
    for i in 0..200 {
        signal_vec[i] = (i as f64) * 0.01;
    }
    
    // Add an SCR (Skin Conductance Response) spike, which is a fast upward swing
    for i in 80..100 {
        signal_vec[i] += (i - 80) as f64 * 0.2;
    }
    for i in 100..120 {
        signal_vec[i] += (120 - i) as f64 * 0.2;
    }

    let signal = Array1::from_vec(signal_vec);
    
    // Clean
    let cleaned = eda_clean(&signal, 100.0);
    assert_eq!(cleaned.len(), signal.len());

    // Isolate Phasic component
    let phasic = eda_phasic(&cleaned, 100.0);
    assert_eq!(phasic.len(), signal.len());

    // Find peaks
    let peaks = eda_findpeaks(&phasic);

    let true_peaks: Vec<usize> = peaks.iter()
        .enumerate()
        .filter_map(|(i, &p)| if p { Some(i) } else { None })
        .collect();

    assert!(!true_peaks.is_empty(), "Expected to extract the SCR peak");
    let has_peak = true_peaks.iter().any(|&idx| idx >= 90 && idx <= 110);
    assert!(has_peak, "Expected to find SCR peak aligned with the rapid rise constraint");
}
