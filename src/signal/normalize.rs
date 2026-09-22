use crate::error::{Result, SignalError};
use ndarray::Array1;

/// Degenerate policy enum for handling flat or zero-variance signals ($\Delta = x_{\text{max}} - x_{\text{min}} < 10^{-12}$).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DegeneratePolicy {
    /// Return [`SignalError::DegenerateSignal`] error on zero-variance signal.
    Error,
    /// Output zero array ($0.0^M$).
    Zero,
    /// Output array filled with range midpoint $(a + b) / 2$ (0.5 for $[0.0, 1.0]$ range).
    Midpoint,
}

/// Min-Max scaling of a 1D signal into target range $[a, b]$ (default $[0.0, 1.0]$).
///
/// # Mathematical Contract
/// $$y_i = a + \frac{x_i - x_{\text{min}}}{x_{\text{max}} - x_{\text{min}}} (b - a)$$
/// Invariant: All output values satisfy $a \le y_i \le b$.
///
/// # Errors
/// Returns [`SignalError::EmptySignal`] if `signal` is empty.
/// Returns [`SignalError::NonFiniteInput`] if `signal` contains non-finite samples (NaN/Inf).
/// Returns [`SignalError::DegenerateSignal`] if signal is flat and policy is [`DegeneratePolicy::Error`].
pub fn signal_minmax(
    signal: &Array1<f64>,
    range: (f64, f64),
    policy: DegeneratePolicy,
) -> Result<Array1<f64>> {
    let n = signal.len();
    if n == 0 {
        return Err(SignalError::EmptySignal);
    }
    let (a, b) = range;
    if !a.is_finite() || !b.is_finite() || a >= b {
        return Err(SignalError::InvalidWindowSize(0));
    }

    let mut min_v = f64::MAX;
    let mut max_v = f64::MIN;

    for &v in signal.iter() {
        if !v.is_finite() {
            return Err(SignalError::NonFiniteInput);
        }
        if v < min_v {
            min_v = v;
        }
        if v > max_v {
            max_v = v;
        }
    }

    let diff = max_v - min_v;

    if diff < 1e-12 {
        match policy {
            DegeneratePolicy::Error => return Err(SignalError::DegenerateSignal),
            DegeneratePolicy::Zero => return Ok(Array1::zeros(n)),
            DegeneratePolicy::Midpoint => {
                let mid = (a + b) / 2.0;
                return Ok(Array1::from_elem(n, mid));
            }
        }
    }

    let span = b - a;
    let mut out = signal.to_owned();
    for v in out.iter_mut() {
        *v = a + ((*v - min_v) / diff) * span;
    }

    Ok(out)
}
