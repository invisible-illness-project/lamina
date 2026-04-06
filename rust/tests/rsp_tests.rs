use ndarray::Array1;
use lamina::rsp::clean::rsp_clean;
use lamina::rsp::peaks::rsp_findpeaks;

#[test]
fn test_rsp_pipeline_mock() {
    let mut signal_vec = vec![0.0; 200];
    
    // Broad, slow breath wave (e.g. 15 breaths per minute => 0.25Hz => 4 seconds per breath)
    // At 10Hz sampling rate, 1 cycle = 40 samples
    for i in 0..200 {
        use std::f64::consts::PI;
        // Generate sine wave
        signal_vec[i] = (2.0 * PI * (i as f64) / 40.0).sin();
    }

    let signal = Array1::from_vec(signal_vec);
    
    let cleaned = rsp_clean(&signal, 10.0);
    assert_eq!(cleaned.len(), signal.len());

    let peaks = rsp_findpeaks(&cleaned);
    let true_peaks: Vec<usize> = peaks.iter()
        .enumerate()
        .filter_map(|(i, &p)| if p { Some(i) } else { None })
        .collect();

    // In 200 samples (20 seconds), we expect 20 / 4 = 5 breath peaks.
    assert!(true_peaks.len() >= 3 && true_peaks.len() <= 6, "Expected ~5 respiratory peaks!");
}
