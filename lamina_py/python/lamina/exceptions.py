"""
Exception hierarchy for Lamina scientific computing engine.
"""

try:
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
except ImportError:
    # Fallback for doc generation or uncompiled stubs
    class LaminaError(Exception):
        """Base exception for all Lamina errors."""
        pass

    class LaminaInputError(ValueError, LaminaError):
        """Invalid input signal data or dimensions."""
        pass

    class EmptySignalError(LaminaInputError):
        pass

    class NonFiniteInputError(LaminaInputError):
        pass

    class DimensionMismatchError(LaminaInputError):
        pass

    class UnsortedEventsError(LaminaInputError):
        pass

    class InsufficientSamplesError(LaminaInputError):
        pass

    class InsufficientPeaksError(LaminaInputError):
        pass

    class LaminaConfigurationError(ValueError, LaminaError):
        """Invalid configuration parameters."""
        pass

    class InvalidSamplingRateError(LaminaConfigurationError):
        pass

    class InvalidCutoffFrequencyError(LaminaConfigurationError):
        pass

    class InvalidWindowSizeError(LaminaConfigurationError):
        pass

    class InvalidFilterOrderError(LaminaConfigurationError):
        pass

    class LaminaProcessingError(RuntimeError, LaminaError):
        """Failure during computational execution."""
        pass


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
