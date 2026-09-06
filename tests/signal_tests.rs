use lamina::error::SignalError;
use lamina::signal::filter::signal_filter;
use lamina::signal::peaks::signal_findpeaks;
use lamina::signal::smooth::signal_smooth_moving_average;
use ndarray::{Array1, array};

#[test]
fn test_signal_smooth_moving_average() {
    let signal = array![1.0, 2.0, 3.0, 4.0, 5.0];
    let smoothed = signal_smooth_moving_average(&signal, 3).expect("Smoothing failed");

    let expected = array![1.5, 2.0, 3.0, 4.0, 4.5];

    for (a, b) in smoothed.iter().zip(expected.iter()) {
        assert!((a - b).abs() < 1e-6, "Expected {}, got {}", b, a);
    }
}

#[test]
fn test_signal_findpeaks() {
    let signal = array![0.0, 1.0, 0.0, 2.0, 1.0, 3.0, 3.0, 0.0];
    let peaks = signal_findpeaks(&signal).expect("Peak detection failed");

    // find_peaks identifies strict local maxima at idx 1, 3 and plateau peak at idx 5
    let expected = array![false, true, false, true, false, true, false, false];
    assert_eq!(peaks, expected);
}

#[test]
fn test_signal_smooth_edge_cases() {
    let empty = Array1::<f64>::zeros(0);
    assert_eq!(
        signal_smooth_moving_average(&empty, 3),
        Err(SignalError::EmptySignal)
    );

    let signal = array![1.0, 2.0, 3.0];
    assert_eq!(
        signal_smooth_moving_average(&signal, 0),
        Err(SignalError::InvalidWindowSize(0))
    );

    let nan_signal = array![1.0, f64::NAN, 3.0];
    assert_eq!(
        signal_smooth_moving_average(&nan_signal, 3),
        Err(SignalError::NonFiniteInput)
    );
}

#[test]
fn test_signal_filter_edge_cases() {
    let signal = array![1.0, 2.0, 3.0, 4.0, 5.0];
    // Invalid sampling rate
    assert!(signal_filter(&signal, -100.0, Some(0.5), Some(5.0), 2).is_err());
    // Cutoff frequency above Nyquist limit (100Hz sampling rate => Nyquist = 50Hz)
    assert!(signal_filter(&signal, 100.0, Some(0.5), Some(60.0), 2).is_err());
    // Highcut <= Lowcut
    assert!(signal_filter(&signal, 100.0, Some(10.0), Some(5.0), 2).is_err());
}
