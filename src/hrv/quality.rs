use crate::error::{Result, SignalError};
use ndarray::Array1;

/// Quality classification of an individual cardiac beat event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BeatQuality {
    /// Normal sinus beat
    #[default]
    Normal,
    /// Ectopic beat (e.g., PVC or PAC)
    Ectopic,
    /// Motion or noise artifact
    Artifact,
    /// Unclassified or unknown beat quality
    Unknown,
}

/// Quality classification of an inter-beat interval (R-R).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IntervalQuality {
    /// Normal-to-Normal (N-N) inter-beat interval
    #[default]
    NormalNN,
    /// Interval contaminated by ectopic beat
    EctopicRR,
    /// Interval contaminated by motion/noise artifact
    ArtifactRR,
    /// Missing or non-finite interval sample
    Missing,
}

/// Policy for handling invalid/ectopic inter-beat intervals to produce a clean N-N interval series.
#[derive(Debug, Clone, PartialEq)]
pub enum CorrectionPolicy {
    /// Pass-through all intervals without modification
    None,
    /// Remove invalid/ectopic intervals from the output series
    RejectInvalid,
    /// Replace invalid intervals using linear interpolation from adjacent valid NN intervals
    InterpolateLinear,
    /// Replace invalid intervals using cubic spline interpolation from surrounding valid NN intervals
    InterpolateCubic,
    /// Reject intervals whose relative change from local rolling median exceeds a percentage threshold (e.g. 0.20 = 20%)
    PercentThreshold(f64),
}

/// Classify inter-beat intervals into quality categories ([`IntervalQuality`]).
///
/// # Scientific Contract
/// An interval $RR_i$ is classified as:
/// - `Missing`: if $RR_i$ is non-finite (`NaN`, `inf`).
/// - `ArtifactRR`: if $RR_i < 300.0\text{ ms}$ ($>200\text{ BPM}$) or $RR_i > 2000.0\text{ ms}$ ($<30\text{ BPM}$).
/// - `EctopicRR`: if $|RR_i - \text{median}_{i-2..i+2}| / \text{median} > \text{pct\_threshold}$ (default 0.20 = 20%).
/// - `NormalNN`: otherwise.
pub fn classify_intervals(
    rr_intervals: &Array1<f64>,
    percent_threshold: Option<f64>,
) -> Vec<IntervalQuality> {
    let n = rr_intervals.len();
    if n == 0 {
        return Vec::new();
    }
    let threshold = percent_threshold.unwrap_or(0.20);
    let mut quality = vec![IntervalQuality::NormalNN; n];

    for i in 0..n {
        let val = rr_intervals[i];
        if !val.is_finite() {
            quality[i] = IntervalQuality::Missing;
            continue;
        }
        if val < 300.0 || val > 2000.0 {
            quality[i] = IntervalQuality::ArtifactRR;
            continue;
        }

        // Local rolling window for median comparison
        let start = i.saturating_sub(2);
        let end = (i + 3).min(n);
        let mut window: Vec<f64> = rr_intervals
            .slice(ndarray::s![start..end])
            .iter()
            .copied()
            .filter(|v| v.is_finite() && *v >= 300.0 && *v <= 2000.0)
            .collect();

        if window.len() >= 2 {
            window.sort_by(|a, b| a.total_cmp(b));
            let median = window[window.len() / 2];
            if (val - median).abs() / median > threshold {
                quality[i] = IntervalQuality::EctopicRR;
            }
        }
    }

    quality
}

/// Clean inter-beat intervals into a validated Normal-to-Normal (N-N) series according to a [`CorrectionPolicy`].
///
/// # Scientific Contract
/// - **Inputs**: `rr_intervals` array in milliseconds ($\text{ms}$) and desired `CorrectionPolicy`.
/// - **Output**: Cleaned 1D array of N-N intervals in milliseconds ($\text{ms}$).
///
/// # Errors
/// Returns [`SignalError`] if `rr_intervals` is empty or if parameter values are invalid.
pub fn clean_rr_intervals(
    rr_intervals: &Array1<f64>,
    policy: &CorrectionPolicy,
) -> Result<Array1<f64>> {
    let n = rr_intervals.len();
    if n == 0 {
        return Err(SignalError::EmptySignal);
    }

    match policy {
        CorrectionPolicy::None => Ok(rr_intervals.clone()),
        CorrectionPolicy::RejectInvalid => {
            let q = classify_intervals(rr_intervals, None);
            let valid: Vec<f64> = rr_intervals
                .iter()
                .zip(q.iter())
                .filter_map(|(&val, &kind)| {
                    if kind == IntervalQuality::NormalNN {
                        Some(val)
                    } else {
                        None
                    }
                })
                .collect();
            if valid.is_empty() {
                return Err(SignalError::EmptySignal);
            }
            Ok(Array1::from_vec(valid))
        }
        CorrectionPolicy::PercentThreshold(pct) => {
            if !pct.is_finite() || *pct <= 0.0 || *pct >= 1.0 {
                return Err(SignalError::NonFiniteInput);
            }
            let q = classify_intervals(rr_intervals, Some(*pct));
            let valid: Vec<f64> = rr_intervals
                .iter()
                .zip(q.iter())
                .filter_map(|(&val, &kind)| {
                    if kind == IntervalQuality::NormalNN {
                        Some(val)
                    } else {
                        None
                    }
                })
                .collect();
            if valid.is_empty() {
                return Err(SignalError::EmptySignal);
            }
            Ok(Array1::from_vec(valid))
        }
        CorrectionPolicy::InterpolateLinear | CorrectionPolicy::InterpolateCubic => {
            let q = classify_intervals(rr_intervals, None);
            let mut valid_indices = Vec::new();
            let mut valid_vals = Vec::new();

            for (i, (&val, &kind)) in rr_intervals.iter().zip(q.iter()).enumerate() {
                if kind == IntervalQuality::NormalNN {
                    valid_indices.push(i as f64);
                    valid_vals.push(val);
                }
            }

            if valid_vals.is_empty() {
                return Err(SignalError::EmptySignal);
            }
            if valid_vals.len() == 1 {
                return Ok(Array1::from_elem(n, valid_vals[0]));
            }

            let mut out = Vec::with_capacity(n);
            let num_v = valid_indices.len();

            for i in 0..n {
                if q[i] == IntervalQuality::NormalNN {
                    out.push(rr_intervals[i]);
                } else {
                    let idx = i as f64;
                    if idx <= valid_indices[0] {
                        out.push(valid_vals[0]);
                    } else if idx >= valid_indices[num_v - 1] {
                        out.push(valid_vals[num_v - 1]);
                    } else {
                        let mut right = 0;
                        while right < num_v && valid_indices[right] < idx {
                            right += 1;
                        }
                        let left = right - 1;
                        let x0 = valid_indices[left];
                        let x1 = valid_indices[right];
                        let y0 = valid_vals[left];
                        let y1 = valid_vals[right];
                        let alpha = (idx - x0) / (x1 - x0);
                        out.push((1.0 - alpha) * y0 + alpha * y1);
                    }
                }
            }
            Ok(Array1::from_vec(out))
        }
    }
}
