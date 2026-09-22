import numpy as np
import pytest

import lamina.ppg as lppg


def test_ppg_preprocess_pulselm_basic():
    fs_in = 250.0
    duration_sec = 35.0
    n_in = int(fs_in * duration_sec)
    t = np.arange(n_in) / fs_in

    # Synthetic signal: DC offset + 1.2 Hz pulse + 20 Hz noise
    raw_ppg = 15.0 + 2.0 * np.sin(2 * np.pi * 1.2 * t) + 0.8 * np.sin(2 * np.pi * 20.0 * t)

    segments = lppg.preprocess_pulselm(raw_ppg, fs_in)

    assert len(segments) == 3
    for seg in segments:
        assert len(seg) == 1250 # 10s @ 125 Hz
        assert np.min(seg) >= -1e-12
        assert np.max(seg) <= 1.0 + 1e-12
        assert abs(np.min(seg) - 0.0) < 1e-5
        assert abs(np.max(seg) - 1.0) < 1e-5


def test_pulselm_pipeline_class_and_provenance_hash():
    pipeline = lppg.PulseLmPipeline(
        target_fs=125.0,
        filter_cutoff_hz=8.0,
        filter_order=4,
        window_sec=10.0,
        stride_sec=10.0,
        tail_policy="drop",
        degenerate_policy="midpoint",
    )

    spec_hash = pipeline.compute_sha256_hash()
    assert isinstance(spec_hash, str)
    assert len(spec_hash) == 64

    fs_in = 200.0
    t = np.arange(int(fs_in * 25.0)) / fs_in
    signal = np.sin(2 * np.pi * 1.0 * t)

    segments = pipeline.process(signal, fs_in)
    assert len(segments) == 2 # 25s resampled to 125 Hz = 3125 samples -> 2 windows of 1250 (25s dropped 5s tail)
    assert len(segments[0]) == 1250
