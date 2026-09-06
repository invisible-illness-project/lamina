use crate::error::{Result, SignalError};
use crate::signal::peaks::signal_findpeaks;
use crate::signal::smooth::signal_smooth_moving_average;
use ndarray::Array1;

/// Locate R-peaks in an Electrocardiogram (ECG) signal using Pan-Tompkins derivative & integration scaffolding.
///
/// # Scientific Contract
/// - **Inputs**:
///   - `signal`: Cleaned or raw 1D ECG signal array.
///   - `sampling_rate`: Sampling frequency $F_s$ in Hertz (Hz). Must be $> 0.0$.
/// - **Output**: Boolean mask of identical length where `true` indicates a detected R-peak location.
/// - **Methodology**: Derivative filtering $\rightarrow$ squaring $\rightarrow$ 150ms moving window integration $\rightarrow$ peak thresholding.
///
/// # Errors
/// Returns [`SignalError`] if `signal` is empty, contains non-finite samples, or if `sampling_rate` is invalid.
pub fn ecg_findpeaks(signal: &Array1<f64>, sampling_rate: f64) -> Result<Array1<bool>> {
    let n = signal.len();
    if n == 0 {
        return Err(SignalError::EmptySignal);
    }
    if !sampling_rate.is_finite() || sampling_rate <= 0.0 {
        return Err(SignalError::InvalidSamplingRate(sampling_rate));
    }

    // 1. Differentiate (first derivative)
    let mut diff = Array1::<f64>::zeros(n);
    for i in 1..(n - 1) {
        diff[i] = signal[i + 1] - signal[i - 1];
    }

    // 2. Square the signal
    let sq_diff = diff.mapv(|x| x * x);

    // 3. Moving average (approx 150ms window)
    let window_size = (0.150 * sampling_rate).max(1.0) as usize;
    let integrated = signal_smooth_moving_average(&sq_diff, window_size.max(1))?;

    // 4. Find local maxima
    let mut peaks = signal_findpeaks(&integrated)?;

    // 5. Apply thresholding (only keep peaks > 1.5 * mean of integrated signal)
    let mean_val = integrated.mean().unwrap_or(0.0);
    for i in 0..n {
        if peaks[i] && integrated[i] <= mean_val * 1.5 {
            peaks[i] = false;
        }
    }

    Ok(peaks)
}
