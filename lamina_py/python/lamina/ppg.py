"""Photoplethysmogram (PPG) processing module for Lamina."""

from typing import Optional, Union
import numpy as np
from numpy.typing import NDArray

import lamina._lamina as _native

PpgPeakDetectionConfig = _native.PyPpgPeakDetectionConfig

def clean(
    signal: Union[NDArray[np.float64], NDArray[np.float32], list],
    sampling_rate: float = 100.0,
) -> NDArray[np.float64]:
    """Clean a PPG signal using bandpass filtering and smoothing."""
    arr = np.asarray(signal, dtype=np.float64)
    return _native.ppg_clean(arr, sampling_rate)

def findpeaks(
    signal: Union[NDArray[np.float64], NDArray[np.float32], list],
    sampling_rate: float = 100.0,
    config: Optional[PpgPeakDetectionConfig] = None,
) -> NDArray[np.int64]:
    """Detect systolic pulse peak sample indices in a PPG signal."""
    arr = np.asarray(signal, dtype=np.float64)
    peaks = _native.ppg_findpeaks(arr, sampling_rate, config)
    return np.asarray(peaks, dtype=np.int64)

def findpeaks_mask(
    signal: Union[NDArray[np.float64], NDArray[np.float32], list],
    sampling_rate: float = 100.0,
    config: Optional[PpgPeakDetectionConfig] = None,
) -> NDArray[np.bool_]:
    """Detect PPG pulse peaks as a boolean mask array."""
    arr = np.asarray(signal, dtype=np.float64)
    mask = _native.ppg_findpeaks_mask(arr, sampling_rate, config)
    return np.asarray(mask, dtype=np.bool_)
