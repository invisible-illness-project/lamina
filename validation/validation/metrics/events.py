"""Event matching and peak-detection metrics (SPEC §5).

Matching rule (greedy one-to-one): reference events are scanned in ascending
sample order; each reference is paired with the nearest not-yet-matched
detection inside ``±tolerance_sec``. Every detection matches at most one
reference event and vice versa.
"""

from __future__ import annotations

from dataclasses import dataclass, field

import numpy as np


@dataclass
class MatchResult:
    """Outcome of greedy one-to-one event matching."""

    matches: list[tuple[int, int]] = field(default_factory=list)  # (ref_idx, det_idx)
    timing_errors_sec: np.ndarray = field(default_factory=lambda: np.empty(0))
    tolerance_sec: float = 0.0

    @property
    def tp(self) -> int:
        return len(self.matches)

    @property
    def fn(self) -> int:
        return self.n_reference - self.tp

    @property
    def fp(self) -> int:
        return self.n_detected - self.tp

    n_reference: int = 0
    n_detected: int = 0


def match_events(
    reference_indices,
    detected_indices,
    fs: float,
    tolerance_sec: float,
) -> MatchResult:
    """Greedy one-to-one event matching (see module docstring)."""
    ref = np.sort(np.asarray(reference_indices, dtype=np.int64))
    det = np.sort(np.asarray(detected_indices, dtype=np.int64))
    tol_samples = tolerance_sec * fs

    matched_det: set[int] = set()
    matches: list[tuple[int, int]] = []
    errors: list[float] = []

    for r in ref:
        # Candidate detections within tolerance, nearest first.
        best_j, best_dist = None, None
        for j, d in enumerate(det):
            if j in matched_det:
                continue
            dist = abs(int(d) - int(r))
            if dist > tol_samples:
                if d > r:  # sorted: further detections can only be farther
                    break
                continue
            if best_dist is None or dist < best_dist:
                best_j, best_dist = j, dist
        if best_j is not None:
            matched_det.add(best_j)
            matches.append((int(r), int(det[best_j])))
            errors.append((int(det[best_j]) - int(r)) / fs)

    return MatchResult(
        matches=matches,
        timing_errors_sec=np.asarray(errors, dtype=float),
        tolerance_sec=float(tolerance_sec),
        n_reference=int(ref.shape[0]),
        n_detected=int(det.shape[0]),
    )


def peak_detection_metrics(
    reference_indices,
    detected_indices,
    fs: float,
    tolerance_sec: float,
) -> dict:
    """TP/FP/FN, precision, recall, F1 and timing-error statistics."""
    m = match_events(reference_indices, detected_indices, fs, tolerance_sec)
    tp, fp, fn = m.tp, m.fp, m.fn
    precision = tp / (tp + fp) if (tp + fp) > 0 else None
    recall = tp / (tp + fn) if (tp + fn) > 0 else None
    f1 = (
        2 * precision * recall / (precision + recall)
        if precision is not None and recall is not None and (precision + recall) > 0
        else None
    )
    errs = m.timing_errors_sec
    return {
        "tp": tp,
        "fp": fp,
        "fn": fn,
        "precision": precision,
        "recall": recall,
        "f1": f1,
        "mean_abs_timing_error_sec": float(np.mean(np.abs(errs))) if errs.size else None,
        "median_timing_error_sec": float(np.median(errs)) if errs.size else None,
        "n_reference": m.n_reference,
        "n_detected": m.n_detected,
        "tolerance_sec": m.tolerance_sec,
    }
