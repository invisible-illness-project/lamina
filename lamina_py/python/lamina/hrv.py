"""Heart Rate Variability (HRV) metrics module for Lamina."""

from typing import Union, List
import numpy as np
from numpy.typing import NDArray

import lamina._lamina as _native

def peaks_to_intervals(
    peaks: Union[NDArray[np.int64], NDArray[np.bool_], List[int]],
    sampling_rate: float,
) -> NDArray[np.float64]:
    """Convert peak indices or boolean peak mask to inter-beat intervals in seconds."""
    arr = np.asarray(peaks)
    if arr.dtype == np.bool_:
        return _native.peaks_to_intervals(arr, sampling_rate)
    else:
        indices = [int(x) for x in arr]
        res = _native.indices_to_intervals(indices, sampling_rate)
        # Convert ms to sec for consistent unit contract in lamina.hrv
        return np.asarray(res, dtype=np.float64) / 1000.0

def indices_to_intervals(
    indices: Union[NDArray[np.int64], List[int]],
    sampling_rate: float,
) -> NDArray[np.float64]:
    """Convert ordered peak indices to inter-beat intervals in seconds."""
    idx_vec = [int(x) for x in indices]
    res = _native.indices_to_intervals(idx_vec, sampling_rate)
    return np.asarray(res, dtype=np.float64) / 1000.0

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
