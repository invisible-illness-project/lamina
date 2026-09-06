use crate::error::{Result, SignalError};
use crate::signal::peaks::signal_findpeaks;
use ndarray::Array1;

/// Locate breath peaks (inhalation maxima) in a cleaned Respiration (RSP) signal.
///
/// # Scientific Contract
/// - **Inputs**:
///   - `cleaned_signal`: Cleaned 1D RSP signal array.
/// - **Output**: Boolean mask of identical length where `true` indicates a detected breath peak.
///
/// # Errors
/// Returns [`SignalError`] if `cleaned_signal` is empty or contains non-finite samples.
pub fn rsp_findpeaks(cleaned_signal: &Array1<f64>) -> Result<Array1<bool>> {
    if cleaned_signal.is_empty() {
        return Err(SignalError::EmptySignal);
    }
    signal_findpeaks(cleaned_signal)
}
