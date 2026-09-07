#!/usr/bin/env python3
"""Phase 3, Phase 4, Phase 5: ECG Mechanism, 6-Case Regression Matrix, Wrist s6 Validation."""

import csv
import json
import math
import numpy as np
from pathlib import Path
import wfdb

from validation.bridge import LaminaBridge
from validation.metrics.events import match_events, peak_detection_metrics

def make_qrs(amp: float, width_sec: float, fs: float) -> np.ndarray:
    length = int(width_sec * fs)
    pulse = np.zeros(length, dtype=float)
    half_len = length / 2.0
    sig_scale = length / 4.0
    for i in range(length):
        t = (i - half_len) / sig_scale
        pulse[i] = amp * math.exp(-0.5 * t * t)
    return pulse

def run_phase3_4_5():
    repo_root = Path(__file__).resolve().parents[1]
    out_dir = repo_root / "validation" / "revalidation-v2" / "results"
    out_dir.mkdir(parents=True, exist_ok=True)
    
    bridge = LaminaBridge(repo_root=repo_root)
    
    # ----------------------------------------------------
    # Phase 4: Six-Case ECG Regression Matrix
    # ----------------------------------------------------
    fs = 250.0
    duration = 10.0
    n = int(fs * duration)
    tol_samples = int(round(0.150 * fs))
    
    matrix_rows = []
    
    # Helper to run and evaluate a synthetic case
    def eval_case(case_num, desc, sig, exp_indices):
        res = bridge.ecg_peaks(sig, fs)
        det_peaks = np.asarray(res["peaks"], dtype=np.int64)
        m = match_events(exp_indices, det_peaks, fs, tolerance_sec=0.150)
        metrics = peak_detection_metrics(exp_indices, det_peaks, fs, tolerance_sec=0.150)
        
        tp = metrics["tp"]
        fp = metrics["fp"]
        fn = metrics["fn"]
        prec = metrics["precision"] if metrics["precision"] is not None else 0.0
        rec = metrics["recall"] if metrics["recall"] is not None else 0.0
        f1 = metrics["f1"] if metrics["f1"] is not None else 0.0
        align_ok = (tp == len(exp_indices)) and (fp <= (1 if case_num == 6 else 0))
        
        row = {
            "case": f"Case {case_num}",
            "description": desc,
            "expected_peaks": len(exp_indices),
            "detected_peaks": len(det_peaks),
            "TP": tp,
            "FP": fp,
            "FN": fn,
            "precision": round(prec, 6),
            "recall": round(rec, 6),
            "F1": round(f1, 6),
            "peak_alignment_ok": align_ok,
        }
        matrix_rows.append(row)
        print(f"Case {case_num} ({desc}): TP={tp}, FP={fp}, FN={fn}, Prec={prec:.4f}, Rec={rec:.4f}, F1={f1:.4f}, AlignOK={align_ok}")
        return det_peaks
        
    # Case 1: Normal QRS
    sig1 = np.zeros(n, dtype=float)
    exp1 = []
    for k in range(1, 9):
        pos = int(k * 1.0 * fs)
        exp1.append(pos)
        qrs = make_qrs(1.0, 0.08, fs)
        sig1[pos:pos+len(qrs)] += qrs
    eval_case(1, "Normal QRS", sig1, exp1)
    
    # Case 2: 3.5:1 PVC amplitude disparity
    sig2 = np.zeros(n, dtype=float)
    exp2 = []
    for k in range(1, 9):
        pos = int(k * 1.0 * fs)
        exp2.append(pos)
        amp = 3.5 if (k % 3 == 0) else 1.0
        qrs = make_qrs(amp, 0.08, fs)
        sig2[pos:pos+len(qrs)] += qrs
    eval_case(2, "3.5:1 PVC amplitude disparity", sig2, exp2)
    
    # Case 3: Continuous bigeminy
    sig3 = np.zeros(n, dtype=float)
    exp3 = []
    for k in range(6):
        pos_norm = int((1.0 + k * 1.4) * fs)
        pos_pvc = int((1.5 + k * 1.4) * fs)
        exp3.append(pos_norm)
        exp3.append(pos_pvc)
        qrs_norm = make_qrs(1.0, 0.08, fs)
        qrs_pvc = make_qrs(2.5, 0.12, fs)
        sig3[pos_norm:pos_norm+len(qrs_norm)] += qrs_norm
        sig3[pos_pvc:pos_pvc+len(qrs_pvc)] += qrs_pvc
    exp3.sort()
    eval_case(3, "Continuous bigeminy", sig3, exp3)
    
    # Case 4: Narrow / biphasic QRS
    sig4 = np.zeros(n, dtype=float)
    exp4 = []
    for k in range(1, 9):
        pos = int(k * 1.0 * fs)
        exp4.append(pos)
        for i in range(10):
            if pos + i < n:
                sig4[pos + i] = 1.0 * (i / 5.0)
            if pos + 10 + i < n:
                sig4[pos + 10 + i] = -(1.0 - i / 5.0)
    eval_case(4, "Narrow/biphasic QRS", sig4, exp4)
    
    # Case 5: Paced ECG
    sig5 = sig1.copy()
    for exp_p in exp1:
        spk_pos = exp_p - 10
        if spk_pos > 0:
            sig5[spk_pos] = 10.0  # Sharp pacing spike
    eval_case(5, "Paced ECG", sig5, exp1)
    
    # Case 6: EMG noise burst
    sig6 = sig1.copy()
    for i in range(int(3.0 * fs), int(4.0 * fs)):
        if i < n:
            sig6[i] += 0.2 * math.sin(i * 50.0)
    eval_case(6, "EMG noise burst", sig6, exp1)
    
    # Write ecg_regression_matrix.csv
    matrix_csv = out_dir / "ecg_regression_matrix.csv"
    with open(matrix_csv, "w", newline="") as f:
        writer = csv.DictWriter(f, fieldnames=list(matrix_rows[0].keys()))
        writer.writeheader()
        writer.writerows(matrix_rows)
        
    print(f"Six-Case Regression Matrix saved to {matrix_csv}")
    
    # ----------------------------------------------------
    # Phase 5: Wrist ECG / s6 Regression
    # ----------------------------------------------------
    print("\nRunning Phase 5: Wrist ECG s6 Revalidation...")
    try:
        rec = wfdb.rdrecord("s6_low_resistance_bike", pn_dir="wrist")
        ann = wfdb.rdann("s6_low_resistance_bike", "atr", pn_dir="wrist")
    except Exception as exc:
        print(f"Failed to fetch wrist record s6: {exc}")
        rec, ann = None, None
        
    if rec is not None and ann is not None:
        fs_s6 = float(rec.fs)
        sig_s6 = rec.p_signal[:, 0]  # chest_ecg channel
        ref_s6 = np.asarray(ann.sample, dtype=np.int64)
        
        res_s6 = bridge.ecg_peaks(sig_s6, fs_s6)
        det_s6 = np.asarray(res_s6["peaks"], dtype=np.int64)
        
        m_s6 = match_events(ref_s6, det_s6, fs_s6, tolerance_sec=0.150)
        metrics_s6 = peak_detection_metrics(ref_s6, det_s6, fs_s6, tolerance_sec=0.150)
        
        # Calculate detection gaps
        det_sorted = np.sort(det_s6)
        gaps_sec = np.diff(det_sorted) / fs_s6 if len(det_sorted) > 1 else np.array([0.0])
        longest_gap_sec = float(np.max(gaps_sec)) if gaps_sec.size > 0 else 0.0
        
        # Bin evaluation (9 bins across 280 s)
        bin_edges = np.linspace(0.0, 280.0, 10)
        ref_times = ref_s6 / fs_s6
        det_times = det_s6 / fs_s6
        
        bin_counts = []
        for b_idx in range(9):
            t0, t1 = bin_edges[b_idx], bin_edges[b_idx+1]
            n_ref_bin = int(np.sum((ref_times >= t0) & (ref_times < t1)))
            n_det_bin = int(np.sum((det_times >= t0) & (det_times < t1)))
            bin_counts.append({
                "bin_index": b_idx,
                "t_start_sec": round(t0, 2),
                "t_end_sec": round(t1, 2),
                "annotations": n_ref_bin,
                "detections": n_det_bin,
            })
            
        wrist_s6_summary = {
            "record": "s6_low_resistance_bike",
            "total_reference_peaks": len(ref_s6),
            "detected_peaks": len(det_s6),
            "TP": metrics_s6["tp"],
            "FP": metrics_s6["fp"],
            "FN": metrics_s6["fn"],
            "precision": round(metrics_s6["precision"], 6) if metrics_s6["precision"] is not None else 0.0,
            "recall": round(metrics_s6["recall"], 6) if metrics_s6["recall"] is not None else 0.0,
            "F1": round(metrics_s6["f1"], 6) if metrics_s6["f1"] is not None else 0.0,
            "longest_detection_gap_sec": round(longest_gap_sec, 6),
            "blackout_90s_resolved": longest_gap_sec < 5.0,
            "bin_counts": bin_counts,
        }
        
        wrist_json = out_dir / "wrist_s6.json"
        with open(wrist_json, "w") as f:
            json.dump(wrist_s6_summary, f, indent=2)
            
        print(f"Wrist s6 summary saved to {wrist_json}")
        print(f"Wrist s6 F1: {wrist_s6_summary['F1']:.4f}, Rec: {wrist_s6_summary['recall']:.4f}, Prec: {wrist_s6_summary['precision']:.4f}, Longest Gap: {longest_gap_sec:.2f}s (Blackout resolved: {wrist_s6_summary['blackout_90s_resolved']})")

if __name__ == "__main__":
    run_phase3_4_5()
