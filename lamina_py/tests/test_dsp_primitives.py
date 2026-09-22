import numpy as np
import pytest
from scipy import signal as scipy_signal

import lamina.signal as lsig


def test_resample_poly_parity():
    np.random.seed(42)
    x = np.random.randn(500)

    # 1. 500 Hz -> 125 Hz (up=1, down=4)
    y_lamina = lsig.resample_poly(x, 1, 4)
    y_scipy = scipy_signal.resample_poly(x, 1, 4, window=('kaiser', 5.0))

    assert len(y_lamina) == len(y_scipy)
    np.testing.assert_allclose(y_lamina, y_scipy, atol=1e-4)

    # 2. 100 Hz -> 250 Hz (up=5, down=2)
    y_lamina2 = lsig.resample_poly(x, 5, 2)
    y_scipy2 = scipy_signal.resample_poly(x, 5, 2, window=('kaiser', 5.0))
    np.testing.assert_allclose(y_lamina2, y_scipy2, atol=1e-4)


def test_segment_signal():
    x = np.arange(25, dtype=np.float64)

    # Drop incomplete
    segs = lsig.segment_signal(x, window_samples=10, step_samples=10, tail_policy="drop")
    assert len(segs) == 2
    assert len(segs[0]) == 10
    assert len(segs[1]) == 10

    # Pad zeros
    segs_pad = lsig.segment_signal(x, window_samples=10, step_samples=10, tail_policy="pad")
    assert len(segs_pad) == 3
    assert len(segs_pad[2]) == 10
    assert segs_pad[2][4] == 24.0
    assert segs_pad[2][5] == 0.0

    # Keep partial
    segs_keep = lsig.segment_signal(x, window_samples=10, step_samples=10, tail_policy="keep")
    assert len(segs_keep) == 3
    assert len(segs_keep[2]) == 5


def test_remove_dc():
    x = np.array([10.0, 20.0, 30.0, 40.0], dtype=np.float64)
    cleaned = lsig.remove_dc(x)
    np.testing.assert_allclose(cleaned, np.array([-15.0, -5.0, 5.0, 15.0]))
    assert abs(np.mean(cleaned)) < 1e-12


def test_minmax_scale():
    x = np.array([10.0, 20.0, 30.0, 40.0, 50.0], dtype=np.float64)
    norm = lsig.minmax_scale(x, feature_range=(0.0, 1.0), degenerate_policy="midpoint")
    np.testing.assert_allclose(norm, np.array([0.0, 0.25, 0.5, 0.75, 1.0]))

    flat = np.array([5.0, 5.0, 5.0, 5.0], dtype=np.float64)
    flat_mid = lsig.minmax_scale(flat, feature_range=(0.0, 1.0), degenerate_policy="midpoint")
    np.testing.assert_allclose(flat_mid, np.full(4, 0.5))

    with pytest.raises(Exception):
        lsig.minmax_scale(flat, feature_range=(0.0, 1.0), degenerate_policy="error")
