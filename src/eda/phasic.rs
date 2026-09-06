use crate::error::{Result, SignalError};
use crate::signal::filter::signal_filter;
use ndarray::Array1;

/// Extract the Phasic (Skin Conductance Response - SCR) component from an EDA signal using a 0.05 Hz high-pass filter.
///
/// # Scientific Contract
/// - **Inputs**:
///   - `signal`: Cleaned EDA skin conductance signal in microsiemens ($\mu\text{S}$).
///   - `sampling_rate`: Sampling frequency $F_s$ in Hertz (Hz). Must be $> 0.0$.
/// - **Output**: Isolated Phasic SCR signal of identical length.
///
/// # Errors
/// Returns [`SignalError`] if `signal` is empty, contains non-finite samples, or if `sampling_rate` is invalid.
pub fn eda_phasic(signal: &Array1<f64>, sampling_rate: f64) -> Result<Array1<f64>> {
    if signal.is_empty() {
        return Err(SignalError::EmptySignal);
    }
    signal_filter(signal, sampling_rate, Some(0.05), None, 2)
}
