"""
Exception hierarchy for Lamina scientific computing engine.
"""

from lamina._lamina import (
    LaminaError,
    LaminaInputError,
    EmptySignalError,
    NonFiniteInputError,
    DimensionMismatchError,
    UnsortedEventsError,
    InsufficientSamplesError,
    InsufficientPeaksError,
    LaminaConfigurationError,
    InvalidSamplingRateError,
    InvalidCutoffFrequencyError,
    InvalidWindowSizeError,
    InvalidFilterOrderError,
    LaminaProcessingError,
)

__all__ = [
    "LaminaError",
    "LaminaInputError",
    "EmptySignalError",
    "NonFiniteInputError",
    "DimensionMismatchError",
    "UnsortedEventsError",
    "InsufficientSamplesError",
    "InsufficientPeaksError",
    "LaminaConfigurationError",
    "InvalidSamplingRateError",
    "InvalidCutoffFrequencyError",
    "InvalidWindowSizeError",
    "InvalidFilterOrderError",
    "LaminaProcessingError",
]
