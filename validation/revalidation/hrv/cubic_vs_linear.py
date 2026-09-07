#!/usr/bin/env python3
"""Wave F revalidation, task §2: does CorrectionPolicy::InterpolateCubic
differ BEHAVIORALLY from InterpolateLinear?

Crafts a series with consecutive invalid intervals between known-good anchors
in a region of high curvature (sinus arrhythmia trough), where any genuine
cubic Hermite/spline interpolant differs from a straight line by >5 ms.
Compares bridge outputs sample-by-sample, and against scipy's CubicSpline as
the reference for what "cubic" should produce.

Writes cubic_vs_linear.csv and prints the verdict.
"""
from __future__ import annotations

import csv
import json
import sys
from pathlib import Path

import numpy as np
from scipy.interpolate import CubicSpline

REPO = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(REPO))
from validation.bridge import LaminaBridge  # noqa: E402

OUT = Path(__file__).resolve().parent


def main() -> None:
    bridge = LaminaBridge(repo_root=REPO, auto_build=False)

    n = 60
    i = np.arange(n)
    rr = 800.0 + 80.0 * np.sin(2 * np.pi * i / 12.0)
    # Gap 1: three consecutive ARTIFACT intervals (250 ms) straddling the
    # trough (i=9 is the 720 ms trough; anchors i=9 and i=13).
    rr[10:13] = 250.0
    # Gap 2: two consecutive ECTOPIC-type intervals (500 ms) near a steep
    # flank (anchors i=39, i=42).
    rr[40:42] = 500.0

    rl = bridge.hrv_correct(rr, policy="interpolate_linear")
    rc = bridge.hrv_correct(rr, policy="interpolate_cubic")
    nn_l = np.array(rl["nn_intervals_ms"], dtype=float)
    nn_c = np.array(rc["nn_intervals_ms"], dtype=float)
    qual_l = rl["interval_quality"]
    qual_c = rc["interval_quality"]

    assert qual_l == qual_c, "policies disagree on classification?!"
    invalid = [k for k, q in enumerate(qual_l) if q != "normal_nn"]
    print("invalid indices:", invalid)
    print("labels at gaps:", {k: qual_l[k] for k in invalid})

    diff = np.abs(nn_c - nn_l)
    max_diff = float(diff.max())

    # Reference: what a genuine cubic spline through the valid anchors gives.
    valid_idx = np.array([k for k, q in enumerate(qual_l)
                          if q == "normal_nn"], dtype=float)
    valid_val = rr[valid_idx.astype(int)]
    cs = CubicSpline(valid_idx, valid_val)
    ref_cubic = cs(np.array(invalid, dtype=float))
    lin_at_gap = nn_l[invalid]

    rows = []
    for k in invalid:
        rows.append({
            "index": k,
            "quality": qual_l[k],
            "original_ms": round(float(rr[k]), 6),
            "interp_linear_ms": round(float(nn_l[k]), 6),
            "interp_cubic_ms": round(float(nn_c[k]), 6),
            "abs_diff_cubic_minus_linear": round(float(abs(nn_c[k] - nn_l[k])), 12),
            "reference_cubicspline_ms": round(float(cs(float(k))), 6),
            "linear_vs_true_cubic_error_ms": round(
                float(abs(nn_l[k] - cs(float(k)))), 6),
        })

    identical = max_diff < 1e-9
    # sanity: the crafted gaps really are spots where linear and a true cubic
    # differ by > 5 ms (otherwise the test would be inconclusive)
    lin_vs_cubic_err = np.abs(lin_at_gap - ref_cubic)
    conclusive = bool((lin_vs_cubic_err > 5.0).any())

    with open(OUT / "cubic_vs_linear.csv", "w", newline="") as fh:
        w = csv.DictWriter(fh, fieldnames=list(rows[0].keys()))
        w.writeheader()
        w.writerows(rows)

    verdict = {
        "max_abs_diff_between_policies_ms": max_diff,
        "policies_identical": identical,
        "test_conclusive_linear_vs_true_cubic_gt5ms": conclusive,
        "max_linear_error_vs_true_cubic_ms": float(lin_vs_cubic_err.max()),
        "rmssd_linear": rl["rmssd_ms"],
        "rmssd_cubic": rc["rmssd_ms"],
        "verdict": ("BUG CONFIRMED: interpolate_cubic executes the linear "
                    "code path (outputs identical to interpolate_linear)"
                    if identical and conclusive else
                    "INCONCLUSIVE" if not conclusive else
                    "policies differ (cubic is real)"),
    }
    print(json.dumps(verdict, indent=2))
    print(f"wrote {OUT / 'cubic_vs_linear.csv'}")


if __name__ == "__main__":
    main()
