"""Electrodermal Activity (EDA / GSR) processing module for Lamina."""

from typing import List, Optional, Union
import numpy as np
from numpy.typing import NDArray

import lamina._lamina as _native

EdaDecompositionConfig = _native.PyEdaDecompositionConfig
EdaPeakDetectionConfig = _native.PyEdaPeakDetectionConfig
ScrEvent = _native.PyScrEvent
EdaComponents = _native.PyEdaComponents

def clean(
    signal: Union[NDArray[np.float64], NDArray[np.float32], list],
    sampling_rate: float = 100.0,
) -> NDArray[np.float64]:
    """Clean an EDA signal using lowpass filtering."""
    arr = np.asarray(signal, dtype=np.float64)
    return _native.eda_clean(arr, sampling_rate)

def decompose(
    signal: Union[NDArray[np.float64], NDArray[np.float32], list],
    sampling_rate: float = 100.0,
    config: Optional[EdaDecompositionConfig] = None,
) -> EdaComponents:
    """Decompose raw EDA into tonic (SCL) and phasic (SCR) components."""
    arr = np.asarray(signal, dtype=np.float64)
    return _native.eda_decompose(arr, sampling_rate, config)

def phasic(
    signal: Union[NDArray[np.float64], NDArray[np.float32], list],
    sampling_rate: float = 100.0,
) -> NDArray[np.float64]:
    """Extract phasic SCR driver signal directly from EDA."""
    arr = np.asarray(signal, dtype=np.float64)
    return _native.eda_phasic(arr, sampling_rate)

def findpeaks(
    signal: Union[NDArray[np.float64], NDArray[np.float32], list],
    sampling_rate: float = 100.0,
    config: Optional[EdaPeakDetectionConfig] = None,
) -> NDArray[np.int64]:
    """Detect SCR peak sample indices in an EDA signal."""
    arr = np.asarray(signal, dtype=np.float64)
    peaks = _native.eda_findpeaks(arr, sampling_rate, config)
    return np.asarray(peaks, dtype=np.int64)

def findpeaks_events(
    signal: Union[NDArray[np.float64], NDArray[np.float32], list],
    sampling_rate: float = 100.0,
    config: Optional[EdaPeakDetectionConfig] = None,
) -> List[ScrEvent]:
    """Detect SCR events with onset, peak index, amplitude, and rise time."""
    arr = np.asarray(signal, dtype=np.float64)
    return _native.eda_findpeaks_events(arr, sampling_rate, config)
