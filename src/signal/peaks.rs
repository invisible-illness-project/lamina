use crate::error::{Result, SignalError};
use ndarray::Array1;

/// Locate local maxima in a 1D signal.
///
/// # Scientific Contract
/// - **Inputs**:
///   - `signal`: 1D array of real-valued floating-point samples (`f64`).
/// - **Output**: Boolean mask of length $N$, where `true` indicates a 3-point local peak ($x[i] > x[i-1] \land x[i] > x[i+1]$).
/// - **Boundary Behavior**: Endpoints ($i=0$ and $i=N-1$) are always evaluated as `false`.
/// - **Note**: This primitive evaluates strict 3-point local maxima. Support for minimum peak distance, height,
///   and prominence is planned via the `find_peaks` primitive integration.
///
/// # Errors
/// Returns [`SignalError`] if:
/// - `signal` is empty ([`SignalError::EmptySignal`]).
/// - `signal` contains non-finite values ([`SignalError::NonFiniteInput`]).
pub fn signal_findpeaks(signal: &Array1<f64>) -> Result<Array1<bool>> {
    let n = signal.len();
    if n == 0 {
        return Err(SignalError::EmptySignal);
    }
    for &val in signal.iter() {
        if !val.is_finite() {
            return Err(SignalError::NonFiniteInput);
        }
    }

    let mut peaks = Array1::<bool>::from_elem(n, false);
    if n < 3 {
        return Ok(peaks);
    }

    for i in 1..(n - 1) {
        if signal[i] > signal[i - 1] && signal[i] > signal[i + 1] {
            peaks[i] = true;
        }
    }

    Ok(peaks)
}
