use crate::error::{Result, SignalError};
use crate::signal::filter::signal_filter;
use ndarray::Array1;

/// Clean a Respiration (RSP) signal using band-pass filtering (0.1 – 0.35 Hz, ~6–21 breaths/min).
///
/// # Scientific Contract
/// - **Inputs**:
///   - `signal`: Raw respiratory expansion/airflow signal samples.
///   - `sampling_rate`: Sampling frequency $F_s$ in Hertz (Hz). Must be $> 0.0$.
/// - **Output**: Cleaned 1D RSP signal of identical length.
///
/// # Errors
/// Returns [`SignalError`] if `signal` is empty, contains non-finite samples, or if `sampling_rate` is invalid.
pub fn rsp_clean(signal: &Array1<f64>, sampling_rate: f64) -> Result<Array1<f64>> {
    if signal.is_empty() {
        return Err(SignalError::EmptySignal);
    }
    signal_filter(signal, sampling_rate, Some(0.1), Some(0.35), 2)
}
