use ndarray::Array1;
use crate::signal::filter::signal_filter;

/// Clean an EDA signal primarily by applying a low-pass filter to remove fast artifact spikes.
/// Standard practices recommend a 5Hz Butterworth filter.
pub fn eda_clean(signal: &Array1<f64>, sampling_rate: f64) -> Array1<f64> {
    signal_filter(signal, sampling_rate, None, Some(5.0), 4)
}
