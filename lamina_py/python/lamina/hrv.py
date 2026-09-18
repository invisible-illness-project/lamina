"""Heart Rate Variability (HRV) metrics module for Lamina."""

from typing import Union, List, Optional
import numpy as np
from numpy.typing import NDArray

import lamina._lamina as _native

# Unit contract: inter-beat intervals and time-domain HRV metrics are in
# MILLISECONDS (ms), matching the Rust scientific contract.

CorrectionPolicy = _native.PyCorrectionPolicy

def peaks_to_intervals(
    peaks: Union[NDArray[np.int64], NDArray[np.bool_], List[int]],
    sampling_rate: float,
) -> NDArray[np.float64]:
    """Convert peak indices or a boolean peak mask to inter-beat intervals in milliseconds (ms)."""
    arr = np.asarray(peaks)
    if arr.dtype == np.bool_:
        return np.asarray(_native.peaks_to_intervals(arr, sampling_rate), dtype=np.float64)
    else:
        indices = [int(x) for x in arr]
        return np.asarray(_native.indices_to_intervals(indices, sampling_rate), dtype=np.float64)

def indices_to_intervals(
    indices: Union[NDArray[np.int64], List[int]],
    sampling_rate: float,
) -> NDArray[np.float64]:
    """Convert ordered peak indices to inter-beat intervals in milliseconds (ms)."""
    idx_vec = [int(x) for x in indices]
    return np.asarray(_native.indices_to_intervals(idx_vec, sampling_rate), dtype=np.float64)

def rmssd(
    rr_intervals: Union[NDArray[np.float64], NDArray[np.float32], list],
) -> float:
    """Compute Root Mean Square of Successive Differences (RMSSD) in milliseconds (ms)."""
    arr = np.asarray(rr_intervals, dtype=np.float64)
    return _native.rmssd(arr)

def mean_nn(
    rr_intervals: Union[NDArray[np.float64], NDArray[np.float32], list],
) -> float:
    """Compute the mean normal-to-normal (NN) inter-beat interval in milliseconds (ms)."""
    arr = np.asarray(rr_intervals, dtype=np.float64)
    return _native.mean_nn(arr)

def classify_intervals(
    rr_intervals: Union[NDArray[np.float64], NDArray[np.float32], list],
    percent_threshold: Optional[float] = None,
) -> List[str]:
    """Classify each inter-beat interval (ms) as NormalNN, EctopicRR, ArtifactRR, or Missing."""
    arr = np.asarray(rr_intervals, dtype=np.float64)
    return _native.classify_intervals(arr, percent_threshold)

def clean_rr_intervals(
    rr_intervals: Union[NDArray[np.float64], NDArray[np.float32], list],
    policy: CorrectionPolicy,
) -> NDArray[np.float64]:
    """Clean inter-beat intervals (ms) into a validated N-N series per the correction policy."""
    arr = np.asarray(rr_intervals, dtype=np.float64)
    return _native.clean_rr_intervals(arr, policy)
