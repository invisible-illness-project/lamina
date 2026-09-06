use crate::error::{Result, SignalError};
use crate::signal::filter::signal_filter;
use ndarray::Array1;

/// Clean a Photoplethysmogram (PPG) signal using bandpass filtering (0.5 – 8.0 Hz).
///
/// # Scientific Contract
/// - **Inputs**:
///   - `signal`: 1D array of raw PPG pulse waveform samples.
///   - `sampling_rate`: Sampling frequency $F_s$ in Hertz (Hz). Must be $> 0.0$.
/// - **Output**: Cleaned 1D PPG signal of identical length.
///
/// # Errors
/// Returns [`SignalError`] if `signal` is empty, contains non-finite samples, or if `sampling_rate` is invalid.
pub fn ppg_clean(signal: &Array1<f64>, sampling_rate: f64) -> Result<Array1<f64>> {
    if signal.is_empty() {
        return Err(SignalError::EmptySignal);
    }
    // Pass-through bandpass filter (0.5 - 8.0 Hz)
    signal_filter(signal, sampling_rate, Some(0.5), Some(8.0), 3)
}
