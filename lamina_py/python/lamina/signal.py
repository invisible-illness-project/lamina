"""Signal processing module for Lamina."""

from typing import List, Optional, Union
import numpy as np
from numpy.typing import NDArray

import lamina._lamina as _native

PeakDetectionConfig = _native.PyPeakDetectionConfig

_VALID_BTYPES = ("lowpass", "highpass", "bandpass", "notch")

def smooth_moving_average(
    signal: Union[NDArray[np.float64], NDArray[np.float32], List[float]],
    window_size: int,
) -> NDArray[np.float64]:
    """Smooth a 1D numerical signal using a moving average window."""
    arr = np.asarray(signal, dtype=np.float64)
    return _native.smooth_moving_average(arr, window_size)

def filter(
    signal: Union[NDArray[np.float64], NDArray[np.float32], List[float]],
    sampling_rate: float,
    low_cutoff: Optional[float] = None,
    high_cutoff: Optional[float] = None,
    order: int = 1,
    btype: str = "bandpass",
) -> NDArray[np.float64]:
    """Filter a 1D signal with zero-phase Butterworth filtering.

    btype: "lowpass", "highpass", "bandpass" (default), or "notch".
    Single-cutoff kinds use low_cutoff (falling back to high_cutoff);
    bandpass and notch require both cutoffs.
    """
    if btype not in _VALID_BTYPES:
        raise ValueError(f"btype must be one of {_VALID_BTYPES}, got {btype!r}")
    arr = np.asarray(signal, dtype=np.float64)
    return _native.filter(arr, sampling_rate, low_cutoff, high_cutoff, order, btype)

def filtfilt(
    signal: Union[NDArray[np.float64], NDArray[np.float32], List[float]],
    sampling_rate: float,
    low_cutoff: Optional[float] = None,
    high_cutoff: Optional[float] = None,
    order: int = 1,
    btype: str = "bandpass",
) -> NDArray[np.float64]:
    """Apply zero-phase forward-backward Butterworth filtering."""
    if btype not in _VALID_BTYPES:
        raise ValueError(f"btype must be one of {_VALID_BTYPES}, got {btype!r}")
    arr = np.asarray(signal, dtype=np.float64)
    return _native.filtfilt(arr, sampling_rate, low_cutoff, high_cutoff, order, btype)

def findpeaks(
    signal: Union[NDArray[np.float64], NDArray[np.float32], List[float]],
    sampling_rate: float,
    min_distance_sec: float = 0.4,
    min_height: Optional[float] = None,
    min_prominence: Optional[float] = None,
    min_width: Optional[int] = None,
    threshold: Optional[float] = None,
) -> NDArray[np.int64]:
    """Find peak sample indices in a 1D signal.

    min_width is in samples; amplitude parameters are in signal units.
    """
    arr = np.asarray(signal, dtype=np.float64)
    min_dist_samples = int(min_distance_sec * sampling_rate) if min_distance_sec > 0 else None
    cfg = PeakDetectionConfig(
        min_height=min_height,
        min_distance=min_dist_samples,
        min_prominence=min_prominence,
        min_width=min_width,
        threshold=threshold,
    )
    indices = _native.findpeaks(arr, cfg)
    return np.asarray(indices, dtype=np.int64)

def findpeaks_mask(
    signal: Union[NDArray[np.float64], NDArray[np.float32], List[float]],
    sampling_rate: float,
    min_distance_sec: float = 0.4,
    min_height: Optional[float] = None,
    min_prominence: Optional[float] = None,
    min_width: Optional[int] = None,
    threshold: Optional[float] = None,
) -> NDArray[np.bool_]:
    """Find peaks in a 1D signal, returned as a boolean mask."""
    arr = np.asarray(signal, dtype=np.float64)
    min_dist_samples = int(min_distance_sec * sampling_rate) if min_distance_sec > 0 else None
    cfg = PeakDetectionConfig(
        min_height=min_height,
        min_distance=min_dist_samples,
        min_prominence=min_prominence,
        min_width=min_width,
        threshold=threshold,
    )
    mask = _native.findpeaks_mask(arr, cfg)
    return np.asarray(mask, dtype=np.bool_)

def resample_poly(
    signal: Union[NDArray[np.float64], NDArray[np.float32], List[float]],
    up: int,
    down: int,
) -> NDArray[np.float64]:
    """Polyphase rational resampling using Kaiser-windowed FIR filter.

    Matches SciPy scipy.signal.resample_poly.
    """
    arr = np.asarray(signal, dtype=np.float64)
    return _native.resample_poly(arr, up, down)

def segment_signal(
    signal: Union[NDArray[np.float64], NDArray[np.float32], List[float]],
    window_samples: int,
    step_samples: int,
    tail_policy: str = "drop",
) -> List[NDArray[np.float64]]:
    """Partition a 1D signal into fixed-length window segments."""
    arr = np.asarray(signal, dtype=np.float64)
    return _native.segment_signal(arr, window_samples, step_samples, tail_policy)

def remove_dc(
    signal: Union[NDArray[np.float64], NDArray[np.float32], List[float]],
) -> NDArray[np.float64]:
    """Remove DC (non-pulsatile) baseline component via sample mean subtraction."""
    arr = np.asarray(signal, dtype=np.float64)
    return _native.remove_dc(arr)

def minmax_scale(
    signal: Union[NDArray[np.float64], NDArray[np.float32], List[float]],
    feature_range: tuple[float, float] = (0.0, 1.0),
    degenerate_policy: str = "midpoint",
) -> NDArray[np.float64]:
    """Scale signal values into feature_range [a, b]. Handles zero-variance signals with degenerate_policy."""
    arr = np.asarray(signal, dtype=np.float64)
    return _native.minmax_scale(arr, feature_range, degenerate_policy)

