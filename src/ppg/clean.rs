use ndarray::Array1;
use crate::signal::filter::signal_filter;

/// Clean a PPG signal using a bandpass filter (e.g., 0.5 - 8.0 Hz).
/// 
/// Method currently acts as a placeholder relying on `signal_filter`.
pub fn ppg_clean(
    signal: &Array1<f64>,
    sampling_rate: f64,
) -> Array1<f64> {
    // Pass-through to basic signal filter
    signal_filter(signal, sampling_rate, Some(0.5), Some(8.0), 3)
}
