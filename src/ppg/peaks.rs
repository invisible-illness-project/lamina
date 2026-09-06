use crate::error::{Result, SignalError};
use crate::signal::peaks::signal_findpeaks;
use crate::signal::smooth::signal_smooth_moving_average;
use ndarray::Array1;

/// Locate systolic peaks in a Photoplethysmogram (PPG) signal.
///
/// # Scientific Contract
/// - **Inputs**:
///   - `signal`: 1D array of PPG pulse waveform samples.
///   - `sampling_rate`: Sampling frequency $F_s$ in Hertz (Hz). Must be $> 0.0$.
/// - **Output**: Boolean mask of identical length where `true` indicates a systolic peak.
///
/// # Errors
/// Returns [`SignalError`] if `signal` is empty, contains non-finite samples, or if `sampling_rate` is invalid.
pub fn ppg_findpeaks(signal: &Array1<f64>, sampling_rate: f64) -> Result<Array1<bool>> {
    let n = signal.len();
    if n == 0 {
        return Err(SignalError::EmptySignal);
    }
    if !sampling_rate.is_finite() || sampling_rate <= 0.0 {
        return Err(SignalError::InvalidSamplingRate(sampling_rate));
    }

    let window_size = (0.100 * sampling_rate).max(1.0) as usize;
    let smoothed = signal_smooth_moving_average(signal, window_size)?;

    let mut peaks = signal_findpeaks(&smoothed)?;

    let mean_val = smoothed.mean().unwrap_or(0.0);
    for i in 0..n {
        if peaks[i] && smoothed[i] <= mean_val {
            peaks[i] = false;
        }
    }

    Ok(peaks)
}
