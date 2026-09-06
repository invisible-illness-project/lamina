import pytest
import numpy as np
import lamina

def generate_synthetic_ppg(duration_sec=3.0, fs=100.0):
    t = np.linspace(0, duration_sec, int(duration_sec * fs))
    ppg = 1.0 + 0.2 * np.sin(2 * np.pi * 1.2 * t) + 0.05 * np.sin(2 * np.pi * 2.4 * t)
    return ppg

def test_ppg_clean():
    ppg = generate_synthetic_ppg()
    cleaned = lamina.ppg.clean(ppg, sampling_rate=100.0)
    assert isinstance(cleaned, np.ndarray)
    assert len(cleaned) == len(ppg)

def test_ppg_findpeaks():
    ppg = generate_synthetic_ppg(duration_sec=5.0)
    cleaned = lamina.ppg.clean(ppg, sampling_rate=100.0)
    peaks = lamina.ppg.findpeaks(cleaned, sampling_rate=100.0)
    assert isinstance(peaks, np.ndarray)
    assert len(peaks) >= 3

def test_ppg_findpeaks_mask():
    ppg = generate_synthetic_ppg()
    cleaned = lamina.ppg.clean(ppg, sampling_rate=100.0)
    mask = lamina.ppg.findpeaks_mask(cleaned, sampling_rate=100.0)
    assert isinstance(mask, np.ndarray)
    assert mask.dtype == np.bool_
    assert len(mask) == len(cleaned)
