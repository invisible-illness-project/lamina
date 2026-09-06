use crate::error::{Result, SignalError};
use ndarray::Array1;

/// Compute Sample Entropy (SampEn) for a 1D signal.
///
/// # Scientific Contract
/// - **Inputs**:
///   - `signal`: 1D array of real-valued floating-point samples (`f64`).
///   - `m`: Embedding dimension ($m \ge 1$).
///   - `r`: Tolerance threshold ($r > 0.0$), typically $0.2 \times \text{std}(x)$.
/// - **Output**: Non-negative Sample Entropy value $h \ge 0.0$, or `f64::INFINITY` if zero template matches occur ($A = 0$ or $B = 0$).
/// - **Mathematical Definition**: $\text{SampEn}(m, r, N) = -\ln \frac{A}{B}$, where $B$ is the total count of template vectors of length $m$ matching within Chebyshev distance $r$, and $A$ is the count matching for length $m+1$.
///
/// # Errors
/// Returns [`SignalError`] if:
/// - `signal` is empty ([`SignalError::EmptySignal`]).
/// - `signal` length is $\le m+1$ ([`SignalError::InsufficientSamples`]).
/// - `signal` contains non-finite values ([`SignalError::NonFiniteInput`]).
/// - `r` is non-positive or non-finite ([`SignalError::InvalidCutoffFrequency`]).
pub fn sample_entropy(signal: &Array1<f64>, m: usize, r: f64) -> Result<f64> {
    let n = signal.len();
    if n == 0 {
        return Err(SignalError::EmptySignal);
    }
    if n <= m + 1 {
        return Err(SignalError::InsufficientSamples {
            required: m + 2,
            provided: n,
        });
    }
    if !r.is_finite() || r <= 0.0 {
        return Err(SignalError::InvalidCutoffFrequency(format!(
            "Tolerance threshold r ({}) must be > 0.0 and finite",
            r
        )));
    }
    for &val in signal.iter() {
        if !val.is_finite() {
            return Err(SignalError::NonFiniteInput);
        }
    }

    let mut count_m = 0_usize;
    let mut count_m1 = 0_usize;

    // Compare all pairs of templates of length m
    for i in 0..(n - m) {
        for j in (i + 1)..(n - m) {
            let mut match_m = true;
            for k in 0..m {
                if (signal[i + k] - signal[j + k]).abs() > r {
                    match_m = false;
                    break;
                }
            }

            if match_m {
                count_m += 1;
                // Additionally check if the subsequent elements also match (dimension m + 1)
                if i + m < n && j + m < n && (signal[i + m] - signal[j + m]).abs() <= r {
                    count_m1 += 1;
                }
            }
        }
    }

    if count_m == 0 || count_m1 == 0 {
        return Ok(f64::INFINITY);
    }

    Ok(-((count_m1 as f64) / (count_m as f64)).ln())
}
