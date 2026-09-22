"""Photoplethysmogram (PPG) processing module for Lamina."""

from typing import List, Optional, Union
import numpy as np
from numpy.typing import NDArray

import lamina._lamina as _native

PpgPeakDetectionConfig = _native.PyPpgPeakDetectionConfig

def clean(
    signal: Union[NDArray[np.float64], NDArray[np.float32], List[float]],
    sampling_rate: float = 100.0,
) -> NDArray[np.float64]:
    """Clean a PPG signal using bandpass filtering and smoothing."""
    arr = np.asarray(signal, dtype=np.float64)
    return _native.ppg_clean(arr, sampling_rate)

def findpeaks(
    signal: Union[NDArray[np.float64], NDArray[np.float32], List[float]],
    sampling_rate: float = 100.0,
    config: Optional[PpgPeakDetectionConfig] = None,
) -> NDArray[np.int64]:
    """Detect systolic pulse peak sample indices in a PPG signal."""
    arr = np.asarray(signal, dtype=np.float64)
    peaks = _native.ppg_findpeaks(arr, sampling_rate, config)
    return np.asarray(peaks, dtype=np.int64)

def findpeaks_mask(
    signal: Union[NDArray[np.float64], NDArray[np.float32], List[float]],
    sampling_rate: float = 100.0,
    config: Optional[PpgPeakDetectionConfig] = None,
) -> NDArray[np.bool_]:
    """Detect PPG pulse peaks as a boolean mask array."""
    arr = np.asarray(signal, dtype=np.float64)
    mask = _native.ppg_findpeaks_mask(arr, sampling_rate, config)
    return np.asarray(mask, dtype=np.bool_)

PulseLmPipeline = _native.PyPulseLmPipeline

def preprocess_pulselm(
    signal: Union[NDArray[np.float64], NDArray[np.float32], List[float]],
    sampling_rate: float,
) -> List[NDArray[np.float64]]:
    """Standardized 5-stage PulseLM PPG preprocessing pipeline.

    1. Anti-aliased polyphase resampling -> 125 Hz
    2. 4th-order zero-phase Butterworth low-pass -> 8 Hz
    3. 10-second fixed window segmentation -> 1250 samples
    4. Per-segment DC removal (mean subtraction)
    5. Per-segment min-max scaling -> [0, 1]
    """
    arr = np.asarray(signal, dtype=np.float64)
    return _native.ppg_preprocess_pulselm(arr, sampling_rate)

