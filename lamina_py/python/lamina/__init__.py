"""Lamina: Idiomatic scientific Python interface for the Lamina Rust computational engine."""

from lamina._lamina import __version__
from lamina.exceptions import (
    LaminaError,
    LaminaInputError,
    EmptySignalError,
    NonFiniteInputError,
    DimensionMismatchError,
    InsufficientSamplesError,
    LaminaConfigurationError,
    InvalidSamplingRateError,
    InvalidCutoffFrequencyError,
    LaminaProcessingError,
)
from lamina import signal
from lamina import ecg
from lamina import ppg
from lamina import eda
from lamina import rsp
from lamina import hrv
from lamina import complexity
from lamina import autonomic
from lamina import rppg
from lamina import features
from lamina import multimodal

__all__ = [
    "__version__",
    "LaminaError",
    "LaminaInputError",
    "EmptySignalError",
    "NonFiniteInputError",
    "DimensionMismatchError",
    "InsufficientSamplesError",
    "LaminaConfigurationError",
    "InvalidSamplingRateError",
    "InvalidCutoffFrequencyError",
    "LaminaProcessingError",
    "signal",
    "ecg",
    "ppg",
    "eda",
    "rsp",
    "hrv",
    "complexity",
    "autonomic",
    "rppg",
    "features",
    "multimodal",
]
