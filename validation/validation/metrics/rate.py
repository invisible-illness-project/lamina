"""Rate metrics (heart rate, respiratory rate; SPEC §5)."""

from __future__ import annotations

import numpy as np


def rate_metrics(reference, estimated) -> dict:
    """MAE / RMSE / bias / Pearson correlation over aligned rate arrays.

    ``bias`` is mean(estimated − reference). Pairs containing non-finite
    values are dropped; ``n`` reports the number of pairs used.
    """
    ref, est = _aligned(reference, estimated)
    if ref.size == 0:
        return {"mae": None, "rmse": None, "bias": None, "pearson_r": None, "n": 0}
    diff = est - ref
    return {
        "mae": float(np.mean(np.abs(diff))),
        "rmse": float(np.sqrt(np.mean(diff**2))),
        "bias": float(np.mean(diff)),
        "pearson_r": _pearson(ref, est),
        "n": int(ref.size),
    }


def _aligned(reference, estimated) -> tuple[np.ndarray, np.ndarray]:
    ref = np.asarray(reference, dtype=float).ravel()
    est = np.asarray(estimated, dtype=float).ravel()
    if ref.shape != est.shape:
        raise ValueError(f"shape mismatch: reference {ref.shape} vs estimated {est.shape}")
    mask = np.isfinite(ref) & np.isfinite(est)
    return ref[mask], est[mask]


def _pearson(a: np.ndarray, b: np.ndarray) -> float | None:
    if a.size < 2 or np.std(a) == 0 or np.std(b) == 0:
        return None
    return float(np.corrcoef(a, b)[0, 1])
