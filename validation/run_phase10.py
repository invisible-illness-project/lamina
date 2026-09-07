#!/usr/bin/env python3
"""Phase 10: rPPG Polarity and Algorithm Validation."""

import csv
import json
import numpy as np
from pathlib import Path

from validation.bridge import LaminaBridge

def run_phase10():
    repo_root = Path(__file__).resolve().parents[1]
    out_dir = repo_root / "validation" / "revalidation-v2" / "results"
    out_dir.mkdir(parents=True, exist_ok=True)
    
    bridge = LaminaBridge(repo_root=repo_root)
    
    print("Running Phase 10: rPPG Validation...")
    
    # Generate synthetic optical RGB signal (10 s @ 30 fps)
    fps = 30.0
    duration = 10.0
    n = int(fps * duration)
    timestamps = np.linspace(0.0, duration, n)
    
    # Pulsatile signal (1.2 Hz = 72 bpm)
    pulse = np.sin(2.0 * np.pi * 1.2 * timestamps)
    
    # Base RGB levels with pulsatile absorption modulation
    red = 120.0 + 0.5 * pulse
    green = 150.0 + 2.0 * pulse
    blue = 100.0 - 0.3 * pulse
    
    algorithms = ["green", "pos", "chrom"]
    polarities = ["normal", "inverted", "auto"]
    
    rppg_rows = []
    
    for algo in algorithms:
        for pol in polarities:
            try:
                res = bridge.rppg_polarity(
                    timestamps_sec=timestamps,
                    red=red, green=green, blue=blue,
                    polarity=pol,
                    config={"algorithm": algo}
                )
                waveform = np.asarray(res.get("waveform", []), dtype=float)
                flipped = res.get("flipped", False)
                pol_resolved = res.get("polarity_resolved", "")
                
                # Peak detection on extracted BVP waveform
                peaks_res = bridge.ppg_peaks(waveform, fs=fps)
                det_peaks = peaks_res.get("peaks", [])
                
                rppg_rows.append({
                    "algorithm": algo,
                    "requested_polarity": pol,
                    "resolved_polarity": pol_resolved,
                    "waveform_flipped": flipped,
                    "output_length": len(waveform),
                    "detected_bvp_peaks": len(det_peaks),
                    "mean_bvp_val": round(float(np.mean(waveform)), 6) if len(waveform) > 0 else 0.0,
                    "std_bvp_val": round(float(np.std(waveform)), 6) if len(waveform) > 0 else 0.0,
                })
                print(f"rPPG Algo={algo:5s}, Polarity={pol:8s} -> Resolved={pol_resolved:8s}, Flipped={flipped}, BVP Peaks={len(det_peaks)}")
            except Exception as exc:
                rppg_rows.append({
                    "algorithm": algo,
                    "requested_polarity": pol,
                    "error": str(exc),
                })
                print(f"rPPG Algo={algo:5s}, Polarity={pol:8s} -> Error: {exc}")

    # Write rppg_results.csv
    csv_path = out_dir / "rppg_results.csv"
    with open(csv_path, "w", newline="") as f:
        writer = csv.DictWriter(f, fieldnames=list(rppg_rows[0].keys()))
        writer.writeheader()
        writer.writerows(rppg_rows)
    print(f"rPPG results saved to {csv_path}")

if __name__ == "__main__":
    run_phase10()
