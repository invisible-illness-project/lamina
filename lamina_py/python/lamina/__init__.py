"""Lamina: Idiomatic scientific Python interface for the Lamina Rust computational engine."""

from lamina import (
    autonomic,
    complexity,
    ecg,
    eda,
    features,
    hrv,
    multimodal,
    ppg,
    rppg,
    rsp,
    signal,
)
from lamina._lamina import __version__
from lamina.exceptions import (
    DimensionMismatchError,
    EmptySignalError,
    InsufficientSamplesError,
    InvalidCutoffFrequencyError,
    InvalidSamplingRateError,
    LaminaConfigurationError,
    LaminaError,
    LaminaInputError,
    LaminaProcessingError,
    NonFiniteInputError,
)

__all__ = [
    "DimensionMismatchError",
    "EmptySignalError",
    "InsufficientSamplesError",
    "InvalidCutoffFrequencyError",
    "InvalidSamplingRateError",
    "LaminaConfigurationError",
    "LaminaError",
    "LaminaInputError",
    "LaminaProcessingError",
    "NonFiniteInputError",
    "__version__",
    "autonomic",
    "complexity",
    "ecg",
    "eda",
    "features",
    "hrv",
    "multimodal",
    "ppg",
    "rppg",
    "rsp",
    "signal",
]
