"""Electrocardiogram (ECG) processing module for Lamina."""

from typing import Optional, Union
import numpy as np
from numpy.typing import NDArray

import lamina._lamina as _native

EcgPeakDetectionConfig = _native.PyEcgPeakDetectionConfig

def clean(
    signal: Union[NDArray[np.float64], NDArray[np.float32], list],
    sampling_rate: float = 250.0,
) -> NDArray[np.float64]:
    """Clean an ECG signal using bandpass filtering and baseline removal."""
    arr = np.asarray(signal, dtype=np.float64)
    return _native.ecg_clean(arr, sampling_rate)

def findpeaks(
    signal: Union[NDArray[np.float64], NDArray[np.float32], list],
    sampling_rate: float = 250.0,
    config: Optional[EcgPeakDetectionConfig] = None,
) -> NDArray[np.int64]:
    """Detect R-peak sample indices in an ECG signal."""
    arr = np.asarray(signal, dtype=np.float64)
    peaks = _native.ecg_findpeaks(arr, sampling_rate, config)
    return np.asarray(peaks, dtype=np.int64)

def findpeaks_mask(
    signal: Union[NDArray[np.float64], NDArray[np.float32], list],
    sampling_rate: float = 250.0,
    config: Optional[EcgPeakDetectionConfig] = None,
) -> NDArray[np.bool_]:
    """Detect R-peaks as a boolean mask array."""
    arr = np.asarray(signal, dtype=np.float64)
    mask = _native.ecg_findpeaks_mask(arr, sampling_rate, config)
    return np.asarray(mask, dtype=np.bool_)
