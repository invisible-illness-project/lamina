import pytest
import numpy as np
import lamina

def test_hrv_peaks_to_intervals():
    peaks = [100, 350, 600, 850, 1100]
    intervals = lamina.hrv.peaks_to_intervals(peaks, sampling_rate=250.0)
    assert isinstance(intervals, np.ndarray)
    assert len(intervals) == 4
    np.testing.assert_allclose(intervals, [1.0, 1.0, 1.0, 1.0])

def test_hrv_rmssd():
    rr = np.array([1.0, 1.05, 0.98, 1.02, 0.99], dtype=np.float64)
    rmssd_val = lamina.hrv.rmssd(rr)
    assert isinstance(rmssd_val, float)
    assert rmssd_val > 0.0

def test_hrv_mean_nn():
    rr = np.array([1.0, 1.05, 0.98, 1.02, 0.99], dtype=np.float64)
    mean_val = lamina.hrv.mean_nn(rr)
    assert isinstance(mean_val, float)
    assert mean_val == pytest.approx(np.mean(rr))
