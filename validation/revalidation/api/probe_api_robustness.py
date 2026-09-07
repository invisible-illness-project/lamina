#!/usr/bin/env python3
"""Wave G — systematic input-robustness probe for the Lamina JSON bridge.

Validation-only: exercises the compiled ``lamina_bridge`` binary (Lamina is the
implementation under test; this script never touches Lamina source). For each
(op, input-case) it records outcome (ok|lamina_error|bad_request|panic|
timeout|crash), the verbatim error kind/message, the process exit code, and a
correctness judgment (does the error identify the ACTUAL invalid input?).

Non-finite floats cannot be expressed in standard JSON; this probe injects raw
``NaN``/``Infinity``/``-Infinity`` tokens and explicit ``null`` array elements
directly into the input JSON text to test the protocol boundary as well as the
Lamina validation layer behind it.

Usage: python3 probe_api_robustness.py   (from anywhere; paths are absolute)
Writes: api_results.csv, evidence/raw/*.json, evidence/inputs/*.json
"""

from __future__ import annotations

import csv
import json
import subprocess
import sys
import time
from pathlib import Path

import numpy as np

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[2]
BIN = REPO / "validation" / "lamina_bridge" / "target" / "release" / "lamina_bridge"
RAW_DIR = HERE / "evidence" / "raw"
IN_DIR = HERE / "evidence" / "inputs"
TIMEOUT_SEC = 30.0

# ---------------------------------------------------------------------------
# Raw-JSON token support (NaN / Infinity / null injection past json.dumps)
# ---------------------------------------------------------------------------

class R(str):
    """Raw JSON token injected verbatim into the serialized input."""


def to_json_text(obj) -> str:
    sentinels: dict[str, str] = {}

    def walk(o):
        if isinstance(o, R):
            key = f"__RAW{len(sentinels)}__"
            sentinels[key] = str(o)
            return key
        if isinstance(o, dict):
            return {k: walk(v) for k, v in o.items()}
        if isinstance(o, (list, tuple)):
            return [walk(v) for v in o]
        if isinstance(o, np.ndarray):
            return walk(o.tolist())
        if isinstance(o, (np.floating, np.integer)):
            return o.item()
        if isinstance(o, float):
            if np.isnan(o):
                return walk(R("NaN"))
            if np.isinf(o):
                return walk(R("Infinity" if o > 0 else "-Infinity"))
        return o

    text = json.dumps(walk(obj))
    for k, v in sentinels.items():
        text = text.replace(f'"{k}"', v)
    return text


# ---------------------------------------------------------------------------
# Deterministic baseline signals (fixture-backed where available)
# ---------------------------------------------------------------------------

def _fixture(name, key="signal"):
    p = REPO / "validation" / "fixtures" / name
    if p.exists():
        d = np.load(p)
        return np.asarray(d[key], dtype=float)
    return None


def sig_ecg():
    s = _fixture("ecg_synthetic.npz")
    if s is not None:
        return s, 360.0
    t = np.arange(3600) / 360.0
    beats = np.zeros(3600)
    for k in range(10):
        c = int(180 + k * 360)
        beats += np.exp(-0.5 * ((np.arange(3600) - c) / 8.0) ** 2)
    return beats + 0.05 * np.sin(2 * np.pi * 0.3 * t), 360.0


def sig_ppg():
    s = _fixture("ppg_synthetic.npz")
    if s is not None:
        return s, 100.0
    t = np.arange(1200) / 100.0
    return np.sin(2 * np.pi * 1.25 * t) + 0.3 * np.sin(2 * np.pi * 2.5 * t), 100.0


def sig_rsp():
    s = _fixture("rsp_synthetic.npz")
    if s is not None:
        return s, 100.0
    t = np.arange(3000) / 100.0
    return np.sin(2 * np.pi * 0.25 * t), 100.0


def sig_eda():
    s = _fixture("eda_synthetic.npz")
    if s is not None:
        return s, 100.0
    x = 2.0 * np.ones(3000)
    for c in (600, 1400, 2200):
        x[c:] += 0.5 * np.exp(-(np.arange(3000 - c)) / 200.0)
    return x, 100.0


def sig_generic():
    rng = np.random.default_rng(42)
    t = np.arange(1000) / 250.0
    return np.sin(2 * np.pi * 5 * t) + 0.2 * rng.standard_normal(1000), 250.0


def sig_noise():
    return np.random.default_rng(7).standard_normal(300), None


def sig_rppg():
    n, fs = 900, 30.0
    ts = np.arange(n) / fs
    pulse = 0.02 * np.sin(2 * np.pi * 1.2 * ts)
    g = 100.0 * (1.0 + pulse)
    r = 120.0 * (1.0 + 0.8 * pulse)
    b = 80.0 * (1.0 + 0.6 * pulse)
    return ts, r, g, b


# ---------------------------------------------------------------------------
# Case registry
# ---------------------------------------------------------------------------

CASES: list[dict] = []


def add(op, category, case, params, payload, expect="inspect", notes=""):
    CASES.append(dict(op=op, category=category, case=case, params=params,
                      payload=payload, expect=expect, notes=notes))


