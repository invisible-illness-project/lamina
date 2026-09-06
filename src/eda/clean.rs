use crate::error::{Result, SignalError};
use crate::signal::filter::{FilterSpec, signal_filtfilt};
use ndarray::Array1;

/// Configuration for Electrodermal Activity (EDA/GSR) signal cleaning.
#[derive(Debug, Clone, PartialEq)]
pub struct EdaCleaningConfig {
    /// Low-pass cutoff frequency in Hz (default: Some(5.0 Hz)).
    pub lowpass_cutoff_hz: Option<f64>,
    /// Butterworth filter order (default: Some(4)).
    pub filter_order: Option<usize>,
    /// Bypasses low-pass filtering when requested cutoff exceeds or equals Nyquist (default: true).
    pub pass_through_if_nyquist_violated: bool,
}

impl Default for EdaCleaningConfig {
    fn default() -> Self {
        Self {
            lowpass_cutoff_hz: Some(5.0),
            filter_order: Some(4),
            pass_through_if_nyquist_violated: true,
        }
    }
}

impl EdaCleaningConfig {
    /// Create a new default EDA cleaning configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set low-pass cutoff frequency in Hz.
    pub fn with_lowpass_cutoff_hz(mut self, cutoff: f64) -> Self {
        self.lowpass_cutoff_hz = Some(cutoff);
        self
    }

    /// Set Butterworth filter order.
    pub fn with_filter_order(mut self, order: usize) -> Self {
        self.filter_order = Some(order);
        self
    }

    /// Set whether low-pass filtering is bypassed when cutoff >= Nyquist.
    pub fn with_pass_through_if_nyquist_violated(mut self, pass_through: bool) -> Self {
        self.pass_through_if_nyquist_violated = pass_through;
        self
    }

    /// Validate configuration parameters.
    pub fn validate(&self) -> Result<()> {
        if matches!(self.lowpass_cutoff_hz, Some(c) if !c.is_finite() || c <= 0.0) {
            return Err(SignalError::InvalidCutoffFrequency(
                "Lowpass cutoff frequency must be positive and finite".to_string(),
            ));
        }
        if matches!(self.filter_order, Some(0)) {
            return Err(SignalError::InvalidFilterOrder(0));
        }
        Ok(())
    }
}

/// Clean an Electrodermal Activity (EDA/GSR) signal using a zero-phase low-pass Butterworth filter given a [`EdaCleaningConfig`].
///
/// # Scientific Contract
/// - **Inputs**:
///   - `signal`: Raw EDA skin conductance signal array in microsiemens ($\mu\text{S}$).
///   - `sampling_rate`: Sampling frequency $F_s$ in Hertz (Hz). Must be $> 0.0$.
///   - `config`: [`EdaCleaningConfig`] parameters.
/// - **Output**: Cleaned EDA signal array of identical length $N$.
/// - **Nyquist Safety**: If `lowpass_cutoff_hz >= sampling_rate / 2.0` (e.g. 5.0 Hz cutoff on 4.0 Hz E4 wearable EDA) and `pass_through_if_nyquist_violated` is `true`, low-pass filtering is safely bypassed to preserve natural anti-aliasing without throwing validation errors.
///
/// # Errors
/// Returns [`SignalError`] if `signal` is empty, contains non-finite samples, if `sampling_rate` is invalid, or if cutoff exceeds Nyquist with pass-through disabled.
pub fn eda_clean_config(
    signal: &Array1<f64>,
    sampling_rate: f64,
    config: &EdaCleaningConfig,
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

    let cutoff = match config.lowpass_cutoff_hz {
        Some(c) => c,
        None => return Ok(signal.clone()),
    };

    let nyquist = sampling_rate / 2.0;
    if cutoff >= nyquist {
        if config.pass_through_if_nyquist_violated {
            return Ok(signal.clone());
        } else {
            return Err(SignalError::InvalidCutoffFrequency(format!(
                "Cutoff frequency ({}) must be strictly less than Nyquist frequency ({})",
                cutoff, nyquist
            )));
        }
    }

    let filter_order = config.filter_order.unwrap_or(4);
    if n < 3 * filter_order {
        return Err(SignalError::InsufficientSamples {
            required: 3 * filter_order,
            provided: n,
        });
    }

    let filter_spec = FilterSpec::lowpass(sampling_rate, cutoff, filter_order);
    signal_filtfilt(signal, &filter_spec)
}

/// Clean an Electrodermal Activity (EDA/GSR) signal using default [`EdaCleaningConfig`].
pub fn eda_clean(signal: &Array1<f64>, sampling_rate: f64) -> Result<Array1<f64>> {
    eda_clean_config(signal, sampling_rate, &EdaCleaningConfig::default())
}
