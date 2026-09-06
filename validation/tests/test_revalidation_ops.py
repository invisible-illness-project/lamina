"""Hermetic tests for the revalidation extension ops (SPEC.md Appendix A).

Covers the post-remediation API surface: ``eda-clean-config`` (EdaCleaningConfig
Nyquist/pass-through paths, BUG-003), ``rppg-polarity`` (SignalPolarity BVP
conversion, BUG-005), and ``hrv-correct`` (classify -> CorrectionPolicy -> NN
-> HRV pipeline, BUG-018), plus the ``ecg-clean`` method-dispatch bridge
mapping (BUG-001). Uses committed fixtures and tiny inline synthetics only —
no network, no downloads.
"""

import numpy as np
import pytest

from validation.bridge import LaminaBridgeError


# ---------------------------------------------------------------------------
# eda-clean-config (BUG-003: EdaCleaningConfig + Nyquist pass-through)
# ---------------------------------------------------------------------------

def test_eda_clean_config_pass_through_vs_filtered_differ(bridge, eda_fixture):
    sig, fs = eda_fixture["signal"], eda_fixture["fs"]  # 100 Hz, 30 s
    filtered = bridge.eda_clean_config(sig, fs, {"lowpass_cutoff_hz": 5.0})
    passthrough = bridge.eda_clean_config(sig, fs, {"lowpass_cutoff_hz": None})
    assert filtered["filter_applied"] is True
    assert passthrough["filter_applied"] is False
    assert passthrough["cutoff_hz"] is None
    # Pass-through returns the raw samples (within JSON float round-trip).
    assert np.allclose(passthrough["signal"], sig, atol=1e-12)
    # A 5 Hz low-pass must differ from the unfiltered signal on this fixture.
    assert not np.allclose(filtered["signal"], passthrough["signal"], atol=1e-6)
    # Absent cutoff key keeps the Lamina default (Some(5.0)) — same as explicit.
    default = bridge.eda_clean_config(sig, fs)
    assert default["filter_applied"] is True
    assert np.allclose(default["signal"], filtered["signal"], atol=1e-12)


def test_eda_clean_config_nyquist_paths(bridge):
    # 4 Hz Empatica-E4-style EDA (Nyquist = 2.0 Hz), inline synthetic.
    fs, n = 4.0, 400
    t = np.arange(n) / fs
    sig = 2.0 + 0.1 * np.sin(2 * np.pi * 1.0 * t)

    # Above-Nyquist cutoff with pass-through enabled (Lamina default): bypass.
    r = bridge.eda_clean_config(sig, fs, {"lowpass_cutoff_hz": 5.0})
    assert r["filter_applied"] is False and r["nyquist_hz"] == 2.0
    assert np.allclose(r["signal"], sig, atol=1e-12)

    # Same cutoff with pass-through disabled: Lamina InvalidCutoffFrequency.
    with pytest.raises(LaminaBridgeError) as ei:
        bridge.eda_clean_config(sig, fs, {
            "lowpass_cutoff_hz": 5.0,
            "pass_through_if_nyquist_violated": False,
        })
    assert ei.value.kind == "lamina_error"

    # Near-Nyquist cutoff (1.8 Hz < 2.0 Hz): real filtering happens.
    r = bridge.eda_clean_config(sig, fs, {"lowpass_cutoff_hz": 1.8})
    assert r["filter_applied"] is True
    assert r["signal"].shape == sig.shape and np.all(np.isfinite(r["signal"]))


# ---------------------------------------------------------------------------
# ecg-clean method dispatch (BUG-001 bridge mapping)
# ---------------------------------------------------------------------------

def test_ecg_clean_method_dispatch(bridge, ecg_fixture):
    sig, fs = ecg_fixture["signal"], ecg_fixture["fs"]
    default = bridge.ecg_clean(sig, fs)  # legacy default "none" -> ""
    named = bridge.ecg_clean(sig, fs, method="neurokit")
    assert np.allclose(default, named, atol=1e-12)
    with pytest.raises(LaminaBridgeError) as ei:
        bridge.ecg_clean(sig, fs, method="not-a-method")
    assert ei.value.kind == "lamina_error"


# ---------------------------------------------------------------------------
# rppg-polarity (BUG-005: SignalPolarity / to_bvp_waveform)
# ---------------------------------------------------------------------------

def _rgb_pulse(n=300, fps=30.0, f_pulse=1.2):
    ts = np.arange(n) / fps
    pulse = 1.0 + 0.02 * np.sin(2 * np.pi * f_pulse * ts)
    return ts, 120 * pulse, 150 * pulse, 100 * pulse


def test_rppg_polarity_normal_vs_inverted_flip_sign(bridge):
    ts, r, g, b = _rgb_pulse()
    cfg = {"algorithm": "green", "window_sec": 4.0, "step_sec": 1.0}
    normal = bridge.rppg_polarity(ts, r, g, b, polarity="normal", config=cfg)
    inverted = bridge.rppg_polarity(ts, r, g, b, polarity="inverted", config=cfg)
    assert normal["polarity_resolved"] == "normal" and normal["flipped"] is False
    assert inverted["polarity_resolved"] == "inverted" and inverted["flipped"] is True
    wn = np.asarray(normal["waveform"], dtype=float)
    wi = np.asarray(inverted["waveform"], dtype=float)
    assert wn.shape == wi.shape and wn.shape[0] == len(ts)
    assert np.allclose(wi, -wn, atol=1e-12)


