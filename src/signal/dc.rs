use crate::error::{Result, SignalError};
use ndarray::Array1;

/// Remove DC (non-pulsatile) baseline component from a 1D signal or window segment via mean subtraction.
///
/// # Mathematical Contract
/// $$y_i = x_i - \bar{x}, \quad \text{where } \bar{x} = \frac{1}{M}\sum_{i=1}^M x_i$$
/// Invariant: Output array has mean strictly zero ($|\text{mean}(y)| < 10^{-12}$).
///
/// # Errors
/// Returns [`SignalError::EmptySignal`] if `signal` is empty.
/// Returns [`SignalError::NonFiniteInput`] if `signal` contains non-finite samples (NaN/Inf).
pub fn signal_remove_dc(signal: &Array1<f64>) -> Result<Array1<f64>> {
    let n = signal.len();
    if n == 0 {
        return Err(SignalError::EmptySignal);
    }
    let mut sum = 0.0;
    for &v in signal.iter() {
        if !v.is_finite() {
            return Err(SignalError::NonFiniteInput);
        }
        sum += v;
    }

    let mean = sum / n as f64;
    let mut out = signal.to_owned();
    for v in out.iter_mut() {
        *v -= mean;
    }

    Ok(out)
}
