use crate::error::{Result, SignalError};
use ndarray::Array1;
use std::f64::consts::PI;

/// Modified Bessel function of the zero-th order $I_0(z)$.
pub fn i0(z: f64) -> f64 {
    let mut sum = 1.0;
    let mut term = 1.0;
    let z_sq_4 = (z * z) / 4.0;
    for k in 1..60 {
        term *= z_sq_4 / (k as f64 * k as f64);
        sum += term;
        if term < 1e-16 * sum {
            break;
        }
    }
    sum
}

/// Compute greatest common divisor of two positive integers.
fn gcd(mut a: usize, mut b: usize) -> usize {
    while b != 0 {
        let temp = b;
        b = a % b;
        a = temp;
    }
    a
}

/// Polyphase rational resampling (`signal_resample_poly`) matching SciPy `scipy.signal.resample_poly`.
///
/// # Scientific Contract
/// - **Inputs**:
///   - `signal`: 1D array of input samples `Array1<f64>`.
///   - `up`: Upsampling integer factor $P \ge 1$.
///   - `down`: Downsampling integer factor $Q \ge 1$.
/// - **Output**: Resampled signal of length $\lceil N \cdot P / Q \rceil$.
///
/// # Canonical Specification
/// - Algorithm: Polyphase FIR filter bank with Kaiser windowed sinc anti-aliasing filter.
/// - Kaiser Parameter: $\beta = 5.0$.
/// - FIR Half-Length Factor: $10 \rightarrow N_{\text{taps}} = 2 \cdot 10 \cdot \max(P, Q) + 1$.
/// - Cutoff Frequency: $f_c = \frac{1}{\max(P, Q)}$ relative to Nyquist at upsampled rate.
/// - Boundary Mode: Constant zero boundary extension.
///
/// # Errors
/// Returns [`SignalError::InvalidSamplingRate`] if `up == 0` or `down == 0`.
/// Returns [`SignalError::EmptySignal`] if `signal` is empty.
/// Returns [`SignalError::NonFiniteInput`] if `signal` contains non-finite numbers (NaN/Inf).
pub fn signal_resample_poly(
    signal: &Array1<f64>,
    up: usize,
    down: usize,
) -> Result<Array1<f64>> {
    if up == 0 || down == 0 {
        return Err(SignalError::InvalidSamplingRate(0.0));
    }
    let n_in = signal.len();
    if n_in == 0 {
        return Err(SignalError::EmptySignal);
    }
    for &v in signal.iter() {
        if !v.is_finite() {
            return Err(SignalError::NonFiniteInput);
        }
    }

    let g = gcd(up, down);
    let p = up / g;
    let q = down / g;

    if p == 1 && q == 1 {
        return Ok(signal.clone());
    }

    let max_rate = p.max(q);
    let fc = 1.0 / max_rate as f64;
    let half_len = 10 * max_rate;
    let numtaps = 2 * half_len + 1;

    // Design Kaiser-windowed sinc FIR filter
    let m_center = (numtaps - 1) as f64 / 2.0;
    let beta = 5.0;
    let i0_b = i0(beta);
    let mut h = vec![0.0; numtaps];
    let mut h_sum = 0.0;

    for i in 0..numtaps {
        let u = (2.0 * i as f64 / (numtaps - 1) as f64) - 1.0;
        let w = i0(beta * (1.0 - u * u).max(0.0).sqrt()) / i0_b;
        let sinc = if (i as f64 - m_center).abs() < 1e-12 {
            fc
        } else {
            let x_c = PI * (i as f64 - m_center) * fc;
            fc * x_c.sin() / x_c
        };
        let val = sinc * w;
        h[i] = val;
        h_sum += val;
    }

    // Scale FIR filter gain by P
    let scale = p as f64 / h_sum;
    for val in h.iter_mut() {
        *val *= scale;
    }

    // Pre-pad filter array to align output phase matching SciPy
    let n_pre_pad = (q - (half_len % q)) % q;
    let mut h_padded = vec![0.0; n_pre_pad];
    h_padded.extend_from_slice(&h);

    let n_out = (n_in * p + q - 1) / q; // ceil(n_in * p / q)
    let n_pre_remove = (half_len + n_pre_pad) / q;
    let l_h = h_padded.len();

    let mut out = Vec::with_capacity(n_out);

    for j in 0..n_out {
        let j_eff = j + n_pre_remove;
        let m = j_eff * q;
        let phase = m % p;
        let i0_idx = m / p;
        let mut val = 0.0;
        let mut r = 0;
        loop {
            let k = r * p + phase;
            if k >= l_h {
                break;
            }
            if i0_idx >= r {
                let in_idx = i0_idx - r;
                if in_idx < n_in {
                    val += h_padded[k] * signal[in_idx];
                }
            }
            r += 1;
        }
        out.push(val);
    }

    Ok(Array1::from_vec(out))
}

/// Convenience rate-based resampling entry point `signal_resample(signal, orig_fs, target_fs)`.
pub fn signal_resample(
    signal: &Array1<f64>,
    orig_fs: f64,
    target_fs: f64,
) -> Result<Array1<f64>> {
    if !orig_fs.is_finite() || orig_fs <= 0.0 {
        return Err(SignalError::InvalidSamplingRate(orig_fs));
    }
    if !target_fs.is_finite() || target_fs <= 0.0 {
        return Err(SignalError::InvalidSamplingRate(target_fs));
    }

    // Convert floating point sampling rates into rational integer ratio up/down
    let mult = 1000.0; // scale to 3 decimal places precision for rates like 125.0, 250.0, 360.0
    let up = (target_fs * mult).round() as usize;
    let down = (orig_fs * mult).round() as usize;

    signal_resample_poly(signal, up, down)
}
