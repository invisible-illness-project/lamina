"""Bridge round-trip tests against the compiled Rust binary (SPEC §8).

These tests auto-build the bridge via cargo (skipped with a clear reason when
cargo is unavailable) and use the committed synthetic fixtures only — no
network, no large downloads.
"""

import numpy as np
import pytest

from validation.bridge import LaminaBridgeError
from validation.metrics import match_events, peak_detection_metrics

TOL_SEC = 0.150  # SPEC: >80% of true beats within 150 ms


def test_version_op(bridge):
    v = bridge.version()
    assert v["lamina_version"] and v["lamina_version"] != "unknown"
    assert v["bridge_version"]


def test_ecg_peaks_recovers_true_peaks(bridge, ecg_fixture):
    r = bridge.ecg_peaks(ecg_fixture["signal"], ecg_fixture["fs"])
    m = peak_detection_metrics(ecg_fixture["true_peaks"], r["peaks"],
                               ecg_fixture["fs"], TOL_SEC)
    assert m["recall"] is not None and m["recall"] > 0.80
    assert m["n_detected"] > 0


def test_ecg_clean_roundtrip(bridge, ecg_fixture):
    cleaned = bridge.ecg_clean(ecg_fixture["signal"], ecg_fixture["fs"])
    assert cleaned.shape == ecg_fixture["signal"].shape
    assert np.all(np.isfinite(cleaned))


def test_ppg_peaks_count_sanity(bridge, ppg_fixture):
    r = bridge.ppg_peaks(ppg_fixture["signal"], ppg_fixture["fs"])
    n_true = len(ppg_fixture["true_peaks"])
    assert abs(r["count"] - n_true) <= max(2, int(0.25 * n_true))
    m = match_events(ppg_fixture["true_peaks"], r["peaks"], ppg_fixture["fs"], TOL_SEC)
    assert m.tp / max(n_true, 1) > 0.6


def test_eda_peaks_finds_synthetic_scrs(bridge, eda_fixture):
    r = bridge.eda_peaks(eda_fixture["signal"], eda_fixture["fs"])
    assert r["count"] >= 1
    detected_onsets = [e["onset_index"] for e in r["events"]]
    m = match_events(eda_fixture["true_onsets"], detected_onsets,
                     eda_fixture["fs"], tolerance_sec=1.0)
    assert m.tp >= 2  # at least 2 of 3 synthetic SCRs


def test_eda_decompose_shapes(bridge, eda_fixture):
    comp = bridge.eda_decompose(eda_fixture["signal"], eda_fixture["fs"])
    n = len(eda_fixture["signal"])
    assert comp["tonic"].shape == (n,) and comp["phasic"].shape == (n,)


def test_rsp_cycles_rate_sanity(bridge, rsp_fixture):
    r = bridge.rsp_cycles(rsp_fixture["signal"], rsp_fixture["fs"])
    rates = [c["respiratory_rate_bpm"] for c in r["cycles"]]
    assert r["count"] >= 4
    assert abs(np.median(rates) - 15.0) < 3.0
    assert len(r["rate"]) == len(rsp_fixture["signal"])


def test_hrv_from_known_peak_train(bridge):
    fs = 250.0
    rr_ms = np.array([800.0, 810.0, 790.0, 805.0, 795.0, 800.0])
    peaks = np.cumsum(rr_ms / 1000.0 * fs).astype(int) + 10
    n = int(peaks[-1] + 200)
    r = bridge.hrv(peaks.tolist(), n, fs)
    assert r["n_intervals"] == len(rr_ms) - 1
    # Expected values derived from the exact (integer-sample) peak train.
    intervals_ms = np.diff(peaks) / fs * 1000.0
    assert r["rmssd_ms"] == pytest.approx(
        float(np.sqrt(np.mean(np.diff(intervals_ms) ** 2))), rel=1e-6)
    assert r["mean_nn_ms"] == pytest.approx(float(np.mean(intervals_ms)), rel=1e-6)


def test_hrv_null_when_insufficient(bridge):
    r = bridge.hrv([100], 1000, 250.0)
    assert r["rmssd_ms"] is None and r["mean_nn_ms"] is None


def test_signal_peaks_op(bridge, ppg_fixture):
    r = bridge.signal_peaks(
        ppg_fixture["signal"], {"min_distance": 50, "min_prominence": 0.3}
    )
    assert r["count"] > 0
    assert len(r["peaks"]) == r["count"]


def test_filter_op_length_and_finite(bridge, ecg_fixture):
    out = bridge.filter(ecg_fixture["signal"], ecg_fixture["fs"],
                        kind="bandpass", cutoffs=[5.0, 15.0], order=2)
    assert out.shape == ecg_fixture["signal"].shape
    assert np.all(np.isfinite(out))


def test_sample_entropy_finite_and_infinite(bridge):
    rng = np.random.default_rng(0)
    noisy = rng.standard_normal(500)
    r = bridge.sample_entropy(noisy, m=2, r=0.2)
    assert r["is_infinite"] is False
    assert r["sample_entropy"] is not None and r["sample_entropy"] > 0
    # Strictly monotonic ramp with tiny r: no template matches at all ->
    # Lamina returns +inf, serialized as null with the is_infinite flag
    # (documented Lamina behavior; see docs/validation/BUGS.md taxonomy).
    ramp = np.arange(500, dtype=float)
    r2 = bridge.sample_entropy(ramp, m=2, r=1e-9)
    assert r2["is_infinite"] is True and r2["sample_entropy"] is None


def test_rppg_algorithm_recovers_pulse_rate(bridge):
    fps, n = 30.0, 300
    ts = np.arange(n) / fps
    pulse = 1.0 + 0.02 * np.sin(2 * np.pi * 1.2 * ts)
    r = bridge.rppg_algorithm(
        ts, 120 * pulse, 150 * pulse, 100 * pulse,
        config={"algorithm": "green", "window_sec": 4.0, "step_sec": 1.0},
    )
    wf = np.asarray(r["waveform"], dtype=float)
    assert wf.shape[0] == n
    assert np.all(np.isfinite(wf))
    spec = np.abs(np.fft.rfft(wf - wf.mean()))
    freqs = np.fft.rfftfreq(n, 1 / fps)
    assert freqs[np.argmax(spec[1:]) + 1] == pytest.approx(1.2, abs=0.1)


def test_bridge_error_envelope(bridge):
    with pytest.raises(LaminaBridgeError) as ei:
        bridge.ecg_peaks([1.0, 2.0, 3.0], 360.0)  # too short -> Lamina error
    assert ei.value.kind in ("lamina_error", "panic")


def test_unknown_op_raises(bridge):
    with pytest.raises(LaminaBridgeError):
        bridge.run_op("no-such-op", {})
