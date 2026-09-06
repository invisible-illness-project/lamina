#!/usr/bin/env python3
"""Synthetic probe for BUG-ECG-003: default ecg-peaks adaptive threshold
under-detects low-amplitude normal beats in the presence of tall PVCs.

Constructs an ECG-like signal with small normal beats (amp 0.7) interleaved
with tall wide PVCs (amp 2.2) at the same mean rate, matching the amplitude
disparity reported for mitdb/228. No dataset download required.
"""
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parents[3]))
from validation.bridge import LaminaBridge, LaminaBridgeError


def build_signal(fs=360.0, dur=120.0, seed=0):
    """Normal beats at 75 bpm; every 4th beat is replaced by a tall PVC."""
    rng = np.random.RandomState(seed)
    n = int(fs * dur)
    sig = 0.02 * rng.randn(n)
    t = np.arange(n) / fs
    sig += 0.05 * np.sin(2 * np.pi * 0.3 * t)
    true_beats = []
    beat_times = np.arange(1.0, dur - 0.5, 0.8)  # 75 bpm
    for i, bt in enumerate(beat_times):
        c = int(bt * fs)
        idx = np.arange(n) - c
        if i % 4 == 3:
            # PVC: tall (2.2), wide, no P wave
            sig += 2.2 * np.exp(-0.5 * (idx / (0.035 * fs)) ** 2)
            sig -= 0.8 * np.exp(-0.5 * ((idx - 0.09 * fs) / (0.05 * fs)) ** 2)
        else:
            # normal: small R (0.7)
            sig += 0.7 * np.exp(-0.5 * (idx / (0.018 * fs)) ** 2)
            sig -= 0.10 * np.exp(-0.5 * ((idx + 0.06 * fs) / (0.025 * fs)) ** 2)
        true_beats.append(c)
    return sig, np.array(true_beats)


def match(peaks, truth, fs, tol_sec=0.150):
    peaks = np.asarray(peaks)
    tol = tol_sec * fs
    tp = 0
    used = set()
    for tb in truth:
        if len(peaks) == 0:
            break
        d = np.abs(peaks - tb)
        j = int(np.argmin(d))
        if d[j] <= tol and j not in used:
            tp += 1
            used.add(j)
    rec = tp / max(len(truth), 1)
    prec = tp / max(len(peaks), 1)
    f1 = 2 * prec * rec / max(prec + rec, 1e-12)
    return rec, prec, f1


def main():
    b = LaminaBridge(auto_build=False)
    print("lamina_version:", b.version())
    fs = 360.0
    sig, truth = build_signal(fs)
    print(f"signal: n={len(sig)}, true beats={len(truth)} (25% tall PVCs, amp ratio ~3:1)")

    for label, cfg in [("default", None),
                       ("threshold_multiplier=0.1", {"threshold_multiplier": 0.1}),
                       ("threshold_multiplier=0.5", {"threshold_multiplier": 0.5})]:
        try:
            r = b.ecg_peaks(sig, fs, config=cfg)
            rec, prec, f1 = match(r["peaks"], truth, fs)
            print(f"  {label:30s}: detected={len(r['peaks']):4d}  "
                  f"recall={rec:.3f} precision={prec:.3f} F1={f1:.3f}")
        except LaminaBridgeError as e:
            print(f"  {label:30s}: ERROR: {e}")


if __name__ == "__main__":
    main()
