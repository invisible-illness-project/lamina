"""Respiration (RSP) processing module for Lamina."""

from typing import List, Optional, Union
import numpy as np
from numpy.typing import NDArray

import lamina._lamina as _native

RspCleaningConfig = _native.PyRspCleaningConfig
RspProcessingConfig = _native.PyRspProcessingConfig
RespirationCycle = _native.PyRespirationCycle

def clean(
    signal: Union[NDArray[np.float64], NDArray[np.float32], list],
    sampling_rate: float = 100.0,
    config: Optional[RspCleaningConfig] = None,
) -> NDArray[np.float64]:
    """Clean a respiration signal using bandpass filtering."""
    arr = np.asarray(signal, dtype=np.float64)
    return _native.rsp_clean(arr, sampling_rate, config)

def findpeaks(
    signal: Union[NDArray[np.float64], NDArray[np.float32], list],
    sampling_rate: float = 100.0,
    config: Optional[RspProcessingConfig] = None,
) -> NDArray[np.int64]:
    """Detect inspiration peak sample indices in a respiration signal."""
    arr = np.asarray(signal, dtype=np.float64)
    peaks = _native.rsp_findpeaks(arr, sampling_rate, config)
    return np.asarray(peaks, dtype=np.int64)

def cycles(
    signal: Union[NDArray[np.float64], NDArray[np.float32], list],
    sampling_rate: float = 100.0,
    peaks: Optional[Union[NDArray[np.int64], List[int]]] = None,
    config: Optional[RspProcessingConfig] = None,
) -> List[RespirationCycle]:
    """Extract individual respiration cycle structures."""
    arr = np.asarray(signal, dtype=np.float64)
    p_vec = list(peaks) if peaks is not None else None
    return _native.rsp_cycles(arr, sampling_rate, p_vec, config)

def rate(
    rsp_cycles: List[RespirationCycle],
    sampling_rate: float = 100.0,
    signal_length: Optional[int] = None,
) -> NDArray[np.float64]:
    """Compute continuous respiratory rate curve in breaths per minute (BPM)."""
    return np.asarray(_native.rsp_rate(rsp_cycles, sampling_rate, signal_length), dtype=np.float64)