def sig_payload(op, sig, fs, config=None):
    p = {"signal": np.asarray(sig, dtype=float)}
    if fs is not None:
        p["sampling_rate"] = fs
    if config:
        p["config"] = config
    return p


def mut(payload, **kw):
    """Return a mutated deep-ish copy. Values may be R() raw tokens, None (=drop
    when prefixed 'drop__'), or ('cfg', key, value) via cfg__ keys."""
    p = json.loads(to_json_text(payload))  # plain copy (raw tokens resolved to strings; replaced later)
    # re-resolve raw tokens: simpler path — apply mutations on the original object graph
    p = {k: v for k, v in payload.items()}
    if "config" in p and p["config"] is not None:
        p["config"] = dict(p["config"])
    for k, v in kw.items():
        if k.startswith("drop__"):
            p.pop(k[6:], None)
        elif k.startswith("cfg__"):
            p.setdefault("config", {})
            if v == "__DROP__":
                p["config"].pop(k[5:], None)
            else:
                p["config"][k[5:]] = v
        else:
            p[k] = v
    return p


NAN = R("NaN")
INF = R("Infinity")
NINF = R("-Infinity")
NULL = R("null")


def with_sample(sig, idx, token):
    s = list(np.asarray(sig, dtype=float))
    s[idx] = token
    return s


# ---------------------------------------------------------------------------
# Build the matrix
# ---------------------------------------------------------------------------

SIGNAL_OPS = {  # op -> (baseline signal, fs, baseline config or None)
    "ecg-clean": ("ecg", None),
    "ecg-peaks": ("ecg", None),
    "ppg-clean": ("ppg", None),
    "ppg-peaks": ("ppg", None),
    "eda-clean": ("eda", None),
    "eda-decompose": ("eda", None),
    "eda-peaks": ("eda", None),
    "rsp-clean": ("rsp", None),
    "rsp-cycles": ("rsp", None),
    "filter": ("gen", {"kind": "lowpass", "cutoff": 10.0, "order": 2}),
    "sample-entropy": ("noise", None),  # no sampling_rate
    "signal-peaks": ("gen", None),      # no sampling_rate
}


def get_sig(name):
    return {"ecg": sig_ecg, "ppg": sig_ppg, "rsp": sig_rsp,
            "eda": sig_eda, "gen": sig_generic, "noise": sig_noise}[name]()


