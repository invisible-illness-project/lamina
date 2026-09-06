import pytest
import numpy as np
import lamina

def test_signal_smooth_moving_average():
    data = np.array([1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0], dtype=np.float64)
    res = lamina.signal.smooth_moving_average(data, 3)
    assert isinstance(res, np.ndarray)
    assert len(res) == len(data)
    assert res[1] == pytest.approx(2.0)

def test_signal_filter_bandpass():
    t = np.linspace(0, 2.0, 500)
    sig = np.sin(2 * np.pi * 10 * t) + np.sin(2 * np.pi * 50 * t)
    filtered = lamina.signal.filter(sig, sampling_rate=250.0, low_cutoff=5.0, high_cutoff=15.0)
    assert isinstance(filtered, np.ndarray)
    assert len(filtered) == len(sig)

def test_signal_filtfilt():
    t = np.linspace(0, 2.0, 500)
    sig = np.sin(2 * np.pi * 10 * t)
    filtered = lamina.signal.filtfilt(sig, sampling_rate=250.0, low_cutoff=5.0, high_cutoff=15.0)
    assert isinstance(filtered, np.ndarray)
    assert len(filtered) == len(sig)

def test_signal_findpeaks():
    t = np.linspace(0, 2.0, 500)
    sig = np.sin(2 * np.pi * 5 * t)
    peaks = lamina.signal.findpeaks(sig, sampling_rate=250.0, min_distance_sec=0.1)
    assert isinstance(peaks, np.ndarray)
    assert len(peaks) > 0

def test_signal_findpeaks_mask():
    t = np.linspace(0, 2.0, 500)
    sig = np.sin(2 * np.pi * 5 * t)
    mask = lamina.signal.findpeaks_mask(sig, sampling_rate=250.0, min_distance_sec=0.1)
    assert isinstance(mask, np.ndarray)
    assert mask.dtype == np.bool_
    assert len(mask) == len(sig)
