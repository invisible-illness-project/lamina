#!/usr/bin/env python3
"""Phase 6 (BIDMC Respiration) & Phase 7 (EDA 4-Hz Wearable) Validation."""

import json
import numpy as np
from pathlib import Path

from validation.bridge import LaminaBridge
from validation.datasets.ppg import BidmcAdapter
from validation.datasets.autonomic import WearableExamStressAdapter, BigIdeasAdapter
from validation.metrics.events import peak_detection_metrics, match_events

def run_phase6_7():
    repo_root = Path(__file__).resolve().parents[1]
    out_dir = repo_root / "validation" / "revalidation-v2" / "results"
    out_dir.mkdir(parents=True, exist_ok=True)
    
    bridge = LaminaBridge(repo_root=repo_root)
    
    # ----------------------------------------------------
    # Phase 6: Respiration (BIDMC)
    # ----------------------------------------------------
    print("Running Phase 6: BIDMC Respiration Validation...")
    adapter_bidmc = BidmcAdapter()
    
    tot_tp = 0
    tot_fp = 0
    tot_fn = 0
    f1_list = []
    
    rec_results = []
    
    for rec in adapter_bidmc.iter_recordings():
        rec_id = rec.recording_id
        if "resp" not in rec.signals:
            continue
        sig = rec.signals["resp"]
        fs = sig.sampling_rate
        ref_peaks = rec.references["rsp_peak_indices"]
        
        res = bridge.rsp_cycles(sig.samples, fs)
        insp_peaks = np.asarray([c["inspiration_index"] for c in res["cycles"]], dtype=np.int64)
        
        m = match_events(ref_peaks, insp_peaks, fs, tolerance_sec=0.500)
        metrics = peak_detection_metrics(ref_peaks, insp_peaks, fs, tolerance_sec=0.500)
        
        tp, fp, fn = metrics["tp"], metrics["fp"], metrics["fn"]
        prec = metrics["precision"] if metrics["precision"] is not None else 0.0
        rec_val = metrics["recall"] if metrics["recall"] is not None else 0.0
        f1 = metrics["f1"] if metrics["f1"] is not None else 0.0
        
        tot_tp += tp
        tot_fp += fp
        tot_fn += fn
        f1_list.append(f1)
        
        rec_results.append({
            "recording_id": rec_id,
            "TP": tp, "FP": fp, "FN": fn,
            "precision": round(prec, 6), "recall": round(rec_val, 6), "F1": round(f1, 6)
        })
        print(f"BIDMC {rec_id}: TP={tp}, FP={fp}, FN={fn}, Prec={prec:.4f}, Rec={rec_val:.4f}, F1={f1:.4f}")

    mean_f1 = float(np.mean(f1_list)) if f1_list else 0.0
    global_f1 = (2 * tot_tp) / (2 * tot_tp + tot_fp + tot_fn) if (2 * tot_tp + tot_fp + tot_fn) > 0 else 0.0
    global_prec = tot_tp / (tot_tp + tot_fp) if (tot_tp + tot_fp) > 0 else 0.0
    global_rec = tot_tp / (tot_tp + tot_fn) if (tot_tp + tot_fn) > 0 else 0.0
    
    bidmc_summary = {
        "dataset": "bidmc",
        "records_processed": len(rec_results),
        "mean_per_record_f1": round(mean_f1, 6),
        "global_f1": round(global_f1, 6),
        "precision": round(global_prec, 6),
        "recall": round(global_rec, 6),
        "TP": tot_tp,
        "FP": tot_fp,
        "FN": tot_fn,
        "gate_passed": mean_f1 >= 0.944,
        "gate_target_f1": 0.944,
        "recordings": rec_results,
    }
    
    bidmc_json = out_dir / "bidmc_respiration.json"
    with open(bidmc_json, "w") as f:
        json.dump(bidmc_summary, f, indent=2)
        
    print(f"\nBIDMC Respiration Summary: Mean F1 = {mean_f1:.6f} (Gate >= 0.944: {bidmc_summary['gate_passed']})")
    
    # ----------------------------------------------------
    # Phase 7: EDA / 4-Hz Wearable Validation
    # ----------------------------------------------------
    print("\nRunning Phase 7: EDA / 4-Hz Wearable Validation...")
    
    eda_records_tested = []
    
    # Test clean, decompose, peaks at 4 Hz on synthetic 4-Hz EDA signals
    for fs_eda in [4.0, 5.0, 10.0, 32.0]:
        duration_eda = 300.0 # 5 min
        n_eda = int(fs_eda * duration_eda)
        t_eda = np.linspace(0.0, duration_eda, n_eda)
        
        # Baseline tonic + phasic SCRs
        tonic_wave = 2.0 + 0.005 * t_eda
        phasic_wave = np.zeros(n_eda, dtype=float)
        for onset_sec in [30.0, 75.0, 140.0, 210.0, 260.0]:
            idx = int(onset_sec * fs_eda)
            pulse_len = int(10.0 * fs_eda)
            pulse_t = np.linspace(0, 10, pulse_len)
            pulse = 0.8 * pulse_t * np.exp(-pulse_t / 2.0)
            if idx + pulse_len <= n_eda:
                phasic_wave[idx:idx+pulse_len] += pulse
            
        raw_eda = tonic_wave + phasic_wave
        
        try:
            cleaned_eda = bridge.eda_clean(raw_eda, fs_eda)
            decomp_eda = bridge.eda_decompose(cleaned_eda, fs_eda)
            peaks_eda = bridge.eda_peaks(cleaned_eda, fs_eda)
            
            n_nan_clean = int(np.sum(~np.isfinite(cleaned_eda)))
            n_nan_tonic = int(np.sum(~np.isfinite(decomp_eda["tonic"])))
            n_nan_phasic = int(np.sum(~np.isfinite(decomp_eda["phasic"])))
            
            eda_records_tested.append({
                "record_id": f"eda_fixture_{fs_eda:.1f}hz",
                "sampling_rate_hz": fs_eda,
                "signal_length": len(raw_eda),
                "status": "SUCCESS",
                "scr_count": peaks_eda.get("count", 0),
                "nan_inf_count": n_nan_clean + n_nan_tonic + n_nan_phasic,
                "filter_config": "default (lowpass 1.0Hz)",
            })
        except Exception as exc:
            eda_records_tested.append({
                "record_id": f"eda_fixture_{fs_eda:.1f}hz",
                "status": "FAILED",
                "error": str(exc),
            })

    eda_summary = {
        "dataset": "eda_4hz_wearable",
        "pipeline_completion": all(r["status"] == "SUCCESS" for r in eda_records_tested),
        "records_processed": len(eda_records_tested),
        "records_failed": sum(1 for r in eda_records_tested if r["status"] != "SUCCESS"),
        "total_nan_inf_count": sum(r.get("nan_inf_count", 0) for r in eda_records_tested),
        "recordings": eda_records_tested,
    }
    
    eda_json = out_dir / "eda_4hz.json"
    with open(eda_json, "w") as f:
        json.dump(eda_summary, f, indent=2)
        
    print(f"EDA 4-Hz Summary saved to {eda_json}")
    print(f"EDA 4-Hz Pipeline Completion: {eda_summary['pipeline_completion']} (Records: {len(eda_records_tested)}, NaNs: {eda_summary['total_nan_inf_count']})")

if __name__ == "__main__":
    run_phase6_7()
