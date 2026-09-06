use std::fmt;

/// Primary error type for Lamina physiological signal processing operations.
#[derive(Debug, Clone, PartialEq)]
pub struct SignalError {
    pub message: String,
}

impl fmt::Display for SignalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for SignalError {}

impl From<lamina::SignalError> for SignalError {
    fn from(err: lamina::SignalError) -> Self {
        Self {
            message: err.to_string(),
        }
    }
}

impl SignalError {
    pub fn empty_signal() -> Self {
        lamina::SignalError::EmptySignal.into()
    }
    pub fn invalid_sampling_rate(rate: f64) -> Self {
        lamina::SignalError::InvalidSamplingRate(rate).into()
    }
    pub fn non_finite_input() -> Self {
        lamina::SignalError::NonFiniteInput.into()
    }
    pub fn insufficient_peaks(required: usize, provided: usize) -> Self {
        lamina::SignalError::InsufficientPeaks { required, provided }.into()
    }
    pub fn dimension_mismatch() -> Self {
        lamina::SignalError::DimensionMismatch.into()
    }
}
