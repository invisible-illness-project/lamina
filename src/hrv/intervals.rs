use crate::error::{Result, SignalError};
use ndarray::Array1;

/// Convert a boolean peak detection array into peak-to-peak (R-R or N-N) intervals in milliseconds (ms).
///
/// # Scientific Contract
/// - **Inputs**:
///   - `peaks`: Boolean mask where `true` indicates a detected peak index.
///   - `sampling_rate`: Sampling frequency $F_s$ in Hertz (Hz). Must be $> 0.0$.
/// - **Output**: 1D array of intervals in milliseconds ($\text{ms}$) of length $K-1$, where $K$ is the number of detected peaks.
/// - **Formula**: $\Delta t_i = \frac{p_{i+1} - p_i}{F_s} \times 1000.0$.
///
/// # Errors
/// Returns [`SignalError`] if:
/// - `peaks` is empty ([`SignalError::EmptySignal`]).
/// - `sampling_rate` is invalid ([`SignalError::InvalidSamplingRate`]).
/// - Fewer than 2 peaks are detected ([`SignalError::InsufficientPeaks`]).
pub fn peaks_to_intervals(peaks: &Array1<bool>, sampling_rate: f64) -> Result<Array1<f64>> {
    if peaks.is_empty() {
        return Err(SignalError::EmptySignal);
    }
    if !sampling_rate.is_finite() || sampling_rate <= 0.0 {
        return Err(SignalError::InvalidSamplingRate(sampling_rate));
    }

    let peak_indices: Vec<usize> = peaks
        .iter()
        .enumerate()
        .filter_map(|(i, &p)| if p { Some(i) } else { None })
        .collect();

    if peak_indices.len() < 2 {
        return Err(SignalError::InsufficientPeaks {
            required: 2,
            provided: peak_indices.len(),
        });
    }

    let mut intervals = Array1::<f64>::zeros(peak_indices.len() - 1);
    for i in 0..(peak_indices.len() - 1) {
        let diff_samples = peak_indices[i + 1] - peak_indices[i];
        intervals[i] = (diff_samples as f64 / sampling_rate) * 1000.0;
    }

    Ok(intervals)
}
