use crate::error::{Result, SignalError};
use ndarray::Array1;

/// Smooth a 1D signal using a centered rolling moving average.
///
/// # Scientific Contract
/// - **Inputs**:
///   - `signal`: 1D array of real-valued floating-point samples (`f64`).
///   - `window_size`: Width of moving average window $W > 0$ in samples.
/// - **Output**: Smoothed 1D signal of identical length $N$.
/// - **Boundary Behavior**: Uses dynamic window truncations at boundaries ($start = \max(0, i - \lfloor W/2 \rfloor)$,
///   $end = \min(N, i + \lfloor W/2 \rfloor + 1)$) normalized by the actual number of samples within the boundary slice.
/// - **Time Complexity**: $O(N)$ time and memory using prefix sum arrays (cumulative sum).
///
/// # Errors
/// Returns [`SignalError`] if:
/// - `signal` is empty ([`SignalError::EmptySignal`]).
/// - `window_size` is 0 ([`SignalError::InvalidWindowSize`]).
/// - `signal` contains non-finite values ([`SignalError::NonFiniteInput`]).
pub fn signal_smooth_moving_average(
    signal: &Array1<f64>,
    window_size: usize,
) -> Result<Array1<f64>> {
    let n = signal.len();
    if n == 0 {
        return Err(SignalError::EmptySignal);
    }
    if window_size == 0 {
        return Err(SignalError::InvalidWindowSize(window_size));
    }
    for &val in signal.iter() {
        if !val.is_finite() {
            return Err(SignalError::NonFiniteInput);
        }
    }

    if window_size == 1 {
        return Ok(signal.clone());
    }

    let mut prefix_sum = vec![0.0; n + 1];
    for i in 0..n {
        prefix_sum[i + 1] = prefix_sum[i] + signal[i];
    }

    let mut smoothed = Array1::<f64>::zeros(n);
    let half_win = window_size / 2;

    for i in 0..n {
        let start = i.saturating_sub(half_win);
        let end = (i + half_win + 1).min(n);
        let count = (end - start) as f64;
        let sum = prefix_sum[end] - prefix_sum[start];
        smoothed[i] = sum / count;
    }

    Ok(smoothed)
}
