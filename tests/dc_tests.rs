use lamina::signal::dc::signal_remove_dc;
use ndarray::Array1;

#[test]
fn test_signal_remove_dc() {
    let arr = Array1::from_vec(vec![10.0, 20.0, 30.0, 40.0]);
    let cleaned = signal_remove_dc(&arr).unwrap();

    // Mean of [10, 20, 30, 40] is 25
    // Output: [-15, -5, 5, 15]
    assert_eq!(cleaned, Array1::from_vec(vec![-15.0, -5.0, 5.0, 15.0]));

    let mean: f64 = cleaned.iter().sum::<f64>() / cleaned.len() as f64;
    assert!(mean.abs() < 1e-12);
}

#[test]
fn test_signal_remove_dc_constant() {
    let arr = Array1::from_vec(vec![5.0, 5.0, 5.0, 5.0]);
    let cleaned = signal_remove_dc(&arr).unwrap();
    assert_eq!(cleaned, Array1::<f64>::zeros(4));
}
