"""Tests for the canonical Signal schema (SPEC §8)."""

import numpy as np
import pytest

from validation.schema import (
    MODALITIES,
    Signal,
    SignalValidationError,
    resample_signal,
)


def make(fs=100.0, n=1000, **kw):
    kw.setdefault("modality", "ecg")
    return Signal(samples=np.sin(np.arange(n) / 10.0), sampling_rate=fs, **kw)


def test_valid_signal_passes():
    s = make()
    assert s.validate() is s
    assert s.n_samples == 1000
    assert s.duration_sec == pytest.approx(10.0)


def test_default_timestamps_uniform():
    s = make(fs=200.0, n=5)
    ts = s.get_timestamps()
    np.testing.assert_allclose(ts, np.arange(5) / 200.0)


def test_explicit_timestamps_preserved():
    s = make(n=4, timestamps=np.array([1.0, 1.5, 2.0, 2.5]))
    np.testing.assert_allclose(s.get_timestamps(), [1.0, 1.5, 2.0, 2.5])


def test_rejects_empty_and_bad_fs():
    with pytest.raises(SignalValidationError):
        make(n=0).validate()
    with pytest.raises(SignalValidationError):
        make(fs=0.0).validate()
    with pytest.raises(SignalValidationError):
        make(fs=np.nan).validate()


def test_rejects_unknown_modality():
    with pytest.raises(SignalValidationError):
        make(modality="lidar").validate()


def test_rejects_timestamp_length_mismatch_and_decrease():
    with pytest.raises(SignalValidationError):
        make(n=4, timestamps=np.array([0.0, 1.0])).validate()
    with pytest.raises(SignalValidationError):
        make(n=3, timestamps=np.array([0.0, 0.5, 0.4])).validate()


def test_all_spec_modalities_accepted():
    for m in MODALITIES:
        make(modality=m).validate()


def test_resample_signal_length_and_rate():
    s = make(fs=100.0, n=1000)
    up = resample_signal(s, 250.0)
    assert up.sampling_rate == pytest.approx(250.0)
    assert abs(up.n_samples - 2500) <= 3  # polyphase edge handling
    # identity resample returns same object
    assert resample_signal(s, 100.0) is s


def test_resample_signal_preserves_dominant_frequency():
    fs, n = 100.0, 2000
    t = np.arange(n) / fs
    s = Signal(samples=np.sin(2 * np.pi * 5 * t), sampling_rate=fs, modality="rsp")
    up = resample_signal(s, 360.0)
    spec = np.abs(np.fft.rfft(up.samples))
    freqs = np.fft.rfftfreq(up.n_samples, 1 / up.sampling_rate)
    assert freqs[np.argmax(spec)] == pytest.approx(5.0, abs=0.2)