def test_rppg_polarity_auto_resolves(bridge):
    # CHROM needs per-channel amplitude diversity: perfectly proportional
    # channels degenerate the chrominance projection to a constant (NaN out).
    fps, n = 30.0, 300
    ts = np.arange(n) / fps
    ph = 2 * np.pi * 1.2 * ts
    r = 120 * (1.0 + 0.010 * np.sin(ph))
    g = 150 * (1.0 + 0.030 * np.sin(ph))
    b = 100 * (1.0 + 0.005 * np.sin(ph + 0.3))
    cfg = {"algorithm": "chrom", "window_sec": 4.0, "step_sec": 1.0}
    auto = bridge.rppg_polarity(ts, r, g, b, polarity="auto", config=cfg)
    assert auto["polarity_requested"] == "auto"
    assert auto["polarity_resolved"] in ("normal", "inverted")
    # Resolved polarity must be consistent with the explicit-mode output.
    normal = bridge.rppg_polarity(ts, r, g, b, polarity="normal", config=cfg)
    wa = np.asarray(auto["waveform"], dtype=float)
    wn = np.asarray(normal["waveform"], dtype=float)
    expected = -wn if auto["polarity_resolved"] == "inverted" else wn
    assert np.allclose(wa, expected, atol=1e-12)


def test_rppg_polarity_rejects_unknown_polarity(bridge):
    ts, r, g, b = _rgb_pulse()
    with pytest.raises(LaminaBridgeError):
        bridge.rppg_polarity(ts, r, g, b, polarity="sideways",
                             config={"algorithm": "green"})


# ---------------------------------------------------------------------------
# hrv-correct (BUG-018: IntervalQuality -> CorrectionPolicy -> NN -> HRV)
# ---------------------------------------------------------------------------

# One ectopic interval (500 ms vs ~800 ms baseline) at index 2.
RR_ECTOPIC = [800.0, 810.0, 500.0, 805.0, 795.0, 800.0, 810.0, 805.0]


def test_hrv_correct_interpolate_linear_fills_gap(bridge):
    r = bridge.hrv_correct(RR_ECTOPIC, policy="interpolate_linear")
    assert r["n_input_intervals"] == len(RR_ECTOPIC)
    assert r["n_nn"] == len(RR_ECTOPIC)  # interpolation keeps series length
    assert r["interval_quality"][2] == "ectopic_rr"
    assert all(q == "normal_nn" for i, q in enumerate(r["interval_quality"]) if i != 2)
    nn = np.asarray(r["nn_intervals_ms"], dtype=float)
    # Gap filled by linear interpolation between the 810 and 805 ms neighbours.
    assert nn[2] == pytest.approx(807.5, abs=1e-9)
    assert r["rmssd_ms"] is not None and r["mean_nn_ms"] is not None


def test_hrv_correct_none_and_reject(bridge):
    r = bridge.hrv_correct(RR_ECTOPIC, policy="none")
    assert np.allclose(r["nn_intervals_ms"], RR_ECTOPIC)
    assert r["interval_quality"][2] == "ectopic_rr"  # classified, not corrected
    rj = bridge.hrv_correct(RR_ECTOPIC, policy="reject_invalid")
    assert rj["n_nn"] == len(RR_ECTOPIC) - 1
    assert 500.0 not in rj["nn_intervals_ms"]


def test_hrv_correct_percent_threshold_and_artifact(bridge):
    rr = [800.0, 820.0, 2500.0, 805.0, 790.0, 810.0, 800.0, 815.0]
    r = bridge.hrv_correct(rr, policy="percent_threshold", percent_threshold=0.10)
    assert r["interval_quality"][2] == "artifact_rr"  # > 2000 ms hard bound
    assert r["n_nn"] == len(rr) - 1
    with pytest.raises(LaminaBridgeError):
        bridge.hrv_correct(rr, policy="percent_threshold")  # missing param


def test_hrv_correct_accepts_peak_train(bridge):
    fs = 250.0
    rr_ms = np.array([800.0, 810.0, 790.0, 805.0, 795.0, 800.0])
    peaks = np.cumsum(rr_ms / 1000.0 * fs).astype(int) + 10
    n = int(peaks[-1] + 200)
    r = bridge.hrv_correct(peaks=peaks.tolist(), signal_length=n, fs=fs,
                           policy="reject_invalid")
    assert r["n_input_intervals"] == len(rr_ms) - 1
    intervals_ms = np.diff(peaks) / fs * 1000.0
    assert np.allclose(r["nn_intervals_ms"], intervals_ms)
    assert r["mean_nn_ms"] == pytest.approx(float(np.mean(intervals_ms)), rel=1e-6)


def test_hrv_correct_rejects_unknown_policy(bridge):
    with pytest.raises(LaminaBridgeError):
        bridge.hrv_correct(RR_ECTOPIC, policy="shuffle")
    with pytest.raises(ValueError):
        bridge.hrv_correct()  # neither intervals nor peaks given
