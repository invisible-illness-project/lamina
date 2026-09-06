"""Signal processing module for Lamina."""

from typing import Optional, Union, Tuple
import numpy as np
from numpy.typing import NDArray

import lamina._lamina as _native

PeakDetectionConfig = _native.PyPeakDetectionConfig

def smooth_moving_average(
    signal: Union[NDArray[np.float64], NDArray[np.float32], list],
    window_size: int,
) -> NDArray[np.float64]:
    """Smooth a 1D numerical signal using a moving average window."""
    arr = np.asarray(signal, dtype=np.float64)
    return _native.smooth_moving_average(arr, window_size)

def filter(
    signal: Union[NDArray[np.float64], NDArray[np.float32], list],
    sampling_rate: float,
    low_cutoff: Optional[float] = None,
    high_cutoff: Optional[float] = None,
    order: int = 4,
    btype: str = "bandpass",
) -> NDArray[np.float64]:
    """Filter a 1D signal using Butterworth zero-phase filtering."""
    arr = np.asarray(signal, dtype=np.float64)
    return _native.filter(arr, sampling_rate, low_cutoff, high_cutoff, order, btype)

def filtfilt(
    signal: Union[NDArray[np.float64], NDArray[np.float32], list],
    sampling_rate: float,
    low_cutoff: Optional[float] = None,
    high_cutoff: Optional[float] = None,
    order: int = 4,
    btype: str = "bandpass",
) -> NDArray[np.float64]:
    """Apply zero-phase forward-backward Butterworth filtering."""
    arr = np.asarray(signal, dtype=np.float64)
    return _native.filtfilt(arr, sampling_rate, low_cutoff, high_cutoff, order, btype)

def findpeaks(
    signal: Union[NDArray[np.float64], NDArray[np.float32], list],
    sampling_rate: float,
    min_distance_sec: float = 0.4,
    min_height: Optional[float] = None,
) -> NDArray[np.int64]:
    """Find peak sample indices in a 1D signal."""
    arr = np.asarray(signal, dtype=np.float64)
    cfg = PeakDetectionConfig(min_distance_sec=min_distance_sec, min_height=min_height)
    indices = _native.findpeaks(arr, sampling_rate, cfg)
    return np.asarray(indices, dtype=np.int64)

def findpeaks_mask(
    signal: Union[NDArray[np.float64], NDArray[np.float32], list],
    sampling_rate: float,
    min_distance_sec: float = 0.4,
    min_height: Optional[float] = None,
) -> NDArray[np.bool_]:
    """Find peak mask (boolean array) for a 1D signal."""
    arr = np.asarray(signal, dtype=np.float64)
    cfg = PeakDetectionConfig(min_distance_sec=min_distance_sec, min_height=min_height)
    mask = _native.findpeaks_mask(arr, sampling_rate, cfg)
    return np.asarray(mask, dtype=np.bool_)
