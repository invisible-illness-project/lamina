use lamina::SignalError;
use pyo3::create_exception;
use pyo3::exceptions::{PyException, PyRuntimeError, PyValueError};
use pyo3::prelude::*;

create_exception!(_lamina, LaminaError, PyException);
create_exception!(_lamina, LaminaInputError, PyValueError);
create_exception!(_lamina, EmptySignalError, LaminaInputError);
create_exception!(_lamina, NonFiniteInputError, LaminaInputError);
create_exception!(_lamina, DimensionMismatchError, LaminaInputError);
create_exception!(_lamina, UnsortedEventsError, LaminaInputError);
create_exception!(_lamina, InsufficientSamplesError, LaminaInputError);
create_exception!(_lamina, InsufficientPeaksError, LaminaInputError);

create_exception!(_lamina, LaminaConfigurationError, PyValueError);
create_exception!(_lamina, InvalidSamplingRateError, LaminaConfigurationError);
create_exception!(
    _lamina,
    InvalidCutoffFrequencyError,
    LaminaConfigurationError
);
create_exception!(_lamina, InvalidWindowSizeError, LaminaConfigurationError);
create_exception!(_lamina, InvalidFilterOrderError, LaminaConfigurationError);

create_exception!(_lamina, LaminaProcessingError, PyRuntimeError);

pub fn map_signal_error(err: SignalError) -> PyErr {
    match err {
        SignalError::EmptySignal => EmptySignalError::new_err(err.to_string()),
        SignalError::InvalidSamplingRate(_) => InvalidSamplingRateError::new_err(err.to_string()),
        SignalError::InvalidCutoffFrequency(_) => {
            InvalidCutoffFrequencyError::new_err(err.to_string())
        }
        SignalError::InsufficientSamples { .. } => {
            InsufficientSamplesError::new_err(err.to_string())
        }
        SignalError::InvalidWindowSize(_) => InvalidWindowSizeError::new_err(err.to_string()),
        SignalError::InvalidFilterOrder(_) => InvalidFilterOrderError::new_err(err.to_string()),
        SignalError::NonFiniteInput => NonFiniteInputError::new_err(err.to_string()),
        SignalError::InsufficientPeaks { .. } => InsufficientPeaksError::new_err(err.to_string()),
        SignalError::DimensionMismatch => DimensionMismatchError::new_err(err.to_string()),
        SignalError::UnsortedEvents => UnsortedEventsError::new_err(err.to_string()),
    }
}

pub fn register_errors(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("LaminaError", m.py().get_type::<LaminaError>())?;
    m.add("LaminaInputError", m.py().get_type::<LaminaInputError>())?;
    m.add("EmptySignalError", m.py().get_type::<EmptySignalError>())?;
    m.add(
        "NonFiniteInputError",
        m.py().get_type::<NonFiniteInputError>(),
    )?;
    m.add(
        "DimensionMismatchError",
        m.py().get_type::<DimensionMismatchError>(),
    )?;
    m.add(
        "UnsortedEventsError",
        m.py().get_type::<UnsortedEventsError>(),
    )?;
    m.add(
        "InsufficientSamplesError",
        m.py().get_type::<InsufficientSamplesError>(),
    )?;
    m.add(
        "InsufficientPeaksError",
        m.py().get_type::<InsufficientPeaksError>(),
    )?;
    m.add(
        "LaminaConfigurationError",
        m.py().get_type::<LaminaConfigurationError>(),
    )?;
    m.add(
        "InvalidSamplingRateError",
        m.py().get_type::<InvalidSamplingRateError>(),
    )?;
    m.add(
        "InvalidCutoffFrequencyError",
        m.py().get_type::<InvalidCutoffFrequencyError>(),
    )?;
    m.add(
        "InvalidWindowSizeError",
        m.py().get_type::<InvalidWindowSizeError>(),
    )?;
    m.add(
        "InvalidFilterOrderError",
        m.py().get_type::<InvalidFilterOrderError>(),
    )?;
    m.add(
        "LaminaProcessingError",
        m.py().get_type::<LaminaProcessingError>(),
    )?;
    Ok(())
}
