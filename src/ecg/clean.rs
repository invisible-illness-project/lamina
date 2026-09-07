use crate::error::{Result, SignalError};
use crate::signal::filter::signal_filter;
use ndarray::Array1;

/// Clean an Electrocardiogram (ECG) signal.
///
/// # Scientific Contract
/// - **Inputs**:
///   - `signal`: 1D array of raw ECG amplitude measurements in millivolts ($\text{mV}$) or arbitrary units.
///   - `sampling_rate`: Sampling frequency $F_s$ in Hertz (Hz). Must be $> 0.0$.
///   - `method`: Cleaning strategy string identifier (supported: `"neurokit"`, `"pantompkins"`, `"biosppy"`).
/// - **Output**: Cleaned 1D ECG signal of identical length.
/// - **Methodology**: Applies high-pass Butterworth filtering at 0.5 Hz to eliminate baseline wander artifacts.
///
/// # Errors
/// Returns [`SignalError`] if `signal` is empty, contains non-finite samples, if `sampling_rate` is invalid, or if `method` is unsupported.
pub fn ecg_clean(signal: &Array1<f64>, sampling_rate: f64, method: &str) -> Result<Array1<f64>> {
    if signal.is_empty() {
        return Err(SignalError::EmptySignal);
    }

    let norm_method = method.trim().to_lowercase();
    match norm_method.as_str() {
        "none" | "raw" | "passthrough" => Ok(signal.clone()),
        "" | "neurokit" | "pantompkins" | "biosppy" => {
            // High-pass filter at 0.5 Hz to remove baseline wander
            signal_filter(signal, sampling_rate, Some(0.5), None, 5)
        }
        _ => Err(SignalError::InvalidCutoffFrequency(format!(
            "Unsupported ECG cleaning method: {}",
            method
        ))),
    }
}
