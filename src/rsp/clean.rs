use ndarray::Array1;
use crate::signal::filter::signal_filter;

/// Clean a Respiration (RSP) signal using a band-pass filtering range.
/// The typical human respiration lies between 6 breaths/min (0.1Hz) and 21 breaths/min (0.35Hz).
pub fn rsp_clean(signal: &Array1<f64>, sampling_rate: f64) -> Array1<f64> {
    signal_filter(signal, sampling_rate, Some(0.1), Some(0.35), 2)
}
