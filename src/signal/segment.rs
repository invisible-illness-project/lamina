use crate::error::{Result, SignalError};
use ndarray::Array1;

/// Policy for handling trailing incomplete window segments during signal segmentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IncompleteTailPolicy {
    /// Ignore trailing samples if remaining < window_samples (PulseLM default).
    DropIncomplete,
    /// Zero-pad trailing segment to length window_samples.
    PadZeros,
    /// Keep trailing segment as a shorter window.
    KeepPartial,
}

/// Partition a 1D signal into fixed-length window segments by sample count.
///
/// # Scientific Contract
/// - **Inputs**:
///   - `signal`: 1D input array `Array1<f64>`.
///   - `window_samples`: Window size in sample counts ($W > 0$).
///   - `step_samples`: Stride between window starts ($S > 0$).
///   - `tail_policy`: Policy for trailing samples when $N \pmod S \ne 0$.
/// - **Output**: `Vec<Array1<f64>>` containing partitioned window arrays.
///
/// # Errors
/// Returns [`SignalError::InvalidWindowSize`] if `window_samples == 0` or `step_samples == 0`.
/// Returns [`SignalError::EmptySignal`] if `signal` is empty.
/// Returns [`SignalError::NonFiniteInput`] if `signal` contains non-finite samples.
pub fn signal_segment(
    signal: &Array1<f64>,
    window_samples: usize,
    step_samples: usize,
    tail_policy: IncompleteTailPolicy,
) -> Result<Vec<Array1<f64>>> {
    if window_samples == 0 {
        return Err(SignalError::InvalidWindowSize(0));
    }
    if step_samples == 0 {
        return Err(SignalError::InvalidWindowSize(0));
    }
    let n = signal.len();
    if n == 0 {
        return Err(SignalError::EmptySignal);
    }
    for &v in signal.iter() {
        if !v.is_finite() {
            return Err(SignalError::NonFiniteInput);
        }
    }

    let mut segments = Vec::new();
    let mut start = 0;

    while start < n {
        let end = (start + window_samples).min(n);
        let seg_len = end - start;

        if seg_len == window_samples {
            segments.push(signal.slice(ndarray::s![start..end]).to_owned());
        } else {
            match tail_policy {
                IncompleteTailPolicy::DropIncomplete => {
                    break;
                }
                IncompleteTailPolicy::PadZeros => {
                    let mut padded = vec![0.0; window_samples];
                    for (i, &val) in signal.slice(ndarray::s![start..end]).iter().enumerate() {
                        padded[i] = val;
                    }
                    segments.push(Array1::from_vec(padded));
                }
                IncompleteTailPolicy::KeepPartial => {
                    segments.push(signal.slice(ndarray::s![start..end]).to_owned());
                }
            }
        }

        start += step_samples;
    }

    Ok(segments)
}

/// Partition a 1D signal into fixed-length window segments by physical duration (seconds).
pub fn signal_segment_duration(
    signal: &Array1<f64>,
    sampling_rate: f64,
    window_sec: f64,
    stride_sec: f64,
    tail_policy: IncompleteTailPolicy,
) -> Result<Vec<Array1<f64>>> {
    if !sampling_rate.is_finite() || sampling_rate <= 0.0 {
        return Err(SignalError::InvalidSamplingRate(sampling_rate));
    }
    if !window_sec.is_finite() || window_sec <= 0.0 {
        return Err(SignalError::InvalidWindowSize(0));
    }
    if !stride_sec.is_finite() || stride_sec <= 0.0 {
        return Err(SignalError::InvalidWindowSize(0));
    }

    let window_samples = (window_sec * sampling_rate).round() as usize;
    let step_samples = (stride_sec * sampling_rate).round() as usize;

    signal_segment(signal, window_samples, step_samples, tail_policy)
}
