import pytest
import numpy as np
import lamina

def generate_synthetic_eda(duration_sec=10.0, fs=100.0):
    t = np.linspace(0, duration_sec, int(duration_sec * fs))
    tonic = 2.0 + 0.05 * t
    phasic = 0.5 * np.exp(-((t - 3.0)**2) / 0.5) + 0.8 * np.exp(-((t - 7.0)**2) / 0.5)
    return tonic + phasic

def test_eda_clean():
    eda = generate_synthetic_eda()
    cleaned = lamina.eda.clean(eda, sampling_rate=100.0)
    assert isinstance(cleaned, np.ndarray)
    assert len(cleaned) == len(eda)

def test_eda_decompose():
    eda = generate_synthetic_eda()
    comp = lamina.eda.decompose(eda, sampling_rate=100.0)
    assert hasattr(comp, "tonic")
    assert hasattr(comp, "phasic")
    assert len(comp.tonic) == len(eda)
    assert len(comp.phasic) == len(eda)

def test_eda_phasic():
    eda = generate_synthetic_eda()
    ph = lamina.eda.phasic(eda, sampling_rate=100.0)
    assert isinstance(ph, np.ndarray)
    assert len(ph) == len(eda)

def test_eda_findpeaks():
    eda = generate_synthetic_eda()
    peaks = lamina.eda.findpeaks(eda, sampling_rate=100.0)
    assert isinstance(peaks, np.ndarray)
    assert len(peaks) >= 1

def test_eda_findpeaks_events():
    eda = generate_synthetic_eda()
    events = lamina.eda.findpeaks_events(eda, sampling_rate=100.0)
    assert isinstance(events, list)
    if events:
        ev = events[0]
        assert hasattr(ev, "onset_index")
        assert hasattr(ev, "peak_index")
        assert hasattr(ev, "amplitude")
        assert hasattr(ev, "rise_time_sec")
