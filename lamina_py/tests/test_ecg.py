import pytest
import numpy as np
import lamina

def generate_synthetic_ecg(duration_sec=3.0, fs=250.0):
    t = np.linspace(0, duration_sec, int(duration_sec * fs))
    # 1 Hz fundamental (60 BPM) + harmonics
    ecg = np.sin(2 * np.pi * 1.0 * t) + 0.5 * np.sin(2 * np.pi * 2.0 * t) + 2.0 * np.exp(-((t % 1.0 - 0.2)**2) / 0.001)
    return ecg

def test_ecg_clean():
    ecg = generate_synthetic_ecg()
    cleaned = lamina.ecg.clean(ecg, sampling_rate=250.0)
    assert isinstance(cleaned, np.ndarray)
    assert len(cleaned) == len(ecg)

def test_ecg_findpeaks():
    ecg = generate_synthetic_ecg(duration_sec=5.0)
    cleaned = lamina.ecg.clean(ecg, sampling_rate=250.0)
    peaks = lamina.ecg.findpeaks(cleaned, sampling_rate=250.0)
    assert isinstance(peaks, np.ndarray)
    assert len(peaks) >= 4

def test_ecg_findpeaks_mask():
    ecg = generate_synthetic_ecg()
    cleaned = lamina.ecg.clean(ecg, sampling_rate=250.0)
    mask = lamina.ecg.findpeaks_mask(cleaned, sampling_rate=250.0)
    assert isinstance(mask, np.ndarray)
    assert mask.dtype == np.bool_
    assert len(mask) == len(cleaned)
