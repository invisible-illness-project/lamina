"""Tests for event matching and peak-detection metrics (SPEC §5/§8)."""

import numpy as np
import pytest

from validation.metrics import match_events, peak_detection_metrics


def test_perfect_match():
    ref = [100, 200, 300]
    m = match_events(ref, [101, 199, 300], fs=100.0, tolerance_sec=0.05)
    assert m.tp == 3 and m.fp == 0 and m.fn == 0
    assert m.timing_errors_sec == pytest.approx([0.01, -0.01, 0.0])


def test_tolerance_boundary():
    # 5 samples @ fs=100 with tol=0.05s -> exactly at the boundary counts.
    m = match_events([1000], [1005], fs=100.0, tolerance_sec=0.05)
    assert m.tp == 1
    m2 = match_events([1000], [1006], fs=100.0, tolerance_sec=0.05)
    assert m2.tp == 0 and m2.fn == 1 and m2.fp == 1


def test_greedy_one_to_one_no_double_matching():
    # Two detections near one reference: only the nearest matches.
    m = match_events([100], [98, 103], fs=100.0, tolerance_sec=0.1)
    assert m.tp == 1 and m.fp == 1 and m.fn == 0
    assert m.matches[0][1] == 98
    # One detection near two references: matches exactly one.
    m2 = match_events([100, 104], [102], fs=100.0, tolerance_sec=0.1)
    assert m2.tp == 1 and m2.fn == 1 and m2.fp == 0


def test_fp_fn_accounting():
    m = peak_detection_metrics([100, 200], [100, 250], fs=100.0, tolerance_sec=0.02)
    assert m["tp"] == 1 and m["fn"] == 1 and m["fp"] == 1
    assert m["precision"] == pytest.approx(0.5)
    assert m["recall"] == pytest.approx(0.5)
    assert m["f1"] == pytest.approx(0.5)


def test_empty_inputs():
    m = peak_detection_metrics([], [], fs=100.0, tolerance_sec=0.1)
    assert m["tp"] == 0 and m["precision"] is None and m["recall"] is None
    assert m["mean_abs_timing_error_sec"] is None
    m2 = peak_detection_metrics([1, 2], [], fs=100.0, tolerance_sec=0.1)
    assert m2["fn"] == 2 and m2["recall"] == 0.0


def test_timing_error_statistics():
    m = peak_detection_metrics([100, 200, 300], [102, 204, 310], fs=100.0,
                               tolerance_sec=0.5)
    assert m["mean_abs_timing_error_sec"] == pytest.approx(np.mean([0.02, 0.04, 0.10]))
    assert m["median_timing_error_sec"] == pytest.approx(0.04)
    assert m["n_reference"] == 3 and m["n_detected"] == 3


def test_unsorted_inputs_handled():
    m = match_events([300, 100], [100, 300], fs=100.0, tolerance_sec=0.01)
    assert m.tp == 2
