use crate::error::{Result, SignalError};
use ndarray::Array1;

/// Compute RMSSD (Root Mean Square of Successive Differences) from NN/RR intervals (in milliseconds).
///
/// # Scientific Contract
/// - **Inputs**: 1D array of inter-beat intervals in milliseconds ($\text{ms}$).
/// - **Output**: RMSSD metric in milliseconds ($\text{ms}$).
/// - **Formula**: $\text{RMSSD} = \sqrt{\frac{1}{N-1} \sum_{i=1}^{N-1} (RR_{i+1} - RR_i)^2}$.
///
/// # Errors
/// Returns [`SignalError`] if:
/// - `intervals` is empty ([`SignalError::EmptySignal`]).
/// - `intervals` has fewer than 2 elements ([`SignalError::InsufficientPeaks`]).
/// - `intervals` contains non-finite samples ([`SignalError::NonFiniteInput`]).
pub fn hrv_rmssd(intervals: &Array1<f64>) -> Result<f64> {
    let n = intervals.len();
    if n < 2 {
        return Err(SignalError::InsufficientPeaks {
            required: 2,
            provided: n,
        });
    }
    for &val in intervals.iter() {
        if !val.is_finite() {
            return Err(SignalError::NonFiniteInput);
        }
    }

    let mut sq_diff_sum = 0.0;
    for i in 0..(n - 1) {
        let diff = intervals[i + 1] - intervals[i];
        sq_diff_sum += diff * diff;
    }

    Ok((sq_diff_sum / (n - 1) as f64).sqrt())
}

/// Compute the Mean NN interval length (in milliseconds).
///
/// # Scientific Contract
/// - **Inputs**: 1D array of inter-beat intervals in milliseconds ($\text{ms}$).
/// - **Output**: Mean interval length in milliseconds ($\text{ms}$).
///
/// # Errors
/// Returns [`SignalError`] if:
/// - `intervals` has fewer than 1 element ([`SignalError::InsufficientPeaks`]).
/// - `intervals` contains non-finite samples ([`SignalError::NonFiniteInput`]).
pub fn hrv_mean_nn(intervals: &Array1<f64>) -> Result<f64> {
    let n = intervals.len();
    if n < 1 {
        return Err(SignalError::InsufficientPeaks {
            required: 1,
            provided: 0,
        });
    }
    for &val in intervals.iter() {
        if !val.is_finite() {
            return Err(SignalError::NonFiniteInput);
        }
    }

    intervals.mean().ok_or(SignalError::InsufficientPeaks {
        required: 1,
        provided: 0,
    })
}
