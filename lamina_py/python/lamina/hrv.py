"""Heart Rate Variability (HRV) metrics module for Lamina."""

from typing import Union, List
import numpy as np
from numpy.typing import NDArray

import lamina._lamina as _native

def peaks_to_intervals(
    peaks: Union[NDArray[np.int64], List[int]],
    sampling_rate: float,
) -> NDArray[np.float64]:
    """Convert R-peak / pulse-peak sample indices to inter-beat intervals in seconds."""
    peak_vec = list(peaks)
    arr = _native.peaks_to_intervals(peak_vec, sampling_rate)
    return np.asarray(arr, dtype=np.float64)

def indices_to_intervals(
    indices: Union[NDArray[np.int64], List[int]],
    sampling_rate: float,
) -> NDArray[np.float64]:
    """Convert ordered peak indices to inter-beat intervals in seconds."""
    idx_vec = list(indices)
    arr = _native.indices_to_intervals(idx_vec, sampling_rate)
    return np.asarray(arr, dtype=np.float64)

def rmssd(
    rr_intervals: Union[NDArray[np.float64], NDArray[np.float32], list],
) -> float:
    """Compute Root Mean Square of Successive Differences (RMSSD) in seconds."""
    arr = np.asarray(rr_intervals, dtype=np.float64)
    return _native.rmssd(arr)

def mean_nn(
    rr_intervals: Union[NDArray[np.float64], NDArray[np.float32], list],
) -> float:
    """Compute mean normal-to-normal (NN) inter-beat interval in seconds."""
    arr = np.asarray(rr_intervals, dtype=np.float64)
    return _native.mean_nn(arr)
