use ndarray::Array1;
use crate::signal::filter::signal_filter;

/// Clean an ECG signal using a high-pass filter to remove baseline wander,
/// and a powerline filter to remove 50Hz/60Hz noise.
/// 
/// Method currently acts as a placeholder for full zero-phase digital filtering capabilities.
pub fn ecg_clean(
    signal: &Array1<f64>,
    sampling_rate: f64,
    _method: &str,
) -> Array1<f64> {
    // Typical NeuroKit behavior is to run a high-pass filter at 0.5 Hz
    // and a 50Hz powerline filter.
    // For scaffolding, we pipe this to the generic `signal_filter`.
    signal_filter(signal, sampling_rate, Some(0.5), None, 5)
}
