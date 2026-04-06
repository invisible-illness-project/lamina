use ndarray::Array1;
use crate::signal::smooth::signal_smooth_moving_average;
use crate::signal::peaks::signal_findpeaks;

/// Find systolic peaks in a PPG signal.
pub fn ppg_findpeaks(
    signal: &Array1<f64>,
    sampling_rate: f64,
) -> Array1<bool> {
    let n = signal.len();
    if n == 0 {
        return Array1::<bool>::from_elem(0, false);
    }
    
    // Smooth PPG signal heavily to remove dicrotic notches
    // Typically PPG peaks are slower. We smooth over ~0.10s
    let window_size = (0.100 * sampling_rate).max(1.0) as usize;
    let smoothed = signal_smooth_moving_average(signal, window_size);

    // Find peaks
    let mut peaks = signal_findpeaks(&smoothed);

    // Discard peaks below the mean (simple adaptive threshold)
    let mean_val = smoothed.mean().unwrap_or(0.0);
    for i in 0..n {
        if peaks[i] && smoothed[i] <= mean_val {
            peaks[i] = false;
        }
    }

    peaks
}
