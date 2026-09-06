use crate::error::{Result, SignalError};
use crate::signal::peaks::signal_findpeaks;
use ndarray::Array1;

/// Locate physiological Skin Conductance Response (SCR) peaks in a Phasic EDA signal.
///
/// # Scientific Contract
/// - **Inputs**:
///   - `phasic_signal`: Phasic component of an EDA signal.
/// - **Output**: Boolean mask of identical length where `true` indicates an SCR peak.
///
/// # Errors
/// Returns [`SignalError`] if `phasic_signal` is empty or contains non-finite samples.
pub fn eda_findpeaks(phasic_signal: &Array1<f64>) -> Result<Array1<bool>> {
    let n = phasic_signal.len();
    if n == 0 {
        return Err(SignalError::EmptySignal);
    }
    let mut peaks = signal_findpeaks(phasic_signal)?;

    for i in 0..n {
        if peaks[i] && phasic_signal[i] < 0.0 {
            peaks[i] = false;
        }
    }

    Ok(peaks)
}
