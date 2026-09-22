use lamina::error::SignalError;
use lamina::signal::normalize::{DegeneratePolicy, signal_minmax};
use ndarray::Array1;

#[test]
fn test_signal_minmax_standard() {
    let arr = Array1::from_vec(vec![10.0, 20.0, 30.0, 40.0, 50.0]);
    let norm = signal_minmax(&arr, (0.0, 1.0), DegeneratePolicy::Error).unwrap();

    assert_eq!(norm[0], 0.0);
    assert_eq!(norm[4], 1.0);
    assert_eq!(norm[2], 0.5);

    let norm_neg = signal_minmax(&arr, (-1.0, 1.0), DegeneratePolicy::Error).unwrap();
    assert_eq!(norm_neg[0], -1.0);
    assert_eq!(norm_neg[4], 1.0);
    assert_eq!(norm_neg[2], 0.0);
}

#[test]
fn test_signal_minmax_degenerate_policies() {
    let flat = Array1::from_vec(vec![5.0, 5.0, 5.0, 5.0]);

    // 1. Error policy
    assert_eq!(
        signal_minmax(&flat, (0.0, 1.0), DegeneratePolicy::Error),
        Err(SignalError::DegenerateSignal)
    );

    // 2. Zero policy
    let zero_norm = signal_minmax(&flat, (0.0, 1.0), DegeneratePolicy::Zero).unwrap();
    assert_eq!(zero_norm, Array1::<f64>::zeros(4));

    // 3. Midpoint policy
    let mid_norm = signal_minmax(&flat, (0.0, 1.0), DegeneratePolicy::Midpoint).unwrap();
    assert_eq!(mid_norm, Array1::from_elem(4, 0.5));
}
