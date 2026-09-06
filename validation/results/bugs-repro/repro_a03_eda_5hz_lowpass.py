#!/usr/bin/env python3
"""Repro for BUG-A03: eda_clean/eda_peaks hardcode a 5 Hz lowpass cutoff ->
any fs <= 10 Hz fails (blocks Empatica E4 EDA @ 4 Hz). eda_decompose unaffected.

Synthetic signal only.
"""
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parents[3]))
from validation.bridge import LaminaBridge, LaminaBridgeError


def synthetic_eda(fs, dur=60.0, seed=1):
    rng = np.random.RandomState(seed)
    n = int(fs * dur)
    t = np.arange(n) / fs
    sig = 2.0 + 0.5 * np.sin(2 * np.pi * 0.01 * t)  # slow tonic drift
    sig += np.cumsum(rng.randn(n)) * 0.001
    return sig


def main():
    b = LaminaBridge(auto_build=False)
    print("lamina_version:", b.version())

    print("\neda_clean fs sweep:")
    for fs in [4.0, 8.0, 10.0, 16.0, 32.0, 100.0]:
        sig = synthetic_eda(fs)
        try:
            out = b.eda_clean(sig, fs)
            print(f"  fs={fs:>6}: ok, len={len(out)}")
        except LaminaBridgeError as e:
            print(f"  fs={fs:>6}: ERROR: {e}")

    print("\neda_peaks fs sweep (cleans internally):")
    for fs in [4.0, 10.0, 16.0]:
        sig = synthetic_eda(fs)
        try:
            r = b.eda_peaks(sig, fs)
            print(f"  fs={fs:>6}: ok, peaks={r.get('peaks')}")
        except LaminaBridgeError as e:
            print(f"  fs={fs:>6}: ERROR: {e}")

    print("\neda_decompose at fs=4 (expected NOT to fail):")
    sig = synthetic_eda(4.0)
    try:
        r = b.eda_decompose(sig, 4.0)
        print(f"  ok: tonic len={len(r['tonic'])}, phasic len={len(r['phasic'])}")
    except LaminaBridgeError as e:
        print(f"  ERROR: {e}")


if __name__ == "__main__":
    main()
