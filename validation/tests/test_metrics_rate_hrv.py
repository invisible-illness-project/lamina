"""Tests for rate and HRV metrics (SPEC §5/§8)."""

import numpy as np
import pytest

from validation.metrics import hrv_metrics, rate_metrics


def test_rate_metrics_known_values():
    ref = [60.0, 70.0, 80.0]
    est = [61.0, 69.0, 82.0]
    m = rate_metrics(ref, est)
    diff = np.array([1.0, -1.0, 2.0])
    assert m["mae"] == pytest.approx(np.mean(np.abs(diff)))
    assert m["rmse"] == pytest.approx(np.sqrt(np.mean(diff**2)))
    assert m["bias"] == pytest.approx(np.mean(diff))
    assert m["n"] == 3


def test_rate_metrics_perfect_correlation():
    ref = np.linspace(50, 100, 20)
    m = rate_metrics(ref, ref + 1.0)
    assert m["pearson_r"] == pytest.approx(1.0)
    assert m["mae"] == pytest.approx(1.0)


def test_rate_metrics_drops_nonfinite_pairs():
    m = rate_metrics([60.0, np.nan, 80.0], [60.0, 70.0, 80.0])
    assert m["n"] == 2 and m["mae"] == pytest.approx(0.0)


def test_rate_metrics_empty():
    m = rate_metrics([], [])
    assert m["n"] == 0 and m["mae"] is None


def test_rate_metrics_shape_mismatch_raises():
    with pytest.raises(ValueError):
        rate_metrics([1.0, 2.0], [1.0])


def test_hrv_metrics_known_values():
    ref = [40.0, 50.0]
    est = [44.0, 45.0]
    m = hrv_metrics(ref, est)
    assert m["mae"] == pytest.approx(4.5)
    assert m["mean_relative_error"] == pytest.approx(np.mean([4 / 40, 5 / 50]))
    assert m["bias"] == pytest.approx(-0.5)


def test_hrv_metrics_zero_reference_excluded_from_relative_error():
    m = hrv_metrics([0.0, 50.0], [10.0, 55.0])
    assert m["mean_relative_error"] == pytest.approx(5 / 50)
