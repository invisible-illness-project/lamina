#!/usr/bin/env python3
"""Phase 8 (HRV Validation & Counterexamples) & Phase 9 (Cubic Interpolation) Validation."""

import csv
import json
import numpy as np
from pathlib import Path

from validation.bridge import LaminaBridge, LaminaBridgeError

def run_phase8_9():
    repo_root = Path(__file__).resolve().parents[1]
    out_dir = repo_root / "validation" / "revalidation-v2" / "results"
    out_dir.mkdir(parents=True, exist_ok=True)
    
    bridge = LaminaBridge(repo_root=repo_root)
    
    # ----------------------------------------------------
    # Phase 8: HRV Validation & Counterexample Matrix
    # ----------------------------------------------------
    print("Running Phase 8: HRV Validation & Counterexample Matrix...")
    
    test_cases = [
        ("Case 1: Clean Sinus Rhythm", [800.0, 805.0, 798.0, 802.0, 800.0]),
        ("Case 2: Physiological RR Variability", [790.0, 815.0, 785.0, 820.0, 800.0, 795.0]),
        ("Case 3: Respiratory Sinus Arrhythmia (RSA)", [750.0, 780.0, 810.0, 840.0, 820.0, 790.0, 760.0]),
        ("Case 4: Isolated Ectopic Beat", [800.0, 800.0, 450.0, 1150.0, 800.0, 800.0]),
        ("Case 5: Sustained Bigeminy", [600.0, 1000.0, 600.0, 1000.0, 600.0, 1000.0, 600.0]),
        ("Case 6: Trigeminy", [800.0, 800.0, 500.0, 800.0, 800.0, 500.0, 800.0]),
        ("Case 7: Missing Beat", [800.0, 1600.0, 800.0, 805.0, 795.0]),
        ("Case 8: Artifact/Outlier Interval", [800.0, 150.0, 800.0, 2500.0, 800.0]),
        ("Case 9: Alternating Artifact Pattern", [440.0, 830.0, 440.0, 830.0, 440.0, 830.0, 440.0, 830.0]),
    ]
    
    hrv_rows = []
    
    for case_name, raw_rr in test_cases:
        # Run bridge hrv-correct with default policy none to see raw classifications
        try:
            res_raw = bridge.hrv_correct(rr_intervals_ms=raw_rr, policy="none")
            classifications = res_raw.get("classifications", [])
        except LaminaBridgeError as e:
            classifications = [f"ERROR: {e}"]

        # Policy: Reject Invalid
        try:
            res_reject = bridge.hrv_correct(rr_intervals_ms=raw_rr, policy="reject_invalid")
            nn_reject = res_reject.get("nn_intervals_ms", [])
            rmssd_reject = res_reject.get("rmssd_ms")
            mean_nn_reject = res_reject.get("mean_nn_ms")
        except LaminaBridgeError as e:
            nn_reject = []
            rmssd_reject = None
            mean_nn_reject = None

        # Policy: Interpolate Linear
        try:
            res_lin = bridge.hrv_correct(rr_intervals_ms=raw_rr, policy="interpolate_linear")
            nn_lin = res_lin.get("nn_intervals_ms", [])
            rmssd_lin = res_lin.get("rmssd_ms")
            mean_nn_lin = res_lin.get("mean_nn_ms")
        except LaminaBridgeError as e:
            nn_lin = []
            rmssd_lin = None
            mean_nn_lin = None

        # Policy: Interpolate Cubic
        try:
            res_cub = bridge.hrv_correct(rr_intervals_ms=raw_rr, policy="interpolate_cubic")
            nn_cub = res_cub.get("nn_intervals_ms", [])
            rmssd_cub = res_cub.get("rmssd_ms")
            mean_nn_cub = res_cub.get("mean_nn_ms")
        except LaminaBridgeError as e:
            nn_cub = []
            rmssd_cub = None
            mean_nn_cub = None
        
        hrv_rows.append({
            "case_name": case_name,
            "raw_rr_ms": str(raw_rr),
            "classifications": str(classifications),
            "reject_nn_count": len(nn_reject),
            "reject_rmssd_ms": round(rmssd_reject, 4) if rmssd_reject is not None else "",
            "reject_mean_nn_ms": round(mean_nn_reject, 4) if mean_nn_reject is not None else "",
            "linear_rmssd_ms": round(rmssd_lin, 4) if rmssd_lin is not None else "",
            "linear_mean_nn_ms": round(mean_nn_lin, 4) if mean_nn_lin is not None else "",
            "cubic_rmssd_ms": round(rmssd_cub, 4) if rmssd_cub is not None else "",
            "cubic_mean_nn_ms": round(mean_nn_cub, 4) if mean_nn_cub is not None else "",
        })
        
        print(f"\n{case_name}:")
        print(f"  Raw RR: {raw_rr}")
        print(f"  Classifications: {classifications}")
        print(f"  Reject RMSSD: {rmssd_reject}, Linear RMSSD: {rmssd_lin}, Cubic RMSSD: {rmssd_cub}")

    # Write hrv_counterexamples.csv
    csv_hrv = out_dir / "hrv_counterexamples.csv"
    with open(csv_hrv, "w", newline="") as f:
        writer = csv.DictWriter(f, fieldnames=list(hrv_rows[0].keys()))
        writer.writeheader()
        writer.writerows(hrv_rows)
    print(f"\nHRV Counterexamples saved to {csv_hrv}")
    
    # ----------------------------------------------------
    # Phase 9: Cubic Interpolation Validation
    # ----------------------------------------------------
    print("\nRunning Phase 9: Cubic Interpolation Validation...")
    
    cubic_scenarios = [
        ("Curved Data with 5 valid NormalNN points (ectopic at index 3)", [800.0, 810.0, 840.0, 400.0, 1600.0, 890.0, 930.0]),
        ("Quadratic Curve with 4 valid points (ectopic at index 2)", [700.0, 750.0, 300.0, 1300.0, 875.0, 960.0]),
        ("Sinusoidal Curve with 6 valid points (artifact at index 3)", [800.0, 830.0, 850.0, 100.0, 830.0, 800.0, 770.0]),
        ("Curved Data with <4 valid points (triggers linear fallback)", [800.0, 820.0, 400.0, 860.0, 880.0]),
        ("Endpoint Invalid (First point invalid)", [3000.0, 800.0, 820.0, 850.0, 890.0]),
        ("Monotonic Data with 4 valid points", [750.0, 780.0, 300.0, 1300.0, 840.0, 870.0]),
    ]
    
    cubic_results = []
    
    for sc_name, rr_in in cubic_scenarios:
        try:
            res_lin = bridge.hrv_correct(rr_intervals_ms=rr_in, policy="interpolate_linear")
            nn_lin = np.asarray(res_lin.get("nn_intervals_ms", []), dtype=float)
        except LaminaBridgeError:
            nn_lin = np.array([])

        try:
            res_cub = bridge.hrv_correct(rr_intervals_ms=rr_in, policy="interpolate_cubic")
            nn_cub = np.asarray(res_cub.get("nn_intervals_ms", []), dtype=float)
        except LaminaBridgeError:
            nn_cub = np.array([])
        
        diff = np.abs(nn_lin - nn_cub) if (len(nn_lin) > 0 and len(nn_lin) == len(nn_cub)) else np.array([0.0])
        max_diff = float(np.max(diff)) if len(diff) > 0 else 0.0
        differ_substantially = max_diff > 1e-3
        
        cubic_results.append({
            "scenario": sc_name,
            "input_rr_ms": str(rr_in),
            "linear_output": str(nn_lin.tolist()),
            "cubic_output": str(nn_cub.tolist()),
            "max_abs_diff_ms": round(max_diff, 4),
            "differ_substantially": differ_substantially,
        })
        print(f"{sc_name}: Max Diff = {max_diff:.4f} ms (Differ: {differ_substantially})")
        
    cubic_json = out_dir / "cubic_vs_linear.json"
    with open(cubic_json, "w") as f:
        json.dump(cubic_results, f, indent=2)
    print(f"Cubic vs Linear interpolation comparison saved to {cubic_json}")

if __name__ == "__main__":
    run_phase8_9()
