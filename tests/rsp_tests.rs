use lamina::rsp::clean::rsp_clean;
use lamina::rsp::peaks::rsp_findpeaks;
use ndarray::Array1;

#[test]
fn test_rsp_pipeline_mock() {
    let mut signal_vec = vec![0.0; 200];

    for i in 0..200 {
        use std::f64::consts::PI;
        signal_vec[i] = (2.0 * PI * (i as f64) / 40.0).sin();
    }

    let signal = Array1::from_vec(signal_vec);

    let cleaned = rsp_clean(&signal, 10.0).expect("RSP clean failed");
    assert_eq!(cleaned.len(), signal.len());

    let peaks = rsp_findpeaks(&cleaned).expect("RSP findpeaks failed");
    let true_peaks: Vec<usize> = peaks
        .iter()
        .enumerate()
        .filter_map(|(i, &p)| if p { Some(i) } else { None })
        .collect();

    assert!(
        true_peaks.len() >= 3 && true_peaks.len() <= 6,
        "Expected ~5 respiratory peaks!"
    );
}
