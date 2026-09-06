use crate::error::{Result, SignalError};
use crate::signal::filter::{FilterSpec, signal_filtfilt};
use ndarray::Array1;

/// Configuration for Respiration (RSP) signal pre-cleaning.
#[derive(Debug, Clone, PartialEq)]
pub struct RspCleaningConfig {
    /// Bandpass lower cutoff frequency in Hz (default: 0.05 Hz, ~3 breaths/min).
    pub lowcut: Option<f64>,
    /// Bandpass upper cutoff frequency in Hz (default: 0.50 Hz, ~30 breaths/min).
    pub highcut: Option<f64>,
    /// Butterworth filter order (default: 3).
    pub filter_order: Option<usize>,
}

impl Default for RspCleaningConfig {
    fn default() -> Self {
        Self {
            lowcut: Some(0.05),
            highcut: Some(0.50),
            filter_order: Some(3),
        }
    }
}

impl RspCleaningConfig {
    /// Create a new default RSP cleaning configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set lower cutoff frequency in Hz.
    pub fn with_lowcut(mut self, lowcut: f64) -> Self {
        self.lowcut = Some(lowcut);
        self
    }

    /// Set upper cutoff frequency in Hz.
    pub fn with_highcut(mut self, highcut: f64) -> Self {
        self.highcut = Some(highcut);
        self
    }

    /// Set Butterworth filter order.
    pub fn with_filter_order(mut self, order: usize) -> Self {
        self.filter_order = Some(order);
        self
    }

    /// Validate configuration parameters.
    pub fn validate(&self) -> Result<()> {
        if matches!(self.lowcut, Some(lc) if !lc.is_finite() || lc <= 0.0) {
            return Err(SignalError::InvalidCutoffFrequency(
                "Lowcut frequency must be positive and finite".to_string(),
            ));
        }
        if matches!(self.highcut, Some(hc) if !hc.is_finite() || hc <= 0.0) {
            return Err(SignalError::InvalidCutoffFrequency(
                "Highcut frequency must be positive and finite".to_string(),
            ));
        }
        if matches!((self.lowcut, self.highcut), (Some(lc), Some(hc)) if lc >= hc) {
            return Err(SignalError::InvalidCutoffFrequency(
                "Lowcut frequency must be strictly less than highcut frequency".to_string(),
            ));
        }
        if matches!(self.filter_order, Some(0)) {
            return Err(SignalError::InvalidFilterOrder(0));
        }
        Ok(())
    }
}

/// Clean a Respiration (RSP) signal using zero-phase band-pass Butterworth filtering with custom config.
///
/// # Scientific Contract
/// - **Inputs**:
///   - `signal`: Raw respiratory airflow or expansion signal array.
///   - `sampling_rate`: Sampling frequency $F_s$ in Hertz (Hz). Must be $> 0.0$.
///   - `config`: [`RspCleaningConfig`] parameters.
/// - **Output**: Cleaned 1D RSP signal array of identical length $N$.
/// - **Methodology**: Applies zero-phase 3rd-order Butterworth bandpass filtering (0.05–0.50 Hz) via `signal_filtfilt` to isolate physiological breathing expansion waves.
///
/// # Errors
/// Returns [`SignalError`] if `signal` is empty, contains non-finite samples, if `sampling_rate` is invalid, or if parameters exceed Nyquist bounds.
pub fn rsp_clean_config(
    signal: &Array1<f64>,
    sampling_rate: f64,
    config: &RspCleaningConfig,
) -> Result<Array1<f64>> {
    let n = signal.len();
    if n == 0 {
        return Err(SignalError::EmptySignal);
    }
    if !sampling_rate.is_finite() || sampling_rate <= 0.0 {
        return Err(SignalError::InvalidSamplingRate(sampling_rate));
    }
    for &val in signal.iter() {
        if !val.is_finite() {
            return Err(SignalError::NonFiniteInput);
        }
    }
    config.validate()?;

    let lowcut = config.lowcut.unwrap_or(0.05);
    let highcut = config.highcut.unwrap_or(0.50);
    let filter_order = config.filter_order.unwrap_or(3);

    let nyquist = sampling_rate / 2.0;
    if lowcut >= highcut || highcut >= nyquist {
        return Err(SignalError::InvalidCutoffFrequency(format!(
            "Cutoff frequencies ({}, {}) must satisfy 0 < lowcut < highcut < Nyquist ({})",
            lowcut, highcut, nyquist
        )));
    }

    if n < 3 * filter_order {
        return Err(SignalError::InsufficientSamples {
            required: 3 * filter_order,
            provided: n,
        });
    }

    let filter_spec = FilterSpec::bandpass(sampling_rate, lowcut, highcut, filter_order);
    signal_filtfilt(signal, &filter_spec)
}

/// Clean a Respiration (RSP) signal using default band-pass filtering (0.05 – 0.50 Hz).
///
/// Convenience entry point maintaining backward compatibility.
pub fn rsp_clean(signal: &Array1<f64>, sampling_rate: f64) -> Result<Array1<f64>> {
    let config = RspCleaningConfig::default();
    rsp_clean_config(signal, sampling_rate, &config)
}
