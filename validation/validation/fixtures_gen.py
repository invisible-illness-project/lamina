"""Deterministic fixture generator (SPEC §8).

Creates tiny synthetic signals with known ground truth under
``validation/fixtures/``. Run directly:

    python -m validation.fixtures_gen

Everything is seeded and deterministic; total output < 200 KB so the fixtures
are committed to git.
"""

from __future__ import annotations

from pathlib import Path

import numpy as np

FIXTURES_DIR = Path(__file__).resolve().parents[1] / "fixtures"
SEED = 20240601


def synth_ecg(fs: float = 360.0, duration_sec: float = 10.0, hr_bpm: float = 60.0):
    """ECG-like signal: gaussian QRS + P/T waves on a known peak train."""
    rng = np.random.default_rng(SEED)
    n = int(fs * duration_sec)
    idx = np.arange(n)
    t = idx / fs
    period = fs * 60.0 / hr_bpm
    true_peaks = np.arange(int(period), n, int(round(period)))
    sig = np.zeros(n)
    for p in true_peaks:
        sig += np.exp(-0.5 * ((idx - p) / (0.018 * fs)) ** 2)                # R
        sig -= 0.18 * np.exp(-0.5 * ((idx - p + 0.020 * fs) / (0.012 * fs)) ** 2)  # Q
        sig -= 0.22 * np.exp(-0.5 * ((idx - p - 0.025 * fs) / (0.012 * fs)) ** 2)  # S
        sig += 0.25 * np.exp(-0.5 * ((idx - p - 0.28 * fs) / (0.05 * fs)) ** 2)    # T
        sig += 0.12 * np.exp(-0.5 * ((idx - p + 0.20 * fs) / (0.04 * fs)) ** 2)    # P
    sig += 0.03 * rng.standard_normal(n)
    sig += 0.05 * np.sin(2 * np.pi * 0.33 * t)  # mild baseline wander
    return sig, true_peaks, fs


def synth_ppg(fs: float = 100.0, duration_sec: float = 12.0, hr_bpm: float = 75.0):
    """PPG-like pulse wave: asymmetric systolic peaks on a known peak train."""
    rng = np.random.default_rng(SEED + 1)
    n = int(fs * duration_sec)
    idx = np.arange(n)
    t = idx / fs
    period = fs * 60.0 / hr_bpm
    true_peaks = np.arange(int(period), n, int(round(period)))
    sig = np.zeros(n)
    for p in true_peaks:
        sig += np.exp(-0.5 * ((idx - p) / (0.09 * fs)) ** 2)
        sig += 0.3 * np.exp(-0.5 * ((idx - p - 0.30 * fs) / (0.10 * fs)) ** 2)  # dicrotic
    sig += 0.10 * np.sin(2 * np.pi * 0.25 * t)  # respiratory modulation
    sig += 0.01 * rng.standard_normal(n)
    return sig, true_peaks, fs


def synth_rsp(fs: float = 100.0, duration_sec: float = 30.0, brpm: float = 15.0):
    """Respiration-like sinusoid; true inspiratory peaks."""
    rng = np.random.default_rng(SEED + 2)
    n = int(fs * duration_sec)
    t = np.arange(n) / fs
    f = brpm / 60.0
    sig = np.sin(2 * np.pi * f * t) + 0.02 * rng.standard_normal(n)
    period = fs / f
    true_peaks = np.arange(int(0.25 * period), n, int(round(period)))  # sine maxima
    return sig, true_peaks, fs


def synth_eda(fs: float = 100.0, duration_sec: float = 30.0):
    """EDA: slow tonic drift + 3 known SCR bumps (fast rise, slow decay)."""
    rng = np.random.default_rng(SEED + 3)
    n = int(fs * duration_sec)
    idx = np.arange(n)
    sig = 2.0 + 0.4 * (idx / n)  # tonic level + drift
    onsets = np.array([int(5 * fs), int(15 * fs), int(25 * fs)])
    amps = [0.5, 0.35, 0.45]
    true_peaks = np.zeros_like(onsets)
    for k, (o, a) in enumerate(zip(onsets, amps)):
        d = idx - o
        bump = np.where(
            d >= 0,
            a * (1 - np.exp(-d / (0.5 * fs))) * np.exp(-np.maximum(d - 0.5 * fs, 0) / (3.0 * fs)),
            0.0,
        )
        sig += bump
        true_peaks[k] = o + int(np.argmax(bump[o:]))
    sig += 0.002 * rng.standard_normal(n)
    return sig, onsets, true_peaks, fs


def generate(fixtures_dir: Path | None = None) -> dict[str, Path]:
    out = Path(fixtures_dir) if fixtures_dir else FIXTURES_DIR
    out.mkdir(parents=True, exist_ok=True)
    written: dict[str, Path] = {}

    ecg, ecg_peaks, fs = synth_ecg()
    p = out / "ecg_synthetic.npz"
    np.savez_compressed(p, signal=ecg, fs=fs, true_peaks=ecg_peaks,
                        modality="ecg", description="synthetic ECG @360Hz, 60 bpm")
    written["ecg"] = p

    ppg, ppg_peaks, fs = synth_ppg()
    p = out / "ppg_synthetic.npz"
    np.savez_compressed(p, signal=ppg, fs=fs, true_peaks=ppg_peaks,
                        modality="ppg", description="synthetic PPG @100Hz, 75 bpm")
    written["ppg"] = p

    rsp, rsp_peaks, fs = synth_rsp()
    p = out / "rsp_synthetic.npz"
    np.savez_compressed(p, signal=rsp, fs=fs, true_peaks=rsp_peaks,
                        modality="rsp", description="synthetic respiration @100Hz, 15 brpm")
    written["rsp"] = p

    eda, onsets, scr_peaks, fs = synth_eda()
    p = out / "eda_synthetic.npz"
    np.savez_compressed(p, signal=eda, fs=fs, true_onsets=onsets, true_peaks=scr_peaks,
                        modality="eda", description="synthetic EDA @100Hz, 3 SCRs")
    written["eda"] = p

    return written


if __name__ == "__main__":
    for name, path in generate().items():
        print(f"wrote {name}: {path} ({path.stat().st_size} bytes)")
