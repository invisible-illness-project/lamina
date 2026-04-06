use ndarray::Array1;
use crate::signal::filter::signal_filter;

/// Extract the Phasic (Skin Conductance Response - SCR) element of the EDA signal.
/// Uses a simplified 0.05 Hz high-pass mechanism to eliminate the slow-moving Tonic baseline.
pub fn eda_phasic(signal: &Array1<f64>, sampling_rate: f64) -> Array1<f64> {
    signal_filter(signal, sampling_rate, Some(0.05), None, 2)
}
