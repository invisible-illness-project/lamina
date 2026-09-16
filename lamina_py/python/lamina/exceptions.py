"""
Exception hierarchy for Lamina scientific computing engine.
"""

from lamina._lamina import (
    DimensionMismatchError,
    EmptySignalError,
    InsufficientPeaksError,
    InsufficientSamplesError,
    InvalidCutoffFrequencyError,
    InvalidFilterOrderError,
    InvalidSamplingRateError,
    InvalidWindowSizeError,
    LaminaConfigurationError,
    LaminaError,
    LaminaInputError,
    LaminaProcessingError,
    NonFiniteInputError,
    UnsortedEventsError,
)

__all__ = [
    "DimensionMismatchError",
    "EmptySignalError",
    "InsufficientPeaksError",
    "InsufficientSamplesError",
    "InvalidCutoffFrequencyError",
    "InvalidFilterOrderError",
    "InvalidSamplingRateError",
    "InvalidWindowSizeError",
    "LaminaConfigurationError",
    "LaminaError",
    "LaminaInputError",
    "LaminaProcessingError",
    "NonFiniteInputError",
    "UnsortedEventsError",
]
