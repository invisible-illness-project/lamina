use ndarray::Array1;
use crate::signal::smooth::signal_smooth_moving_average;
use crate::signal::peaks::signal_findpeaks;

/// Find R-peaks in an ECG signal.
/// Uses a simplified derivative-based approach similar to Pan-Tompkins for algorithmic scaffolding.
pub fn ecg_findpeaks(
    signal: &Array1<f64>,
    sampling_rate: f64,
) -> Array1<bool> {
    let n = signal.len();
    
    // 1. Differentiate (first derivative)
    let mut diff = Array1::<f64>::zeros(n);
    for i in 1..(n-1) {
        // Simple central difference
        diff[i] = signal[i + 1] - signal[i - 1];
    }

    // 2. Square the signal
    let mut sq_diff = diff.mapv(|x| x * x);
    
    // 3. Moving average (approx 150ms window)
    let window_size = (0.150 * sampling_rate) as usize;
    let integrated = signal_smooth_moving_average(&sq_diff, window_size.max(1));

    // 4. Find local maxima
    let mut peaks = signal_findpeaks(&integrated);
    
    // 5. Apply thresholding (only keep peaks > mean of integrated signal)
    let mean_val = integrated.mean().unwrap_or(0.0);
    for i in 0..n {
        if peaks[i] && integrated[i] <= mean_val * 1.5 {
            peaks[i] = false;
        }
    }

    peaks
}
