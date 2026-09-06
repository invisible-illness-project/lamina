"""HRV metrics (SPEC §5).

Callers MUST keep the provenance of the compared values straight: HRV derived
from Lamina-detected beats is not the same quantity as HRV derived from
dataset annotations, and the two should be reported separately (task §8/HRV).
"""

from __future__ import annotations

import numpy as np

from .rate import _aligned, _pearson


def hrv_metrics(reference, estimated) -> dict:
    """Absolute/relative error and correlation over aligned HRV value arrays.

    Works elementwise, e.g. per-recording RMSSD values. ``mean_relative_error``
    is mean(|est − ref| / |ref|) over pairs with ref != 0.
    """
    ref, est = _aligned(reference, estimated)
    if ref.size == 0:
        return {
            "mae": None, "rmse": None, "bias": None,
            "mean_relative_error": None, "pearson_r": None, "n": 0,
        }
    diff = est - ref
    nz = ref != 0
    mre = float(np.mean(np.abs(diff[nz]) / np.abs(ref[nz]))) if np.any(nz) else None
    return {
        "mae": float(np.mean(np.abs(diff))),
        "rmse": float(np.sqrt(np.mean(diff**2))),
        "bias": float(np.mean(diff)),
        "mean_relative_error": mre,
        "pearson_r": _pearson(ref, est),
        "n": int(ref.size),
    }
