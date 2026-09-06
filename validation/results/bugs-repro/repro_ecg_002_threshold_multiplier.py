#!/usr/bin/env python3
"""Repro for BUG-ECG-002: ecg-peaks fails with a spurious "non-finite values"
error whenever config threshold_multiplier >= 1.0, on provably finite input.

Synthetic signal only. Sweeps threshold_multiplier across the 1.0 boundary.
"""
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parents[3]))
from validation.bridge import LaminaBridge, LaminaBridgeError

from repro_ecg_001_method_ignored import synthetic_ecg


def main():
    b = LaminaBridge(auto_build=False)
    print("lamina_version:", b.version())
    fs = 360.0
    sig = synthetic_ecg(fs, dur=30.0)
    print(f"signal: n={len(sig)}, all finite: {bool(np.isfinite(sig).all())}")
    print()

    # Baseline: default config works
    r = b.ecg_peaks(sig, fs)
    print(f"default config -> {len(r['peaks'])} peaks")
    print()

    print("threshold_multiplier sweep:")
    for tm in [0.1, 0.25, 0.5, 0.9, 0.99, 0.9999, 1.0, 1.0000001, 1.1, 1.5, 2.0, 3.0]:
        try:
            r = b.ecg_peaks(sig, fs, config={"threshold_multiplier": tm})
            print(f"  {tm:>10}: ok, {len(r['peaks'])} peaks")
        except LaminaBridgeError as e:
            print(f"  {tm:>10}: ERROR: {e}")

    print()
    print("Other config fields at threshold_multiplier=0.25 (sanity: they work):")
    for cfg in [
        {"refractory_period_sec": 0.3},
        {"integration_window_sec": 0.08},
        {"lowcut": 3.0, "highcut": 20.0},
        {"searchback": False},
        {"filter_order": 3},
    ]:
        try:
            r = b.ecg_peaks(sig, fs, config=cfg)
            print(f"  {cfg}: ok, {len(r['peaks'])} peaks")
        except LaminaBridgeError as e:
            print(f"  {cfg}: ERROR: {e}")


if __name__ == "__main__":
    main()
