use ndarray::{array, Array1};
use neurokit_rs::signal::smooth::signal_smooth_moving_average;
use neurokit_rs::signal::peaks::signal_findpeaks;

#[test]
fn test_signal_smooth_moving_average() {
    let signal = array![1.0, 2.0, 3.0, 4.0, 5.0];
    let smoothed = signal_smooth_moving_average(&signal, 3);
    
    // For win=3, half_win=1
    // i=0: [0..2] -> (1+2)/2 = 1.5
    // i=1: [0..3] -> (1+2+3)/3 = 2.0
    // i=2: [1..4] -> (2+3+4)/3 = 3.0
    // i=3: [2..5] -> (3+4+5)/3 = 4.0
    // i=4: [3..5] -> (4+5)/2 = 4.5
    
    let expected = array![1.5, 2.0, 3.0, 4.0, 4.5];
    
    for (a, b) in smoothed.iter().zip(expected.iter()) {
        assert!((a - b).abs() < 1e-6, "Expected {}, got {}", b, a);
    }
}

#[test]
fn test_signal_findpeaks() {
    let signal = array![0.0, 1.0, 0.0, 2.0, 1.0, 3.0, 3.0, 0.0];
    let peaks = signal_findpeaks(&signal);
    
    let expected = array![false, true, false, true, false, false, false, false];
    assert_eq!(peaks, expected);
}
