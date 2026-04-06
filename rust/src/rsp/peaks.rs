use ndarray::Array1;
use crate::signal::peaks::signal_findpeaks;

/// Locate exhalation peaks mapping individual breaths in a cleaned RSP signal.
pub fn rsp_findpeaks(cleaned_signal: &Array1<f64>) -> Array1<bool> {
    // Simple maxima retrieval for cleaned respiratory sine waves
    signal_findpeaks(cleaned_signal)
}
