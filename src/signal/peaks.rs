use crate::error::{Result, SignalError};
use find_peaks::PeakFinder;
use ndarray::Array1;

/// Configuration for generic peak detection algorithm.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PeakDetectionConfig {
    /// Absolute minimum peak height (amplitude).
    pub min_height: Option<f64>,
    /// Minimum horizontal distance between neighbor peaks in samples.
    pub min_distance: Option<usize>,
    /// Minimum peak prominence.
    pub min_prominence: Option<f64>,
    /// Minimum peak width in samples.
    pub min_width: Option<usize>,
    /// Minimum vertical threshold relative to immediate neighbors ($x[i] - \max(x[i-1], x[i+1]) \ge \text{threshold}$).
    pub threshold: Option<f64>,
}

impl PeakDetectionConfig {
    /// Create a default peak detection configuration with no constraints.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set minimum peak height.
    pub fn with_min_height(mut self, height: f64) -> Self {
        self.min_height = Some(height);
        self
    }

    /// Set minimum distance between peaks in samples.
    pub fn with_min_distance(mut self, distance: usize) -> Self {
        self.min_distance = Some(distance);
        self
    }

    /// Set minimum peak prominence.
    pub fn with_min_prominence(mut self, prominence: f64) -> Self {
        self.min_prominence = Some(prominence);
        self
    }

    /// Set minimum peak width in samples.
    pub fn with_min_width(mut self, width: usize) -> Self {
        self.min_width = Some(width);
        self
    }

    /// Set minimum threshold distance to immediate neighbors.
    pub fn with_threshold(mut self, threshold: f64) -> Self {
        self.threshold = Some(threshold);
        self
    }

    /// Validate configuration options.
    pub fn validate(&self) -> Result<()> {
        if let Some(h) = self.min_height {
            let invalid = !h.is_finite();
            if invalid {
                return Err(SignalError::NonFiniteInput);
            }
        }
        if let Some(d) = self.min_distance {
            let invalid = d == 0;
            if invalid {
                return Err(SignalError::InvalidWindowSize(0));
            }
        }
        if let Some(p) = self.min_prominence {
            let invalid = !p.is_finite() || p < 0.0;
            if invalid {
                return Err(SignalError::NonFiniteInput);
            }
        }
        if let Some(w) = self.min_width {
            let invalid = w == 0;
            if invalid {
                return Err(SignalError::InvalidWindowSize(0));
            }
        }
        if let Some(t) = self.threshold {
            let invalid = !t.is_finite();
            if invalid {
                return Err(SignalError::NonFiniteInput);
            }
        }
        Ok(())
    }
}

/// Locate peak sample indices in a 1D signal given a [`PeakDetectionConfig`].
///
/// # Scientific Contract
/// - **Inputs**:
///   - `signal`: 1D array of real-valued floating-point samples (`f64`).
///   - `config`: Configuration struct expressing optional constraints (height, distance, prominence, width, threshold).
/// - **Output**: `Vec<usize>` containing 0-indexed sample indices of detected peaks relative to input signal in ascending order.
/// - **Boundary Behavior**: Endpoints ($i=0$ and $i=N-1$) are excluded.
///
/// # Errors
/// Returns [`SignalError`] if:
/// - `signal` is empty ([`SignalError::EmptySignal`]).
/// - `signal` or configuration parameters contain non-finite values ([`SignalError::NonFiniteInput`]).
/// - `min_distance` or `min_width` is 0 ([`SignalError::InvalidWindowSize`]).
pub fn signal_findpeaks_config(
    signal: &Array1<f64>,
    config: &PeakDetectionConfig,
) -> Result<Vec<usize>> {
    let n = signal.len();
    if n == 0 {
        return Err(SignalError::EmptySignal);
    }
    for &val in signal.iter() {
        if !val.is_finite() {
            return Err(SignalError::NonFiniteInput);
        }
    }
    config.validate()?;

    if n < 3 {
        return Ok(Vec::new());
    }

    let vec_signal = signal.to_vec();
    let mut fp = PeakFinder::new(&vec_signal);

    if let Some(h) = config.min_height {
        fp.with_min_height(h);
    }
    if let Some(d) = config.min_distance {
        fp.with_min_distance(d);
    }
    if let Some(p) = config.min_prominence {
        fp.with_min_prominence(p);
    }

    let detected_peaks = fp.find_peaks();

    let mut result_indices: Vec<usize> = Vec::new();
    for peak in detected_peaks {
        let idx = (peak.position.start + peak.position.end - 1) / 2;

        if let Some(w) = config.min_width {
            let width = peak.position.end - peak.position.start;
            if width < w {
                continue;
            }
        }

        if let Some(thresh) = config.threshold {
            let min_diff = peak.left_diff.min(peak.right_diff);
            if min_diff < thresh {
                continue;
            }
        }
        result_indices.push(idx);
    }

    result_indices.sort_unstable();
    Ok(result_indices)
}

/// Locate peak mask (`Array1<bool>`) in a 1D signal given a [`PeakDetectionConfig`].
pub fn signal_findpeaks_mask(
    signal: &Array1<f64>,
    config: &PeakDetectionConfig,
) -> Result<Array1<bool>> {
    let indices = signal_findpeaks_config(signal, config)?;
    let mut mask = Array1::<bool>::from_elem(signal.len(), false);
    for idx in indices {
        mask[idx] = true;
    }
    Ok(mask)
}

/// Locate local maxima in a 1D signal using default configuration.
///
/// Convenience entry point maintaining backward compatibility.
pub fn signal_findpeaks(signal: &Array1<f64>) -> Result<Array1<bool>> {
    let config = PeakDetectionConfig::default();
    signal_findpeaks_mask(signal, &config)
}