def build_signal_matrix():
    for op, (which, base_cfg) in SIGNAL_OPS.items():
        sig, fs = get_sig(which)
        base = sig_payload(op, sig, fs, base_cfg)
        n = len(sig)

        # --- baseline sanity
        add(op, "baseline", "valid_baseline", "defaults", base, expect="ok")

        # --- sampling rate cases
        if fs is not None:
            for case, v, note in [
                ("fs_zero", 0.0, "contract: Fs>0 -> InvalidSamplingRate"),
                ("fs_negative", -100.0, "contract: Fs>0 -> InvalidSamplingRate"),
                ("fs_nan", NAN, "raw JSON NaN token"),
                ("fs_inf", INF, "raw JSON Infinity token"),
                ("fs_huge", 1e9, "finite positive but absurd; check filter sanity"),
            ]:
                add(op, "sampling_rate", case, f"sampling_rate={v}",
                    mut(base, sampling_rate=v), notes=note)
            add(op, "sampling_rate", "fs_missing", "field omitted",
                mut(base, drop__sampling_rate=True))
            add(op, "sampling_rate", "fs_null", "sampling_rate=null",
                mut(base, sampling_rate=NULL))

        # --- signal shape cases
        add(op, "signal_shape", "sig_empty", "[]", mut(base, signal=[]))
        add(op, "signal_shape", "sig_single", "[1.0]", mut(base, signal=[1.0]))
        add(op, "signal_shape", "sig_two", "[1.0,2.0]", mut(base, signal=[1.0, 2.0]))
        add(op, "signal_shape", "sig_zeros", "1000 zeros", mut(base, signal=[0.0] * 1000))
        add(op, "signal_shape", "sig_ones", "1000 ones", mut(base, signal=[1.0] * 1000))
        add(op, "signal_shape", "sig_missing", "field omitted", mut(base, drop__signal=True))
        add(op, "signal_shape", "sig_null_element", "signal[5]=null",
            mut(base, signal=with_sample(sig, 5, NULL)))

        # --- non-finite samples (raw JSON tokens)
        for case, idx, tok in [
            ("sig_nan_first", 0, NAN), ("sig_nan_mid", n // 2, NAN),
            ("sig_nan_last", n - 1, NAN),
            ("sig_posinf_mid", n // 2, INF), ("sig_neginf_mid", n // 2, NINF),
        ]:
            add(op, "nonfinite", case, f"signal[{idx}]={tok}",
                mut(base, signal=with_sample(sig, idx, tok)))

        # --- extreme amplitudes
        add(op, "amplitude", "sig_amp_1e12", "baseline*1e12",
            mut(base, signal=(np.asarray(sig) * 1e12)))
        add(op, "amplitude", "sig_amp_1e-12", "baseline*1e-12",
            mut(base, signal=(np.asarray(sig) * 1e-12)))


def build_ecg_cases():
    sig, fs = sig_ecg()
    base_clean = sig_payload("ecg-clean", sig, fs)
    for case, m in [("method_bogus", "bogus"), ("method_empty", ""),
                    ("method_neurokit", "neurokit"), ("method_upper", "NEUROKIT"),
                    ("method_hamilton", "hamilton")]:
        add("ecg-clean", "method", case, f"method={m!r}",
            mut(base_clean, config={"method": m}),
            notes="BUG-001 dispatch: ''/neurokit/pantompkins/biosppy accepted")
    base = sig_payload("ecg-peaks", sig, fs)
    for case, v in [("tm_negative", -0.5), ("tm_zero", 0.0), ("tm_one", 1.0),
                    ("tm_1p5", 1.5), ("tm_nan", NAN)]:
        add("ecg-peaks", "threshold", case, f"threshold_multiplier={v}",
            mut(base, cfg__threshold_multiplier=v),
            notes="BUG-002: expect range error (0,1), not NonFiniteInput")
    for case, cfg in [
        ("lowcut_zero", {"lowcut": 0.0}), ("lowcut_negative", {"lowcut": -1.0}),
        ("lowcut_above_nyquist", {"lowcut": 200.0}),
        ("lowcut_gt_highcut", {"lowcut": 40.0, "highcut": 5.0}),
        ("highcut_eq_nyquist", {"lowcut": 5.0, "highcut": 180.0}),
        ("filter_order_zero", {"filter_order": 0}),
        ("integration_window_zero", {"integration_window_sec": 0.0}),
        ("integration_window_negative", {"integration_window_sec": -0.1}),
        ("refractory_negative", {"refractory_period_sec": -0.2}),
        ("refractory_zero", {"refractory_period_sec": 0.0}),
    ]:
        add("ecg-peaks", "config", case, json.dumps(cfg), mut(base, config=cfg))


def build_ppg_cases():
    sig, fs = sig_ppg()
    base = sig_payload("ppg-peaks", sig, fs)
    for case, cfg in [
        ("lowcut_negative", {"lowcut": -1.0}), ("lowcut_zero", {"lowcut": 0.0}),
        ("highcut_above_nyquist", {"highcut": 200.0}),
        ("highcut_eq_nyquist", {"highcut": 50.0}),
        ("lowcut_gt_highcut", {"lowcut": 10.0, "highcut": 0.5}),
        ("alpha_zero", {"alpha": 0.0}), ("alpha_negative", {"alpha": -0.1}),
        ("alpha_gt1", {"alpha": 1.5}),
        ("w_peak_zero", {"w_peak_sec": 0.0}), ("w_peak_negative", {"w_peak_sec": -0.1}),
        ("w_beat_zero", {"w_beat_sec": 0.0}), ("w_beat_negative", {"w_beat_sec": -1.0}),
        ("refractory_negative", {"refractory_period_sec": -0.1}),
        ("filter_order_zero", {"filter_order": 0}),
    ]:
        add("ppg-peaks", "config", case, json.dumps(cfg), mut(base, config=cfg))


def build_eda_cases():
    sig, fs = sig_eda()
    base_dec = sig_payload("eda-decompose", sig, fs)
    for case, cfg in [
        ("tonic_cutoff_zero", {"tonic_cutoff_hz": 0.0}),
        ("tonic_cutoff_negative", {"tonic_cutoff_hz": -0.05}),
        ("tonic_above_nyquist", {"tonic_cutoff_hz": 200.0}),
        ("tonic_eq_nyquist", {"tonic_cutoff_hz": 50.0}),
        ("filter_order_zero", {"filter_order": 0}),
    ]:
        add("eda-decompose", "config", case, json.dumps(cfg), mut(base_dec, config=cfg))
    base_pk = sig_payload("eda-peaks", sig, fs)
    for case, cfg in [
        ("min_amplitude_negative", {"min_amplitude": -0.1}),
        ("min_prominence_negative", {"min_prominence": -0.05}),
        ("min_distance_negative", {"min_distance_sec": -1.0}),
        ("min_rise_negative", {"min_rise_time_sec": -0.5}),
        ("rise_inverted", {"min_rise_time_sec": 5.0, "max_rise_time_sec": 1.0}),
        ("max_rise_zero", {"max_rise_time_sec": 0.0}),
    ]:
        add("eda-peaks", "threshold", case, json.dumps(cfg), mut(base_pk, config=cfg))
    # fs-sanity (BUG-003): default eda-clean has 5 Hz lowpass
    for case, f in [("fs4_default_5hz_lowpass", 4.0), ("fs8", 8.0),
                    ("fs10_cutoff_eq_nyquist", 10.0), ("fs11", 11.0)]:
        add("eda-clean", "fs_sanity", case, f"fs={f} default cutoff=5Hz",
            sig_payload("eda-clean", sig, f),
            notes="contract 2.5: bypass or 0.45*fs; contract 2.1: InvalidCutoffFrequency")
    # eda-clean-config explicit paths
    base = sig_payload("eda-clean-config", sig, None)
    for case, f, cfg, note in [
        ("fs4_default", 4.0, None, "pass-through expected (BUG-003 remediation)"),
        ("fs4_passthrough_false", 4.0, {"pass_through_if_nyquist_violated": False},
         "expect InvalidCutoffFrequency"),
        ("fs4_cutoff_null", 4.0, {"lowpass_cutoff_hz": NULL}, "explicit null = no lowpass"),
        ("fs100_cutoff60_pt_true", 100.0, {"lowpass_cutoff_hz": 60.0}, "above-Nyquist pass-through"),
        ("fs100_cutoff60_pt_false", 100.0,
         {"lowpass_cutoff_hz": 60.0, "pass_through_if_nyquist_violated": False},
         "expect InvalidCutoffFrequency"),
        ("cutoff_eq_nyquist_fs10", 10.0, {"lowpass_cutoff_hz": 5.0},
         "cutoff==Nyquist: error.rs says strictly-less; contract 2.1 says Fs>=2fc ok"),
        ("cutoff_zero", 100.0, {"lowpass_cutoff_hz": 0.0}, ""),
        ("cutoff_negative", 100.0, {"lowpass_cutoff_hz": -1.0}, ""),
        ("cutoff_nan", 100.0, {"lowpass_cutoff_hz": NAN}, "raw token"),
        ("cutoff_string", 100.0, {"lowpass_cutoff_hz": "five"}, "wrong JSON type"),
        ("filter_order_zero", 100.0, {"filter_order": 0}, ""),
    ]:
        add("eda-clean-config", "fs_sanity" if case.startswith("fs") else "config",
            case, f"fs={f} cfg={json.dumps(cfg) if cfg else 'defaults'}",
            sig_payload("eda-clean-config", sig, f, cfg), notes=note)


def build_rsp_cases():
    sig, fs = sig_rsp()
    base = sig_payload("rsp-clean", sig, fs)
    for case, cfg in [
        ("lowcut_zero", {"lowcut": 0.0}), ("lowcut_negative", {"lowcut": -0.1}),
        ("highcut_above_nyquist", {"highcut": 100.0}),
        ("highcut_eq_nyquist", {"highcut": 50.0}),
        ("lowcut_gt_highcut", {"lowcut": 0.9, "highcut": 0.1}),
        ("filter_order_zero", {"filter_order": 0}),
    ]:
        add("rsp-clean", "config", case, json.dumps(cfg), mut(base, config=cfg))
    base = sig_payload("rsp-cycles", sig, fs)
    for case, cfg in [
        ("min_gt_max_interval", {"min_breath_interval_sec": 10.0, "max_breath_interval_sec": 1.0}),
        ("min_interval_zero", {"min_breath_interval_sec": 0.0}),
        ("min_interval_negative", {"min_breath_interval_sec": -1.0}),
        ("max_interval_negative", {"max_breath_interval_sec": -5.0}),
        ("min_amplitude_negative", {"min_amplitude": -0.5}),
        ("precleaned_true_on_raw", {"precleaned": True}),
    ]:
        add("rsp-cycles", "config", case, json.dumps(cfg), mut(base, config=cfg))


def build_filter_cases():
    sig, fs = sig_generic()
    base = sig_payload("filter", sig, fs, {"kind": "lowpass", "cutoff": 10.0, "order": 2})
    for case, cfg, note in [
        ("kind_missing", {"cutoff": 10.0}, "no kind"),
        ("kind_bogus", {"kind": "bogus", "cutoff": 10.0}, ""),
        ("kind_upper", {"kind": "LOWPASS", "cutoff": 10.0}, "case sensitivity"),
        ("lowpass_missing_cutoff", {"kind": "lowpass"}, ""),
        ("cutoff_negative", {"kind": "lowpass", "cutoff": -1.0}, ""),
        ("cutoff_zero", {"kind": "lowpass", "cutoff": 0.0}, ""),
        ("cutoff_above_nyquist", {"kind": "lowpass", "cutoff": 200.0}, ""),
        ("cutoff_eq_nyquist", {"kind": "lowpass", "cutoff": 125.0}, "fs=250"),
        ("cutoff_nan", {"kind": "lowpass", "cutoff": NAN}, "raw token"),
        ("highpass_above_nyquist", {"kind": "highpass", "cutoff": 200.0}, ""),
        ("bandpass_inverted", {"kind": "bandpass", "cutoffs": [20.0, 5.0]}, ""),
        ("bandpass_equal", {"kind": "bandpass", "cutoffs": [10.0, 10.0]}, ""),
        ("bandpass_len1", {"kind": "bandpass", "cutoffs": [10.0]}, ""),
        ("bandpass_len3", {"kind": "bandpass", "cutoffs": [5.0, 10.0, 20.0]}, ""),
        ("bandpass_high_above_nyquist", {"kind": "bandpass", "cutoffs": [5.0, 200.0]}, ""),
        ("notch_ok", {"kind": "notch", "cutoffs": [48.0, 52.0]}, "valid control"),
        ("order_zero", {"kind": "lowpass", "cutoff": 10.0, "order": 0}, ""),
        ("order_huge", {"kind": "lowpass", "cutoff": 10.0, "order": 500}, ""),
        ("zero_phase_false_bad_cutoff", {"kind": "lowpass", "cutoff": 200.0, "zero_phase": False}, ""),
    ]:
        add("filter", "config", case, json.dumps(cfg), mut(base, config=cfg), notes=note)


def build_entropy_cases():
    sig, _ = sig_noise()
    base = sig_payload("sample-entropy", sig, None)
    for case, cfg, note in [
        ("m_zero", {"m": 0}, "m>=1 per contract"),
        ("m_one", {"m": 1}, ""),
        ("m_huge", {"m": 1000}, "n=300: InsufficientSamples expected"),
        ("m_eq_n_minus_1", {"m": 298}, "boundary n<=m+1"),
        ("r_zero", {"r": 0.0}, ""),
        ("r_negative", {"r": -0.1}, ""),
        ("r_nan", {"r": NAN}, "raw token"),
        ("r_inf", {"r": INF}, "raw token"),
        ("r_tiny_no_matches", {"r": 1e-9}, "BUG-006: Ok(+inf) documented; bridge is_infinite"),
    ]:
        add("sample-entropy", "config", case, json.dumps(cfg), mut(base, config=cfg), notes=note)
    add("sample-entropy", "config", "constant_default_r", "zeros, r absent -> 0.2*std=0",
        sig_payload("sample-entropy", [0.0] * 300, None),
        notes="bridge convenience default collapses to r=0 (BUG-006 note)")


def build_signal_peaks_cases():
    sig, _ = sig_generic()
    base = sig_payload("signal-peaks", sig, None)
    for case, cfg in [
        ("min_height_nan", {"min_height": NAN}),
        ("min_height_above_all", {"min_height": 1e9}),
        ("min_distance_zero", {"min_distance": 0}),
        ("min_prominence_negative", {"min_prominence": -1.0}),
        ("min_prominence_nan", {"min_prominence": NAN}),
        ("min_width_zero", {"min_width": 0}),
        ("threshold_nan", {"threshold": NAN}),
    ]:
        add("signal-peaks", "config", case, json.dumps(cfg), mut(base, config=cfg))


def build_hrv_cases():
    good_peaks = [int(p) for p in np.arange(360, 3600 * 3, 360)]  # 60 bpm @360Hz
    base = {"peaks": good_peaks, "signal_length": 3600 * 3, "sampling_rate": 360.0}
    for case, p, note in [
        ("valid_baseline", {}, "ok expected"),
        ("fs_zero", {"sampling_rate": 0.0}, ""),
        ("fs_negative", {"sampling_rate": -360.0}, ""),
        ("fs_nan", {"sampling_rate": NAN}, "raw token"),
        ("fs_inf", {"sampling_rate": INF}, "raw token"),
        ("fs_missing", {"drop__sampling_rate": True}, ""),
        ("peaks_missing", {"drop__peaks": True}, ""),
        ("signal_length_missing", {"drop__signal_length": True}, ""),
        ("peaks_empty", {"peaks": []}, "SPEC: empty intervals + null HRV, not error"),
        ("peaks_single", {"peaks": [360]}, "null HRV, not error"),
        ("peaks_two", {"peaks": [360, 720]}, "1 interval"),
        ("peak_out_of_range", {"peaks": [360, 999999]}, "bridge range check"),
        ("peaks_unsorted", {"peaks": [720, 360, 1800]}, "mask: order lost"),
        ("peaks_duplicate", {"peaks": [360, 360, 720, 1080]}, "duplicate indices"),
        ("peaks_float", {"peaks": [360, 720.5]}, "JSON float into Vec<usize>"),
        ("peaks_negative", {"peaks": [-5, 360]}, "JSON negative into Vec<usize>"),
        ("signal_length_zero", {"peaks": [], "signal_length": 0}, ""),
    ]:
        add("hrv", "envelope" if "missing" in case or "float" in case or "negative" in case
            else ("sampling_rate" if case.startswith("fs") else "peaks"),
            case, json.dumps(p, default=str), mut(base, **p), notes=note)


def build_hrv_correct_cases():
    rr_normal = list(800.0 + 20.0 * np.sin(np.arange(20)))
    rr_out = [800.0, 820.0, 300.0, 810.0, 2500.0, 805.0, 799.0, 815.0, 780.0, 820.0]
    base = {"rr_intervals_ms": rr_normal}
    for case, p, note in [
        ("valid_default", {}, "policy none"),
        ("rr_empty", {"rr_intervals_ms": []}, "SPEC: empty outputs + null metrics, not error"),
        ("rr_single", {"rr_intervals_ms": [800.0]}, ""),
        ("policy_bogus", {"config": {"policy": "bogus"}}, ""),
        ("policy_reject_invalid", {"rr_intervals_ms": rr_out, "config": {"policy": "reject_invalid"}}, ""),
        ("policy_interpolate_linear", {"rr_intervals_ms": rr_out, "config": {"policy": "interpolate_linear"}}, ""),
        ("policy_interpolate_cubic", {"rr_intervals_ms": rr_out, "config": {"policy": "interpolate_cubic"}},
         "SPEC A.3: cubic == linear code path"),
        ("policy_percent_threshold_ok", {"rr_intervals_ms": rr_out,
         "config": {"policy": "percent_threshold", "percent_threshold": 0.2}}, ""),
        ("pt_missing_param", {"config": {"policy": "percent_threshold"}}, "requires percent_threshold"),
        ("pt_zero", {"config": {"policy": "percent_threshold", "percent_threshold": 0.0}}, "0<p<1"),
        ("pt_one", {"config": {"policy": "percent_threshold", "percent_threshold": 1.0}}, "0<p<1"),
        ("pt_gt1", {"config": {"policy": "percent_threshold", "percent_threshold": 1.5}}, "0<p<1"),
        ("pt_negative", {"config": {"policy": "percent_threshold", "percent_threshold": -0.2}}, ""),
        ("pt_nan", {"config": {"policy": "percent_threshold", "percent_threshold": NAN}}, "raw token"),
        ("classify_threshold_negative", {"config": {"classify_threshold": -0.1}}, ""),
        ("classify_threshold_zero", {"config": {"classify_threshold": 0.0}},
         "every deviation > 0 -> all ectopic?"),
        ("classify_threshold_gt1", {"config": {"classify_threshold": 1.5}}, ""),
        ("classify_threshold_nan", {"config": {"classify_threshold": NAN}}, "raw token"),
        ("rr_negative_interval", {"rr_intervals_ms": [800.0, -50.0, 810.0, 820.0, 800.0]},
         "negative RR is <300ms -> artifact?"),
        ("rr_zero_interval", {"rr_intervals_ms": [800.0, 0.0, 810.0, 820.0, 800.0]}, ""),
        ("rr_nan", {"rr_intervals_ms": [800.0, NAN, 810.0, 820.0, 800.0]}, "raw token"),
        ("rr_inf", {"rr_intervals_ms": [800.0, INF, 810.0, 820.0, 800.0]}, "raw token"),
        ("rr_all_artifact_reject", {"rr_intervals_ms": [100.0, 150.0, 200.0],
         "config": {"policy": "reject_invalid"}}, "all rejected -> empty NN series"),
        ("peaks_envelope_valid", {"drop__rr_intervals_ms": True, "peaks": [360, 720, 1080, 1500],
         "signal_length": 3600, "sampling_rate": 360.0}, ""),
        ("peaks_envelope_fs_zero", {"drop__rr_intervals_ms": True, "peaks": [360, 720],
         "signal_length": 3600, "sampling_rate": 0.0}, ""),
        ("nothing_given", {"drop__rr_intervals_ms": True}, "no rr, no peaks"),
    ]:
        add("hrv-correct", "policy" if "polic" in case or case.startswith("pt_")
            else ("threshold" if case.startswith("classify") else "input"),
            case, json.dumps(p, default=str), mut(base, **p), notes=note)


def build_rppg_cases():
    ts, r, g, b = sig_rppg()
    base = {"timestamps_sec": ts, "red": r, "green": g, "blue": b}
    for op in ("rppg-algorithm", "rppg-polarity"):
        b2 = dict(base)
        if op == "rppg-polarity":
            b2["config"] = {"polarity": "normal"}
        add(op, "baseline", "valid_baseline", "defaults", b2, expect="ok")
        for case, p, note in [
            ("missing_red", {"drop__red": True}, ""),
            ("missing_timestamps", {"drop__timestamps_sec": True}, ""),
            ("length_mismatch", {"green": list(g)[:-1]}, ""),
            ("empty_arrays", {"timestamps_sec": [], "red": [], "green": [], "blue": []}, ""),
            ("single_sample", {"timestamps_sec": [0.0], "red": [100.0], "green": [100.0], "blue": [100.0]}, ""),
            ("two_samples", {"timestamps_sec": [0.0, 0.033], "red": [100.0, 100.1],
             "green": [100.0, 100.1], "blue": [100.0, 100.1]}, ""),
            ("constant_channels", {"red": [100.0] * 900, "green": [100.0] * 900,
             "blue": [100.0] * 900}, "zero variance channels"),
            ("nan_green", {"green": with_sample(g, 450, NAN)}, "raw token"),
            ("inf_red", {"red": with_sample(r, 10, INF)}, "raw token"),
            ("vpc_length_mismatch", {"valid_pixel_counts": [1000, 1000]}, ""),
            ("vpc_negative", {"valid_pixel_counts": [-5] * 900}, "negative into Vec<usize>"),
            ("ts_unsorted", {"timestamps_sec": list(ts)[::-1]}, "reversed timestamps"),
            ("ts_duplicate", {"timestamps_sec": [0.0] * 900}, "all dt=0 -> fs=inf?"),
            ("algo_bogus", {"config": {"algorithm": "bogus"}}, ""),
            ("algo_green", {"config": {"algorithm": "green"}}, "valid control"),
            ("algo_pos", {"config": {"algorithm": "pos"}}, "valid control"),
            ("window_sec_zero", {"config": {"window_sec": 0.0}}, ""),
            ("window_sec_negative", {"config": {"window_sec": -2.0}}, ""),
            ("step_sec_zero", {"config": {"step_sec": 0.0}}, "potential infinite loop"),
            ("step_sec_negative", {"config": {"step_sec": -1.0}}, "potential infinite loop"),
            ("step_gt_window", {"config": {"window_sec": 2.0, "step_sec": 10.0}}, ""),
            ("min_window_fraction_gt1", {"config": {"min_window_fraction": 1.5}}, ""),
            ("min_window_fraction_negative", {"config": {"min_window_fraction": -0.5}}, ""),
            ("short_recording", {"timestamps_sec": list(ts[:60]), "red": list(r[:60]),
             "green": list(g[:60]), "blue": list(b[:60])}, "2s < default window"),
        ]:
            payload = mut(b2, **p)
            if op == "rppg-polarity":
                cfgm = dict(payload.get("config") or {})
                cfgm.setdefault("polarity", "normal")
                payload["config"] = cfgm
            add(op, "input" if case in ("missing_red", "missing_timestamps", "length_mismatch",
                "empty_arrays", "single_sample", "two_samples", "constant_channels",
                "nan_green", "inf_red", "vpc_length_mismatch", "vpc_negative",
                "ts_unsorted", "ts_duplicate", "short_recording") else "config",
                case, json.dumps(p, default=str), payload, notes=note)
    # polarity-specific
    for case, pol in [("polarity_bogus", "bogus"), ("polarity_auto", "auto"),
                      ("polarity_inverted", "inverted"), ("polarity_upper", "AUTO")]:
        add("rppg-polarity", "polarity", case, f"polarity={pol!r}",
            mut(base, config={"polarity": pol}),
            notes="normal|inverted|auto accepted" if pol != "bogus" else "")


build_signal_matrix()
build_ecg_cases()
build_ppg_cases()
build_eda_cases()
build_rsp_cases()
build_filter_cases()
build_entropy_cases()
build_signal_peaks_cases()
build_hrv_cases()
build_hrv_correct_cases()
build_rppg_cases()

# ---------------------------------------------------------------------------
# Runner
# ---------------------------------------------------------------------------

def run_case(c) -> dict:
    op, case = c["op"], c["case"]
    text = to_json_text(c["payload"])
    in_path = IN_DIR / f"{op}__{case}.json"
    out_path = RAW_DIR / f"{op}__{case}.out.json"
    in_path.write_text(text)
    t0 = time.monotonic()
    try:
        proc = subprocess.run(
            [str(BIN), "--op", op, "--input", str(in_path), "--output", str(out_path)],
            capture_output=True, text=True, timeout=TIMEOUT_SEC)
        dt = time.monotonic() - t0
        rc = proc.returncode
        stderr = (proc.stderr or "")[-500:]
    except subprocess.TimeoutExpired:
        return dict(outcome="timeout", error_kind="", error_message="",
                    exit_code="", stderr=f">{TIMEOUT_SEC}s", dt=TIMEOUT_SEC)
    if rc < 0:
        return dict(outcome="crash", error_kind="", error_message="",
                    exit_code=rc, stderr=f"killed by signal {-rc}; {stderr}", dt=dt)
    if not out_path.exists():
        return dict(outcome="crash", error_kind="", error_message="",
                    exit_code=rc, stderr=f"no output file; stderr: {stderr}", dt=dt)
    try:
        env = json.loads(out_path.read_text())
    except Exception as e:
        return dict(outcome="crash", error_kind="", error_message="",
                    exit_code=rc, stderr=f"unparseable output: {e}", dt=dt)
    if env.get("ok"):
        res = env.get("result") or {}
        summary = summarize_result(res)
        return dict(outcome="ok", error_kind="", error_message="",
                    exit_code=rc, stderr=summary, dt=dt)
    err = env.get("error") or {}
    return dict(outcome=err.get("kind", "?"), error_kind=err.get("kind", ""),
                error_message=err.get("message", ""), exit_code=rc, stderr=stderr, dt=dt)


def summarize_result(res: dict) -> str:
    parts = []
    for k, v in res.items():
        if isinstance(v, list):
            if v and isinstance(v[0], (int, float)):
                a = np.asarray([x for x in v if x is not None], dtype=float)
                nul = sum(1 for x in v if x is None)
                fin = np.isfinite(a).all() if a.size else True
                parts.append(f"{k}[{len(v)}]{' finite' if fin else ' NONFINITE'}{' nulls='+str(nul) if nul else ''}")
            else:
                parts.append(f"{k}[{len(v)}]")
        else:
            parts.append(f"{k}={v}")
    return "; ".join(parts)[:400]


# --- judgment tables (derived from first-pass inspection + source review) ---
# Wrong-variant BUG-002 pattern: finite-but-out-of-range config -> NonFiniteInput
WRONG_NONFINITE = {
    ("eda-peaks", "min_amplitude_negative"), ("eda-peaks", "min_prominence_negative"),
    ("ppg-peaks", "alpha_negative"), ("rsp-cycles", "min_amplitude_negative"),
    ("rppg-algorithm", "min_window_fraction_gt1"),
    ("rppg-algorithm", "min_window_fraction_negative"),
    ("rppg-polarity", "min_window_fraction_gt1"),
    ("rppg-polarity", "min_window_fraction_negative"),
    ("hrv-correct", "pt_zero"), ("hrv-correct", "pt_one"),
    ("hrv-correct", "pt_gt1"), ("hrv-correct", "pt_negative"),
    ("signal-peaks", "min_prominence_negative"),
}
# InvalidWindowSize(0) blaming an unrelated concept (time/interval params,
# inverted min/max relations) and echoing value 0 that was never supplied
WRONG_WINDOW_P2 = {
    ("eda-peaks", "min_distance_negative"), ("eda-peaks", "min_rise_negative"),
    ("eda-peaks", "rise_inverted"),
    ("rsp-cycles", "min_gt_max_interval"), ("rsp-cycles", "min_interval_zero"),
    ("rsp-cycles", "min_interval_negative"), ("rsp-cycles", "max_interval_negative"),
}
# InvalidWindowSize(0): right area but echoes 0 for a nonzero value or
# slightly mislabels the parameter concept
WRONG_WINDOW_P3 = {
    ("ecg-peaks", "integration_window_negative"), ("ecg-peaks", "refractory_negative"),
    ("ecg-peaks", "refractory_zero"),
    ("ppg-peaks", "w_peak_negative"), ("ppg-peaks", "w_beat_negative"),
    ("rppg-algorithm", "window_sec_negative"), ("rppg-algorithm", "step_sec_zero"),
    ("rppg-algorithm", "step_sec_negative"), ("rppg-algorithm", "step_gt_window"),
    ("rppg-polarity", "window_sec_negative"), ("rppg-polarity", "step_sec_zero"),
    ("rppg-polarity", "step_sec_negative"), ("rppg-polarity", "step_gt_window"),
    ("eda-peaks", "max_rise_zero"),
}
# ok but the input is invalid / output silently meaningless
SILENT_BAD = {
    ("sample-entropy", "m_zero"): "P2",          # contract requires m>=1; m=0 accepted
    ("ppg-peaks", "alpha_zero"): "P3",
    ("ppg-peaks", "alpha_gt1"): "P3",
    ("hrv-correct", "classify_threshold_negative"): "P3",
    ("hrv-correct", "classify_threshold_zero"): "P3",
    ("hrv-correct", "classify_threshold_gt1"): "P3",
    ("ppg-clean", "fs_huge"): "P3",              # output silently ~0 (biquad degenerate)
    ("eda-clean", "fs_huge"): "P3",
    ("filter", "fs_huge"): "P3",
}
# specific lamina errors whose message misidentifies the actual problem
MISLEADING_OTHER = {
    ("hrv-correct", "rr_all_artifact_reject"): ("no", "P3"),   # EmptySignal blames wrong thing
    ("hrv-correct", "nothing_given"): ("no", "P3"),            # blames sampling_rate, real gap: no rr/peaks
}


def judge(c, r) -> tuple[str, str]:
    """(error_correct, severity)."""
    key = (c["op"], c["case"])
    oc = r["outcome"]
    msg = r["error_message"]
    if oc == "panic":
        return ("no", "P1")
    if oc == "crash":
        return ("no", "P0")
    if oc == "timeout":
        return ("no", "P1")
    if c["expect"] == "ok":
        return ("n-a", "") if oc == "ok" else ("no", "P3")
    if oc == "ok":
        if key in SILENT_BAD:
            return ("no", SILENT_BAD[key])
        return ("n-a", "")
    if key in MISLEADING_OTHER:
        return MISLEADING_OTHER[key]
    if oc == "bad_request":
        return ("yes", "")  # all observed bad_request messages name the offending field/JSON
    # lamina_error
    if key in WRONG_NONFINITE:
        return ("no", "P2")
    if key in WRONG_WINDOW_P2:
        return ("no", "P2")
    if key in WRONG_WINDOW_P3:
        return ("no", "P3")
    if "non-finite" in msg:
        return ("no", "P2")  # safety net for the BUG-002 pattern
    return ("yes", "")


def main():
    if not BIN.exists():
        sys.exit(f"bridge binary missing: {BIN}")
    rows = []
    for i, c in enumerate(CASES):
        r = run_case(c)
        ec, sev = judge(c, r)
        rows.append({
            "op": c["op"], "case_category": c["category"], "case": c["case"],
            "params": c["params"], "outcome": r["outcome"], "error_kind": r["error_kind"],
            "error_message": r["error_message"].replace("\n", " | "),
            "error_correct": ec, "severity": sev,
            "exit_code": r["exit_code"], "detail": r["stderr"].replace("\n", " | "),
            "notes": c["notes"],
        })
        flag = "" if r["outcome"] == "ok" else f"  -> {r['outcome']}: {r['error_message'][:90]}"
        print(f"[{i+1:3d}/{len(CASES)}] {c['op']:16s} {c['case']:28s} {r['outcome']:12s}{flag}")
        # prune raw output files for plain-ok cases (keep failures/panics as evidence)
        out_path = RAW_DIR / f"{c['op']}__{c['case']}.out.json"
        if r["outcome"] == "ok" and out_path.exists():
            out_path.unlink()
        if r["outcome"] == "ok":
            in_path = IN_DIR / f"{c['op']}__{c['case']}.json"
            if in_path.exists():
                in_path.unlink()
    cols = ["op", "case_category", "case", "params", "outcome", "error_kind",
            "error_message", "error_correct", "severity", "notes", "exit_code", "detail"]
    with open(HERE / "api_results.csv", "w", newline="") as f:
        w = csv.DictWriter(f, fieldnames=cols)
        w.writeheader()
        for row in rows:
            w.writerow(row)
    from collections import Counter
    print("\nTotal rows:", len(rows))
    print(Counter(r["outcome"] for r in rows))


if __name__ == "__main__":
    main()
