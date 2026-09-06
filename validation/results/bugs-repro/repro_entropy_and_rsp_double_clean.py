#!/usr/bin/env python3
"""Repro for API-INVENTORY sharp edges:
 (a) sample_entropy returns Ok(+inf) on zero template matches.
 (b) rsp_cycles_config re-cleans its input internally (double filtering when
     the caller pre-cleans).

Synthetic signals only, via the JSON bridge.
"""
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parents[3]))
from validation.bridge import LaminaBridge, LaminaBridgeError


def main():
    b = LaminaBridge(auto_build=False)
    print("lamina_version:", b.version())

    print("\n=== (a) sample_entropy on signals with no template matches ===")
    # Constant signal: no pair of length-m templates can match at tolerance
    # r > 0 within the self-match exclusion, and ramp signal likewise has
    # zero matches for default r.
    cases = {
        "constant(1.0) x 200": np.ones(200),
        "linear ramp x 200": np.linspace(0.0, 1.0, 200),
        "random walk x 500 (control)": np.cumsum(np.random.RandomState(0).randn(500)),
    }
    for name, sig in cases.items():
        try:
            r = b.sample_entropy(sig)
            print(f"  {name:32s}: {r}")
        except LaminaBridgeError as e:
            print(f"  {name:32s}: ERROR: {e}")

    print("\n=== (b) rsp_cycles_config double-cleaning probe ===")
    # Synthetic respiration: 0.25 Hz (15 brpm) + drift + noise, fs=25 Hz.
    fs = 25.0
    dur = 180.0
    n = int(fs * dur)
    t = np.arange(n) / fs
    rng = np.random.RandomState(7)
    raw = np.sin(2 * np.pi * 0.25 * t) + 0.3 * np.sin(2 * np.pi * 0.02 * t) + 0.05 * rng.randn(n)

    pre_cleaned = b.rsp_clean(raw, fs)
    r_raw = b.rsp_cycles(raw, fs)
    r_pre = b.rsp_cycles(pre_cleaned, fs)

    def summarize(r, label):
        cycles = r["cycles"]
        insp = [c["inspiration_index"] for c in cycles]
        rates = [c["respiratory_rate_bpm"] for c in cycles]
        print(f"  {label}: {len(cycles)} cycles, "
              f"median rate {np.median(rates) if rates else float('nan'):.2f} brpm, "
              f"first inspirations {insp[:6]}")
        return insp

    print("If rsp_cycles_config did NOT re-clean internally, pre-cleaned input "
          "would pass through the pipeline unchanged and (near-)identical "
          "cycles would result only if cleaning were idempotent (it is not: "
          "a 3rd-order zero-phase bandpass applied twice attenuates band edges).")
    insp_raw = summarize(r_raw, "rsp_cycles(raw)")
    insp_pre = summarize(r_pre, "rsp_cycles(rsp_clean(raw))")

    same = insp_raw == insp_pre
    print(f"\n  identical inspiration indices: {same}")
    n_diff = len(set(insp_raw) ^ set(insp_pre))
    print(f"  symmetric-difference size: {n_diff}")
    if not same:
        print("  -> outputs differ: input is processed through the internal "
              "cleaning stage again (double filtering when caller pre-cleans).")
    # Extra check: triple application drift
    r_twice = b.rsp_cycles(b.rsp_clean(pre_cleaned, fs), fs)
    summarize(r_twice, "rsp_cycles(rsp_clean^2(raw))")


if __name__ == "__main__":
    main()
