#!/usr/bin/env python3
"""Repro for BUG-ECG-001: ecg_clean silently ignores its `method` argument.

Synthetic signal only (no dataset download). Runs three method strings —
including one bogus value — and a no-method default call, then compares the
outputs pairwise.
"""
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parents[3]))
from validation.bridge import LaminaBridge, LaminaBridgeError


def synthetic_ecg(fs=360.0, dur=30.0, hr_hz=1.2, seed=0):
    rng = np.random.RandomState(seed)
    n = int(fs * dur)
    t = np.arange(n) / fs
    sig = 0.05 * np.sin(2 * np.pi * 0.3 * t)  # baseline wander
    sig += 0.02 * rng.randn(n)
    for beat_t in np.arange(0.5, dur, 1.0 / hr_hz):
        c = int(beat_t * fs)
        idx = np.arange(n) - c
        sig += 1.0 * np.exp(-0.5 * (idx / (0.02 * fs)) ** 2)  # R spike
        sig -= 0.15 * np.exp(-0.5 * ((idx + 0.08 * fs) / (0.03 * fs)) ** 2)  # S
    return sig


def main():
    b = LaminaBridge(auto_build=False)
    print("lamina_version:", b.version())
    fs = 360.0
    sig = synthetic_ecg(fs)
    assert np.isfinite(sig).all()

    out_default = b.ecg_clean(sig, fs)
    outs = {}
    for m in ["none", "neurokit", "pantompkins", "biosppy", "nonexistent-method-xyz"]:
        try:
            outs[m] = b.ecg_clean(sig, fs, method=m)
            print(f"method={m!r:25s} -> ok, len={len(outs[m])}")
        except LaminaBridgeError as e:
            print(f"method={m!r:25s} -> ERROR: {e}")
            outs[m] = None

    print()
    print("max abs diff vs default call (no method):")
    for m, o in outs.items():
        if o is not None:
            print(f"  {m!r:25s}: {np.max(np.abs(o - out_default)):.3e}")

    print()
    print("max abs diff between named methods:")
    keys = [k for k, v in outs.items() if v is not None]
    for i in range(len(keys)):
        for j in range(i + 1, len(keys)):
            d = np.max(np.abs(outs[keys[i]] - outs[keys[j]]))
            print(f"  {keys[i]!r} vs {keys[j]!r}: {d:.3e}")

    changed = np.max(np.abs(out_default - sig))
    print(f"\ncleaning does modify the signal (max abs diff raw vs cleaned: {changed:.3e})")

    all_same = all(
        np.array_equal(outs[m], out_default) for m in keys
    )
    print(f"\nRESULT: all method outputs byte-identical to default: {all_same}")
    if all_same:
        print("CONFIRMED: `method` argument is silently ignored (bogus string accepted, no error).")
    else:
        print("NOT REPRODUCED: outputs differ by method.")


if __name__ == "__main__":
    main()
