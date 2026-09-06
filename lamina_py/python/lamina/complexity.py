"""Nonlinear complexity metrics module for Lamina."""

from typing import Union
import numpy as np
from numpy.typing import NDArray

import lamina._lamina as _native

def sample_entropy(
    signal: Union[NDArray[np.float64], NDArray[np.float32], list],
    m: int = 2,
    r: float = 0.2,
) -> float:
    """Compute Sample Entropy (SampEn) for a 1D numerical signal."""
    arr = np.asarray(signal, dtype=np.float64)
    return _native.sample_entropy(arr, m, r)
