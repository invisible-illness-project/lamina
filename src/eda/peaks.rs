use ndarray::Array1;
use crate::signal::peaks::signal_findpeaks;

/// Locate physiological Skin Conductance Response (SCR) peaks.
/// Generally applied to the Phasic portion of the EDA signal.
pub fn eda_findpeaks(phasic_signal: &Array1<f64>) -> Array1<bool> {
    let mut peaks = signal_findpeaks(phasic_signal);
    
    // Eliminate negative false-peaks (SCR must be a positive conductance burst)
    let n = peaks.len();
    for i in 0..n {
        if peaks[i] && phasic_signal[i] < 0.0 {
            peaks[i] = false;
        }
    }
    
    peaks
}
