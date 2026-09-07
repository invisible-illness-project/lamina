#!/usr/bin/env python3
"""Phase 1 & Phase 2: ECG Validation on MIT-BIH Arrhythmia Database (48 records)."""

import csv
import json
import numpy as np
from pathlib import Path

from validation.bridge import LaminaBridge
from validation.datasets.ecg import MitBihArrhythmiaAdapter, MITDB_RECORDS
from validation.metrics.events import peak_detection_metrics, match_events

def run_mitdb_validation():
    repo_root = Path(__file__).resolve().parents[1]
    out_dir = repo_root / "validation" / "revalidation-v2" / "results"
    out_dir.mkdir(parents=True, exist_ok=True)
    
    baseline_f1_map = {}
    baseline_csv = repo_root / "validation" / "revalidation" / "ecg" / "mitdb_records.csv"
    if baseline_csv.exists():
        with open(baseline_csv, "r") as f:
            reader = csv.DictReader(f)
            for row in reader:
                if row["record"] != "ALL":
                    baseline_f1_map[row["record"]] = float(row["baseline_f1"])

    bridge = LaminaBridge(repo_root=repo_root)
    adapter = MitBihArrhythmiaAdapter()
    
    per_record_rows = []
    
    tot_tp = 0
    tot_fp = 0
    tot_fn = 0
    f1_list = []
    
    print(f"Starting 48-record MIT-BIH validation...")
    for rec in adapter.iter_recordings(cache_dir=Path.home() / ".cache" / "lamina-validation" / "mitdb"):
        rec_id = rec.recording_id
        sig = next(iter(rec.signals.values()))
        fs = sig.sampling_rate
        ref_peaks = rec.references["ecg_peak_indices"]
        
        # Run bridge peak detection
        res = bridge.ecg_peaks(sig.samples, fs)
        det_peaks = np.asarray(res["peaks"], dtype=np.int64)
        
        # Match with 150 ms tolerance
        m = match_events(ref_peaks, det_peaks, fs, tolerance_sec=0.150)
        metrics = peak_detection_metrics(ref_peaks, det_peaks, fs, tolerance_sec=0.150)
        
        tp = metrics["tp"]
        fp = metrics["fp"]
        fn = metrics["fn"]
        prec = metrics["precision"] if metrics["precision"] is not None else 0.0
        rec_val = metrics["recall"] if metrics["recall"] is not None else 0.0
        f1 = metrics["f1"] if metrics["f1"] is not None else 0.0
        
        tot_tp += tp
        tot_fp += fp
        tot_fn += fn
        f1_list.append(f1)
        
        errs = m.timing_errors_sec
        mean_err = float(np.mean(np.abs(errs))) if errs.size > 0 else 0.0
        median_err = float(np.median(errs)) if errs.size > 0 else 0.0
        p95_err = float(np.percentile(np.abs(errs), 95)) if errs.size > 0 else 0.0
        
        base_f1 = baseline_f1_map.get(rec_id, 1.0)
        delta_f1 = f1 - base_f1
        
        per_record_rows.append({
            "record": rec_id,
            "TP": tp,
            "FP": fp,
            "FN": fn,
            "precision": round(prec, 6),
            "recall": round(rec_val, 6),
            "F1": round(f1, 6),
            "detected_peaks": len(det_peaks),
            "reference_peaks": len(ref_peaks),
            "baseline_F1": round(base_f1, 6),
            "delta_F1": round(delta_f1, 6),
            "mean_timing_error_sec": round(mean_err, 6),
            "median_timing_error_sec": round(median_err, 6),
            "p95_timing_error_sec": round(p95_err, 6),
        })
        print(f"Record {rec_id:4s}: TP={tp:4d}, FP={fp:4d}, FN={fn:4d}, Prec={prec:.4f}, Rec={rec_val:.4f}, F1={f1:.4f}, base_F1={base_f1:.4f}, delta_F1={delta_f1:+.4f}")

    mean_per_record_f1 = float(np.mean(f1_list))
    micro_f1 = (2 * tot_tp) / (2 * tot_tp + tot_fp + tot_fn) if (2 * tot_tp + tot_fp + tot_fn) > 0 else 0.0
    
    # Save per_record CSV
    fieldnames = [
        "record", "TP", "FP", "FN", "precision", "recall", "F1",
        "detected_peaks", "reference_peaks", "baseline_F1", "delta_F1",
        "mean_timing_error_sec", "median_timing_error_sec", "p95_timing_error_sec"
    ]
    csv_path = out_dir / "mit_bih_per_record.csv"
    with open(csv_path, "w", newline="") as f:
        writer = csv.DictWriter(f, fieldnames=fieldnames)
        writer.writeheader()
        writer.writerows(per_record_rows)
        
    rec228 = next(r for r in per_record_rows if r["record"] == "228")
    rec123 = next(r for r in per_record_rows if r["record"] == "123")
    rec232 = next(r for r in per_record_rows if r["record"] == "232")
    
    summary = {
        "records_validated": len(per_record_rows),
        "mean_per_record_f1": round(mean_per_record_f1, 6),
        "micro_global_f1": round(micro_f1, 6),
        "total_tp": tot_tp,
        "total_fp": tot_fp,
        "total_fn": tot_fn,
        "gates": {
            "mean_f1_gate_passed": mean_per_record_f1 >= 0.9820,
            "mean_f1_target": 0.9820,
            "total_fp_gate_passed": tot_fp <= 750,
            "total_fp_target": 750,
            "rec228_recall_gate_passed": rec228["recall"] >= 0.950,
            "rec228_recall_target": 0.950,
        },
        "record_228": rec228,
        "record_123": rec123,
        "record_232": rec232,
    }
    
    json_path = out_dir / "mit_bih_summary.json"
    with open(json_path, "w") as f:
        json.dump(summary, f, indent=2)
        
    print("\n--- Phase 1 & 2 Summary ---")
    print(f"Mean per-record F1: {mean_per_record_f1:.6f} (Gate >= 0.9820: {summary['gates']['mean_f1_gate_passed']})")
    print(f"Micro/Global F1:    {micro_f1:.6f}")
    print(f"Total FP:           {tot_fp} (Gate <= 750: {summary['gates']['total_fp_gate_passed']})")
    print(f"Total TP:           {tot_tp}")
    print(f"Total FN:           {tot_fn}")
    print(f"Record 228 Recall:  {rec228['recall']:.6f} (Gate >= 0.950: {summary['gates']['rec228_recall_gate_passed']}), F1: {rec228['F1']:.6f}, FP: {rec228['FP']}, FN: {rec228['FN']}")
    print(f"Record 123 F1:      {rec123['F1']:.6f} (Baseline: {rec123['baseline_F1']:.6f}, Delta: {rec123['delta_F1']:+.6f}), FP: {rec123['FP']}, FN: {rec123['FN']}")
    print(f"Record 232 F1:      {rec232['F1']:.6f} (Baseline: {rec232['baseline_F1']:.6f}, Delta: {rec232['delta_F1']:+.6f}), FP: {rec232['FP']}, FN: {rec232['FN']}")

if __name__ == "__main__":
    run_mitdb_validation()
