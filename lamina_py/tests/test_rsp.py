import pytest
import numpy as np
import lamina

def generate_synthetic_rsp(duration_sec=20.0, fs=100.0):
    t = np.linspace(0, duration_sec, int(duration_sec * fs))
    # 0.25 Hz respiration (15 breaths per min)
    rsp = np.sin(2 * np.pi * 0.25 * t)
    return rsp

def test_rsp_clean():
    rsp = generate_synthetic_rsp()
    cleaned = lamina.rsp.clean(rsp, sampling_rate=100.0)
    assert isinstance(cleaned, np.ndarray)
    assert len(cleaned) == len(rsp)

def test_rsp_findpeaks():
    rsp = generate_synthetic_rsp()
    peaks = lamina.rsp.findpeaks(rsp, sampling_rate=100.0)
    assert isinstance(peaks, np.ndarray)
    assert len(peaks) >= 3

def test_rsp_cycles():
    rsp = generate_synthetic_rsp()
    cycles = lamina.rsp.cycles(rsp, sampling_rate=100.0)
    assert isinstance(cycles, list)
    if cycles:
        c = cycles[0]
        assert hasattr(c, "inspiration_index")
        assert hasattr(c, "expiration_index")
        assert hasattr(c, "duration_sec")
        assert hasattr(c, "respiratory_rate_bpm")

def test_rsp_rate():
    rsp = generate_synthetic_rsp()
    rate_curve = lamina.rsp.rate(rsp, sampling_rate=100.0)
    assert isinstance(rate_curve, np.ndarray)
    assert len(rate_curve) == len(rsp)
