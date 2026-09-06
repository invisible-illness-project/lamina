use crate::error::{Result, SignalError};
use crate::signal::filter::signal_filter;
use ndarray::Array1;

/// Clean an Electrodermal Activity (EDA/GSR) signal using a 5Hz low-pass Butterworth filter.
///
/// # Scientific Contract
/// - **Inputs**:
///   - `signal`: Raw EDA skin conductance signal in microsiemens ($\mu\text{S}$).
///   - `sampling_rate`: Sampling frequency $F_s$ in Hertz (Hz). Must be $> 0.0$.
/// - **Output**: Cleaned EDA signal of identical length.
///
/// # Errors
/// Returns [`SignalError`] if `signal` is empty, contains non-finite samples, or if `sampling_rate` is invalid.
pub fn eda_clean(signal: &Array1<f64>, sampling_rate: f64) -> Result<Array1<f64>> {
    if signal.is_empty() {
        return Err(SignalError::EmptySignal);
    }
    signal_filter(signal, sampling_rate, None, Some(5.0), 4)
}
