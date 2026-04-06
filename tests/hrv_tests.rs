use ndarray::{array, Array1};
use lamina::hrv::intervals::peaks_to_intervals;
use lamina::hrv::time::{hrv_rmssd, hrv_mean_nn};

#[test]
fn test_peaks_to_intervals() {
    let mut peaks = Array1::<bool>::from_elem(1000, false);
    // Synthetic peaks at 0, 100, 300, 600
    peaks[0] = true;
    peaks[100] = true;
    peaks[300] = true;
    peaks[600] = true;
    
    // Sampling rate = 100 Hz. So 100 samples = 1000 ms.
    let intervals = peaks_to_intervals(&peaks, 100.0);
    
    let expected = array![1000.0, 2000.0, 3000.0];
    assert_eq!(intervals, expected);
}

#[test]
fn test_hrv_rmssd_mean() {
    let intervals = array![800.0, 850.0, 820.0, 900.0];
    
    let mean = hrv_mean_nn(&intervals).unwrap();
    // (800+850+820+900) / 4 = 3370 / 4 = 842.5
    assert!((mean - 842.5).abs() < 1e-6);

    // Differences: 50, -30, 80
    // Squares: 2500, 900, 6400
    // Sum squares: 9800
    // Mean square (N-1=3): 3266.666...
    // sqrt: 57.1547...
    let rmssd = hrv_rmssd(&intervals).unwrap();
    assert!((rmssd - 57.15476).abs() < 1e-4);
}
