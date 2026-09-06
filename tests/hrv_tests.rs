use lamina::hrv::intervals::peaks_to_intervals;
use lamina::hrv::time::{hrv_mean_nn, hrv_rmssd};
use ndarray::{Array1, array};

#[test]
fn test_peaks_to_intervals() {
    let mut peaks = Array1::<bool>::from_elem(1000, false);
    peaks[0] = true;
    peaks[100] = true;
    peaks[300] = true;
    peaks[600] = true;

    let intervals = peaks_to_intervals(&peaks, 100.0).expect("Interval calculation failed");

    let expected = array![1000.0, 2000.0, 3000.0];
    assert_eq!(intervals, expected);
}

#[test]
fn test_hrv_rmssd_mean() {
    let intervals = array![800.0, 850.0, 820.0, 900.0];

    let mean = hrv_mean_nn(&intervals).expect("Mean NN failed");
    assert!((mean - 842.5).abs() < 1e-6);

    let rmssd = hrv_rmssd(&intervals).expect("RMSSD failed");
    assert!((rmssd - 57.15476).abs() < 1e-4);
}
