use std::fmt;

/// Primary error type for Lamina physiological signal processing operations.
#[derive(Debug, Clone, PartialEq)]
pub enum SignalError {
    /// Signal array is empty (0 samples).
    EmptySignal,
    /// Sampling rate is invalid (must be > 0.0 and finite).
    InvalidSamplingRate(f64),
    /// Cutoff frequency is invalid (must be > 0.0, less than Nyquist, and lowcut < highcut).
    InvalidCutoffFrequency(String),
    /// Signal length is shorter than required minimum samples for the algorithm.
    InsufficientSamples { required: usize, provided: usize },
    /// Window size is invalid (e.g. 0).
    InvalidWindowSize(usize),
    /// Filter order is invalid (e.g. 0).
    InvalidFilterOrder(usize),
    /// Input signal contains non-finite values (NaN or Infinity).
    NonFiniteInput,
    /// Insufficient detected peaks or intervals to compute the requested metric.
    InsufficientPeaks { required: usize, provided: usize },
}

impl fmt::Display for SignalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SignalError::EmptySignal => write!(f, "Signal array is empty"),
            SignalError::InvalidSamplingRate(rate) => {
                write!(
                    f,
                    "Invalid sampling rate: {} Hz (must be > 0.0 and finite)",
                    rate
                )
            }
            SignalError::InvalidCutoffFrequency(msg) => {
                write!(f, "Invalid cutoff frequency: {}", msg)
            }
            SignalError::InsufficientSamples { required, provided } => {
                write!(
                    f,
                    "Insufficient signal length: requires at least {} samples, provided {}",
                    required, provided
                )
            }
            SignalError::InvalidWindowSize(win) => {
                write!(f, "Invalid window size: {} (must be > 0)", win)
            }
            SignalError::InvalidFilterOrder(order) => {
                write!(f, "Invalid filter order: {} (must be > 0)", order)
            }
            SignalError::NonFiniteInput => {
                write!(
                    f,
                    "Input signal contains non-finite values (NaN or Infinity)"
                )
            }
            SignalError::InsufficientPeaks { required, provided } => {
                write!(
                    f,
                    "Insufficient peaks/intervals: requires at least {}, provided {}",
                    required, provided
                )
            }
        }
    }
}

impl std::error::Error for SignalError {}

/// Alias for `Result<T, SignalError>`
pub type Result<T> = std::result::Result<T, SignalError>;
