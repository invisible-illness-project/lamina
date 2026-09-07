#!/usr/bin/env python3
"""Phase 12: API and Numerical Hygiene Validation."""

import json
import math
import numpy as np
from pathlib import Path

from validation.bridge import LaminaBridge, LaminaBridgeError

def run_phase12():
    repo_root = Path(__file__).resolve().parents[1]
    out_dir = repo_root / "validation" / "revalidation-v2" / "results"
    out_dir.mkdir(parents=True, exist_ok=True)
    
    bridge = LaminaBridge(repo_root=repo_root)
    
    print("Running Phase 12: API and Numerical Hygiene Validation...")
    
    test_cases = []
    
    def probe_op(op_name, func, expected_error_type=None):
        try:
            res = func()
            test_cases.append({
                "op": op_name,
                "status": "UNEXPECTED_SUCCESS",
                "panic": False,
                "result": str(res)[:100],
            })
            print(f"[{op_name:40s}] UNEXPECTED_SUCCESS")
        except LaminaBridgeError as exc:
            kind = getattr(exc, "kind", "unknown")
            msg = str(exc)
            test_cases.append({
                "op": op_name,
                "status": "HANDLED_ERROR",
                "panic": False,
                "error_kind": kind,
                "error_message": msg,
            })
            print(f"[{op_name:40s}] HANDLED_ERROR [{kind}] -> {msg[:60]}...")
        except Exception as exc:
            test_cases.append({
                "op": op_name,
                "status": "CRASH_OR_UNHANDLED",
                "panic": True,
                "error": str(exc),
            })
            print(f"[{op_name:40s}] CRASH/UNHANDLED -> {exc}")

    # 1. ECG Peaks — invalid threshold multiplier (>= 1.0)
    probe_op(
        "ecg-peaks threshold_multiplier >= 1.0",
        lambda: bridge.ecg_peaks(np.ones(500), 100.0, {"threshold_multiplier": 1.0})
    )
    
    # 2. ECG Clean — unsupported method string
    probe_op(
        "ecg-clean unsupported_method string",
        lambda: bridge.ecg_clean(np.ones(100), 100.0, method="unsupported_method")
    )
    
    # 3. ECG Peaks — NaN input
    probe_op(
        "ecg-peaks NaN input",
        lambda: bridge.ecg_peaks([1.0, float("nan"), 2.0], 100.0)
    )
    
    # 4. ECG Peaks — +Inf input
    probe_op(
        "ecg-peaks +Inf input",
        lambda: bridge.ecg_peaks([1.0, float("inf"), 2.0], 100.0)
    )

    # 5. ECG Peaks — -Inf input
    probe_op(
        "ecg-peaks -Inf input",
        lambda: bridge.ecg_peaks([1.0, float("-inf"), 2.0], 100.0)
    )

    # 6. ECG Peaks — empty signal
    probe_op(
        "ecg-peaks empty signal",
        lambda: bridge.ecg_peaks([], 100.0)
    )

    # 7. ECG Peaks — zero sampling rate
    probe_op(
        "ecg-peaks fs=0",
        lambda: bridge.ecg_peaks(np.ones(500), 0.0)
    )

    # 8. ECG Peaks — negative sampling rate
    probe_op(
        "ecg-peaks fs=-100",
        lambda: bridge.ecg_peaks(np.ones(500), -100.0)
    )

    # 9. ECG Peaks — too short signal
    probe_op(
        "ecg-peaks short signal (5 samples)",
        lambda: bridge.ecg_peaks(np.ones(5), 100.0)
    )

    # 10. ECG Peaks — invalid lowcut >= highcut
    probe_op(
        "ecg-peaks lowcut >= highcut",
        lambda: bridge.ecg_peaks(np.ones(500), 100.0, {"lowcut": 20.0, "highcut": 10.0})
    )

    # 11. ECG Peaks — highcut >= Nyquist (e.g. 60Hz at fs=100Hz)
    probe_op(
        "ecg-peaks highcut >= Nyquist",
        lambda: bridge.ecg_peaks(np.ones(500), 100.0, {"highcut": 60.0})
    )

    # 12. Sample Entropy — all zeros
    probe_op(
        "sample-entropy constant zeros",
        lambda: bridge.sample_entropy(np.zeros(500))
    )

    # 13. Filter — unknown filter kind
    probe_op(
        "filter unknown kind",
        lambda: bridge.filter(np.ones(100), 100.0, kind="unknown_kind")
    )

    # 14. HRV Correct — invalid percent_threshold
    probe_op(
        "hrv-correct negative classify threshold",
        lambda: bridge.hrv_correct(rr_intervals_ms=[800.0, 800.0], classify_threshold=-0.1)
    )

    # Summarize findings
    panics = sum(1 for c in test_cases if c["panic"])
    unhandled = sum(1 for c in test_cases if c["status"] == "CRASH_OR_UNHANDLED")
    
    hygiene_summary = {
        "total_probes": len(test_cases),
        "total_panics": panics,
        "total_unhandled": unhandled,
        "clean_hygiene": (panics == 0 and unhandled == 0),
        "cases": test_cases,
    }
    
    json_path = out_dir / "api_hygiene.json"
    with open(json_path, "w") as f:
        json.dump(hygiene_summary, f, indent=2)
        
    print(f"\nAPI Hygiene Summary saved to {json_path}")
    print(f"Total Probes: {len(test_cases)}, Panics: {panics}, Unhandled: {unhandled} (Clean Hygiene: {hygiene_summary['clean_hygiene']})")

if __name__ == "__main__":
    run_phase12()
