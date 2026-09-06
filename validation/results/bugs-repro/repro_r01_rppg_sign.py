#!/usr/bin/env python3
"""Synthetic probe for BUG-R01: inconsistent output sign convention across
rppg algorithms (green/pos vs chrom) and the sign-fragile composition with
ppg-peaks.

Simulates the physics: skin reflectance intensity DECREASES as blood volume
increases. Ground truth = blood-volume pulse (BVP). Camera RGB = anti-phase
intensity. No dataset download required.
"""
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parents[3]))
from validation.bridge import LaminaBridge, LaminaBridgeError


def hr_from_peaks(peaks, fs):
    peaks = np.asarray(peaks)
    if len(peaks) < 2:
        return float("nan")
    rr = np.diff(peaks) / fs
    return 60.0 / np.mean(rr)


def main():
    b = LaminaBridge(auto_build=False)
    print("lamina_version:", b.version())

    fps = 30.0
    dur = 20.0
    n = int(fps * dur)
    t = np.arange(n) / fps
    hr_hz = 1.25  # 75 bpm, inside default signal_band_hz (0.75, 2.5)
    bvp = np.sin(2 * np.pi * hr_hz * t)  # ground-truth blood-volume pulse

    rng = np.random.RandomState(0)
    # Intensity anti-phase with blood volume; green carries the strongest
    # pulsatile component (typical for 530 nm absorption).
    red = 180.0 - 0.6 * bvp + 0.05 * rng.randn(n)
    green = 150.0 - 1.5 * bvp + 0.05 * rng.randn(n)
    blue = 120.0 - 0.4 * bvp + 0.05 * rng.randn(n)

    print(f"\nsynthetic video: {fps} fps, {dur} s, GT HR {hr_hz*60:.1f} bpm; "
          "RGB intensity anti-phase with blood volume\n")

    print(f"{'algorithm':8s} {'r(wave, GT BVP)':>16s} {'HR as-is':>9s} {'HR flipped':>11s} {'n_pk as-is':>11s} {'n_pk flip':>10s}")
    results = {}
    for algo in ["green", "chrom", "pos"]:
        try:
            r = b.rppg_algorithm(t, red, green, blue, config={"algorithm": algo})
        except LaminaBridgeError as e:
            print(f"{algo:8s} ERROR: {e}")
            continue
        wave = np.asarray(r["waveform"], dtype=float)
        wfs = float(r.get("sampling_rate_hz", fps))
        # correlate over common length
        m = min(len(wave), n)
        corr = np.corrcoef(wave[:m], bvp[:m])[0, 1]
        pk_asis = b.ppg_peaks(wave, wfs)["peaks"]
        pk_flip = b.ppg_peaks(-wave, wfs)["peaks"]
        hr_a = hr_from_peaks(pk_asis, wfs)
        hr_f = hr_from_peaks(pk_flip, wfs)
        results[algo] = (corr, hr_a, hr_f)
        print(f"{algo:8s} {corr:16.3f} {hr_a:9.2f} {hr_f:11.2f} {len(pk_asis):11d} {len(pk_flip):10d}")

    print(f"\nGT HR = {hr_hz*60:.1f} bpm")
    print("\nInterpretation: r<0 means the emitted waveform is in intensity phase")
    print("(inverted vs blood volume); r>0 means BVP phase. A sign-sensitive")
    print("downstream detector (ppg-peaks/Elgendi) then gives different HR for")
    print("wave vs -wave.")


if __name__ == "__main__":
    main()
