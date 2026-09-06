use crate::error::{Result, SignalError};
use crate::signal::filter::{FilterSpec, signal_filtfilt};
use ndarray::Array1;

/// Configuration for EDA tonic/phasic signal decomposition.
#[derive(Debug, Clone, PartialEq)]
pub struct EdaDecompositionConfig {
    /// Tonic component low-pass cutoff frequency in Hz (default: 0.05 Hz).
    pub tonic_cutoff_hz: Option<f64>,
    /// Butterworth filter order for decomposition (default: 2).
    pub filter_order: Option<usize>,
}

impl Default for EdaDecompositionConfig {
    fn default() -> Self {
        Self {
            tonic_cutoff_hz: Some(0.05),
            filter_order: Some(2),
        }
    }
}

impl EdaDecompositionConfig {
    /// Create a new default EDA decomposition configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set tonic low-pass cutoff frequency in Hz.
    pub fn with_tonic_cutoff_hz(mut self, cutoff: f64) -> Self {
        self.tonic_cutoff_hz = Some(cutoff);
        self
    }

    /// Set Butterworth filter order.
    pub fn with_filter_order(mut self, order: usize) -> Self {
        self.filter_order = Some(order);
        self
    }

    /// Validate configuration parameters.
    pub fn validate(&self) -> Result<()> {
        if matches!(self.tonic_cutoff_hz, Some(c) if !c.is_finite() || c <= 0.0) {
            return Err(SignalError::InvalidCutoffFrequency(
                "Tonic cutoff frequency must be positive and finite".to_string(),
            ));
        }
        if matches!(self.filter_order, Some(0)) {
            return Err(SignalError::InvalidFilterOrder(0));
        }
        Ok(())
    }
}

/// Decomposed Tonic and Phasic components of an Electrodermal Activity (EDA) signal.
#[derive(Debug, Clone, PartialEq)]
pub struct EdaComponents {
    /// Slowly-varying baseline Skin Conductance Level (SCL) component.
    pub tonic: Array1<f64>,
    /// Fast transient Skin Conductance Response (SCR) component.
    pub phasic: Array1<f64>,
}

/// Decompose an EDA signal into Tonic (SCL) and Phasic (SCR) components.
///
/// # Scientific Contract
/// - **Inputs**:
///   - `signal`: Cleaned EDA signal array in microsiemens ($\mu\text{S}$).
///   - `sampling_rate`: Sampling frequency $F_s$ in Hertz (Hz). Must be $> 0.0$.
///   - `config`: [`EdaDecompositionConfig`] parameters.
/// - **Output**: [`EdaComponents`] containing `tonic` and `phasic` arrays of identical length $N$.
/// - **Reconstruction Invariant**: $\text{tonic}[n] + \text{phasic}[n] = \text{signal}[n]$ for all $n$.
/// - **Methodology**: Applies zero-phase 2nd-order low-pass Butterworth filtering at 0.05 Hz (`signal_filtfilt`) to extract `tonic`, and computes residual `phasic = signal - tonic`.
///
/// # Errors
/// Returns [`SignalError`] if `signal` is empty, contains non-finite values, if `sampling_rate` is invalid, or if cutoff exceeds Nyquist.
pub fn eda_decompose(
    signal: &Array1<f64>,
    sampling_rate: f64,
    config: &EdaDecompositionConfig,
) -> Result<EdaComponents> {
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

    let tonic_cutoff = config.tonic_cutoff_hz.unwrap_or(0.05);
    let filter_order = config.filter_order.unwrap_or(2);

    let nyquist = sampling_rate / 2.0;
    if tonic_cutoff >= nyquist {
        return Err(SignalError::InvalidCutoffFrequency(format!(
            "Tonic cutoff ({}) must be strictly less than Nyquist frequency ({})",
            tonic_cutoff, nyquist
        )));
    }

    if n < 3 * filter_order {
        return Err(SignalError::InsufficientSamples {
            required: 3 * filter_order,
            provided: n,
        });
    }

    let filter_spec = FilterSpec::lowpass(sampling_rate, tonic_cutoff, filter_order);
    let tonic = signal_filtfilt(signal, &filter_spec)?;
    let phasic = signal - &tonic;

    Ok(EdaComponents { tonic, phasic })
}

/// Extract the Phasic (Skin Conductance Response - SCR) component from an EDA signal using default decomposition.
///
/// Convenience entry point maintaining backward compatibility.
pub fn eda_phasic(signal: &Array1<f64>, sampling_rate: f64) -> Result<Array1<f64>> {
    let config = EdaDecompositionConfig::default();
    let components = eda_decompose(signal, sampling_rate, &config)?;
    Ok(components.phasic)
}
