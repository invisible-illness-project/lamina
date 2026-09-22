#!/usr/bin/env python3
"""
PulseLM PPG Preprocessing Pipeline & DSP Primitives Scientific Validation Audit Script
Commit: 179d69f14b9fb4b09769cc28d987302478c7c5e2

Performs quantitative differential testing against independent SciPy references,
boundary condition analysis, spectral alias rejection measurement, and Rust/Python parity checks.
Outputs validation artifacts to validation/pulselm/ and prints summary metrics.
"""

import os
import sys
import json
import math
import numpy as np
from scipy import signal as scipy_signal

import lamina
import lamina.signal as l_sig
import lamina.ppg as l_ppg

ARTIFACT_DIR = "validation/pulselm"
FILTER_DIR = os.path.join(ARTIFACT_DIR, "filter_reference")
RESAMPLE_DIR = os.path.join(ARTIFACT_DIR, "resampling_reference")
PIPELINE_DIR = os.path.join(ARTIFACT_DIR, "pipeline_reference")
REPORT_DIR = os.path.join(ARTIFACT_DIR, "reports")

for d in [FILTER_DIR, RESAMPLE_DIR, PIPELINE_DIR, REPORT_DIR]:
    os.makedirs(d, exist_ok=True)

def measure_errors(a, b):
    a = np.asarray(a, dtype=np.float64)
    b = np.asarray(b, dtype=np.float64)
    if len(a) != len(b):
        return {
            "max_abs_error": float("inf"),
            "rmse": float("inf"),
            "relative_error": float("inf"),
            "finiteness": False,
            "length_mismatch": True
        }
    abs_err = np.abs(a - b)
    max_abs_err = float(np.max(abs_err))
    rmse = float(np.sqrt(np.mean(abs_err ** 2)))
    norm_b = np.linalg.norm(b)
    rel_err = float(np.linalg.norm(a - b) / norm_b) if norm_b > 1e-12 else float(np.linalg.norm(a - b))
    return {
        "max_abs_error": max_abs_err,
        "rmse": rmse,
        "relative_error": rel_err,
        "finiteness": bool(np.all(np.isfinite(a)))
    }

print("==========================================================================")
print("             LAMINA DSP & PULSELM VALIDATION AUDIT (179d69f)              ")
print("==========================================================================")

# --------------------------------------------------------------------------
# AREA 1: Butterworth SOS / Zero-Phase Filtering Audit
# --------------------------------------------------------------------------
print("\n--- Auditing Area 1: Butterworth / Zero-Phase Filtering (filtfilt) ---")

filter_audit_results = []
np.random.seed(12345)

filter_configs = [
    ("lowpass", [8.0], 4, 125.0),
    ("lowpass", [8.0], 1, 125.0),
    ("lowpass", [8.0], 2, 125.0),
    ("lowpass", [8.0], 3, 125.0),
    ("lowpass", [8.0], 5, 125.0),
    ("lowpass", [8.0], 6, 125.0),
    ("highpass", [0.5], 4, 125.0),
    ("highpass", [2.0], 2, 500.0),
    ("bandpass", [0.5, 8.0], 3, 125.0),
    ("bandpass", [5.0, 15.0], 2, 500.0),
    ("notch", [59.0, 61.0], 2, 500.0),
]

for kind, cutoffs, order, fs in filter_configs:
    if kind == "lowpass":
        sos_scipy = scipy_signal.butter(order, cutoffs[0], btype='lowpass', fs=fs, output='sos')
    elif kind == "highpass":
        sos_scipy = scipy_signal.butter(order, cutoffs[0], btype='highpass', fs=fs, output='sos')
    elif kind == "bandpass":
        sos_scipy = scipy_signal.butter(order, cutoffs, btype='bandpass', fs=fs, output='sos')
    elif kind == "notch":
        sos_scipy = scipy_signal.butter(order, cutoffs, btype='bandstop', fs=fs, output='sos')

    t_curr = np.linspace(0, 5, int(5 * fs), endpoint=False)
    test_signals = {
        "sine": np.sin(2 * np.pi * 2.0 * t_curr),
        "multitone": np.sin(2 * np.pi * 1.0 * t_curr) + 0.5 * np.sin(2 * np.pi * 15.0 * t_curr),
        "impulse": np.zeros(len(t_curr)),
        "step": np.zeros(len(t_curr)),
        "chirp": scipy_signal.chirp(t_curr, f0=0.1, t1=5.0, f1=fs*0.4),
        "white_noise": np.random.randn(len(t_curr)),
        "dc_offset": np.sin(2 * np.pi * 2.0 * t_curr) + 100.0,
    }
    test_signals["impulse"][len(t_curr) // 4] = 1.0
    test_signals["step"][len(t_curr) // 4 :] = 1.0

    if len(t_curr) > 35:
        test_signals["short_signal"] = np.sin(2 * np.pi * 2.0 * t_curr[:35])

    for sig_name, sig_in in test_signals.items():
        try:
            expected = scipy_signal.sosfiltfilt(sos_scipy, sig_in)
            scipy_err = None
        except Exception as e:
            expected = None
            scipy_err = str(e)

        try:
            if kind == "lowpass":
                actual = l_sig.filtfilt(sig_in, fs, high_cutoff=cutoffs[0], order=order, btype="lowpass")
            elif kind == "highpass":
                actual = l_sig.filtfilt(sig_in, fs, low_cutoff=cutoffs[0], order=order, btype="highpass")
            elif kind == "bandpass":
                actual = l_sig.filtfilt(sig_in, fs, low_cutoff=cutoffs[0], high_cutoff=cutoffs[1], order=order, btype="bandpass")
            elif kind == "notch":
                actual = l_sig.filtfilt(sig_in, fs, low_cutoff=cutoffs[0], high_cutoff=cutoffs[1], order=order, btype="notch")
            lamina_err = None
        except Exception as e:
            actual = None
            lamina_err = str(e)

        if expected is not None and actual is not None:
            errs = measure_errors(actual, expected)
            status = "PASS" if errs["max_abs_error"] < 1e-3 else "PARTIAL" if errs["max_abs_error"] < 1e-1 else "FAIL"
            filter_audit_results.append({
                "kind": kind,
                "cutoffs": cutoffs,
                "order": order,
                "fs": fs,
                "signal_name": sig_name,
                "signal_length": len(sig_in),
                "metrics": errs,
                "status": status
            })
        else:
            status = "MATCHED_ERROR" if (expected is None and actual is None) else "DISCREPANCY"
            filter_audit_results.append({
                "kind": kind,
                "cutoffs": cutoffs,
                "order": order,
                "fs": fs,
                "signal_name": sig_name,
                "signal_length": len(sig_in),
                "scipy_err": scipy_err,
                "lamina_err": lamina_err,
                "status": status
            })

with open(os.path.join(FILTER_DIR, "filter_differential_results.json"), "w") as f:
    json.dump(filter_audit_results, f, indent=2)

valid_filter_metrics = [r["metrics"] for r in filter_audit_results if "metrics" in r]
max_filter_err = max([m["max_abs_error"] for m in valid_filter_metrics]) if valid_filter_metrics else 0.0
mean_filter_rmse = float(np.mean([m["rmse"] for m in valid_filter_metrics])) if valid_filter_metrics else 0.0
filter_pass_count = sum(1 for r in filter_audit_results if r["status"] == "PASS")

print(f"Filter Audit Total Cases:           {len(filter_audit_results)}")
print(f"Filter Passed (Max Error < 1e-3):   {filter_pass_count}/{len(filter_audit_results)}")
print(f"Filter Max Absolute Error vs SciPy: {max_filter_err:.6e}")
print(f"Filter Mean RMSE vs SciPy:          {mean_filter_rmse:.6e}")

# --------------------------------------------------------------------------
# AREA 2: Polyphase Rational Resampling Audit
# --------------------------------------------------------------------------
print("\n--- Auditing Area 2: Polyphase Resampling (resample_poly) ---")

resample_audit_results = []
ratios_to_test = [
    (125, 250),
    (125, 500),
    (125, 1000),
    (125, 256),
    (125, 128),
    (250, 125),
    (300, 200),
    (125, 125),
]

n_500 = 1000
t_500 = np.arange(n_500) / 500.0
resample_signals = {
    "sine_5hz": np.sin(2 * np.pi * 5.0 * t_500),
    "multitone": np.sin(2 * np.pi * 2.0 * t_500) + 0.5 * np.sin(2 * np.pi * 15.0 * t_500),
    "impulse": np.zeros(n_500),
    "step": np.zeros(n_500),
    "chirp": scipy_signal.chirp(t_500, f0=1.0, t1=2.0, f1=50.0),
    "noise": np.random.randn(n_500),
}
resample_signals["impulse"][50] = 1.0
resample_signals["step"][200:] = 1.0

for up, down in ratios_to_test:
    for sig_name, sig in resample_signals.items():
        expected = scipy_signal.resample_poly(sig, up, down, window=('kaiser', 5.0))
        actual = l_sig.resample_poly(sig, up, down)
        errs = measure_errors(actual, expected)
        expected_len = math.ceil(len(sig) * up / down)
        len_matched = len(actual) == expected_len

        resample_audit_results.append({
            "up": up,
            "down": down,
            "signal_name": sig_name,
            "input_len": len(sig),
            "expected_len": expected_len,
            "actual_len": len(actual),
            "length_correct": len_matched,
            "metrics": errs,
            "status": "PASS" if errs["max_abs_error"] < 1e-3 and len_matched else "FAIL"
        })

# Alias Rejection Test
print("Running Alias Rejection Test (500 Hz -> 125 Hz with 100 Hz component)...")
fs_in = 500.0
fs_out = 125.0
n_alias = 2000
t_alias = np.arange(n_alias) / fs_in
sig_alias_in = np.sin(2 * np.pi * 10.0 * t_alias) + 0.5 * np.sin(2 * np.pi * 30.0 * t_alias) + 0.8 * np.sin(2 * np.pi * 100.0 * t_alias)

resampled_alias = l_sig.resample_poly(sig_alias_in, 1, 4)
t_out = np.arange(len(resampled_alias)) / fs_out
dft_25hz = np.sum(resampled_alias * np.exp(-2j * np.pi * 25.0 * t_out)) * 2.0 / len(resampled_alias)
alias_25hz_amp = float(np.abs(dft_25hz))
attenuation_db = 20.0 * np.log10(alias_25hz_amp / 0.8) if alias_25hz_amp > 1e-12 else -100.0

alias_test_result = {
    "input_fs": fs_in,
    "target_fs": fs_out,
    "input_components_hz": [10.0, 30.0, 100.0],
    "target_nyquist_hz": fs_out / 2.0,
    "aliased_freq_in_output_hz": 25.0,
    "input_100hz_amplitude": 0.8,
    "output_25hz_amplitude": alias_25hz_amp,
    "attenuation_db": attenuation_db,
    "sufficiently_suppressed": alias_25hz_amp < 0.01
}

with open(os.path.join(RESAMPLE_DIR, "resampling_differential_results.json"), "w") as f:
    json.dump({
        "differential_tests": resample_audit_results,
        "alias_rejection_test": alias_test_result
    }, f, indent=2)

max_resample_err = max([r["metrics"]["max_abs_error"] for r in resample_audit_results])
mean_resample_rmse = float(np.mean([r["metrics"]["rmse"] for r in resample_audit_results]))
resample_pass_count = sum(1 for r in resample_audit_results if r["status"] == "PASS")

print(f"Resample Audit Total Cases:          {len(resample_audit_results)}")
print(f"Resample Passed (Max Error < 1e-3):  {resample_pass_count}/{len(resample_audit_results)}")
print(f"Resample Max Abs Error vs SciPy:     {max_resample_err:.6e}")
print(f"Resample Mean RMSE vs SciPy:         {mean_resample_rmse:.6e}")
print(f"100 Hz Alias Suppression Attenuation:{attenuation_db:.2f} dB (Output 25 Hz amp = {alias_25hz_amp:.6f})")

# --------------------------------------------------------------------------
# AREA 3: Waveform Segmentation Audit
# --------------------------------------------------------------------------
print("\n--- Auditing Area 3: Waveform Segmentation (segment_signal) ---")

segment_audit_results = []
seg_cases = [
    (100, 10, 10, "drop", 10, 10),
    (95, 10, 10, "drop", 9, 10),
    (95, 10, 10, "pad", 10, 10),
    (95, 10, 10, "keep", 10, 5),
    (5, 10, 10, "drop", 0, 0),
    (5, 10, 10, "pad", 1, 10),
    (5, 10, 10, "keep", 1, 5),
    (100, 10, 5, "drop", 19, 10),
    (100, 10, 20, "drop", 5, 10),
    (100, 1, 1, "drop", 100, 1),
]

for n_len, w_size, s_step, policy_str, exp_count, exp_last_len in seg_cases:
    sig_in = np.arange(n_len, dtype=np.float64)
    segs = l_sig.segment_signal(sig_in, w_size, s_step, policy_str)

    count_ok = len(segs) == exp_count
    last_len_ok = True if (exp_last_len is None or len(segs) == 0 or len(segs[-1]) == exp_last_len) else False

    indices_correct = True
    for idx, seg in enumerate(segs):
        start_expected = idx * s_step
        if policy_str == "pad" and idx == len(segs) - 1 and n_len % s_step != 0:
            unpadded_len = n_len - start_expected
            if not np.allclose(seg[:unpadded_len], sig_in[start_expected:]):
                indices_correct = False
            if not np.allclose(seg[unpadded_len:], 0.0):
                indices_correct = False
        else:
            if not np.allclose(seg, sig_in[start_expected : start_expected + len(seg)]):
                indices_correct = False

    status = "PASS" if count_ok and last_len_ok and indices_correct else "FAIL"
    segment_audit_results.append({
        "n_len": n_len,
        "w_size": w_size,
        "s_step": s_step,
        "policy": policy_str,
        "expected_count": exp_count,
        "actual_count": len(segs),
        "count_ok": count_ok,
        "last_len_ok": last_len_ok,
        "indices_correct": indices_correct,
        "status": status
    })

print(f"Segmentation Audit Cases:           {len(segment_audit_results)}")
print(f"Segmentation Passed:                {sum(1 for r in segment_audit_results if r['status'] == 'PASS')}/{len(segment_audit_results)}")

# --------------------------------------------------------------------------
# AREA 4: Per-segment DC Removal Audit
# --------------------------------------------------------------------------
print("\n--- Auditing Area 4: DC Removal (remove_dc) ---")

dc_audit_results = []
dc_test_cases = [
    ("sine_zero_mean", np.sin(np.linspace(0, 4*np.pi, 500))),
    ("positive_dc", np.sin(np.linspace(0, 4*np.pi, 500)) + 45.2),
    ("large_dc", np.sin(np.linspace(0, 4*np.pi, 500)) + 1e7),
    ("constant_signal", np.full(500, 12.34)),
    ("near_constant", np.full(500, 12.34) + np.random.randn(500) * 1e-15),
    ("single_sample", np.array([42.0])),
    ("random_signal", np.random.randn(1250) + 99.0),
]

for name, sig in dc_test_cases:
    out = l_sig.remove_dc(sig)
    mean_val = float(np.mean(out))
    mean_zero_ok = abs(mean_val) < 1e-12
    scipy_out = sig - np.mean(sig)
    errs = measure_errors(out, scipy_out)

    dc_audit_results.append({
        "case_name": name,
        "input_len": len(sig),
        "mean_residual": mean_val,
        "mean_zero_satisfied": mean_zero_ok,
        "metrics": errs,
        "status": "PASS" if mean_zero_ok and errs["max_abs_error"] < 1e-12 else "FAIL"
    })

print(f"DC Removal Total Cases:             {len(dc_audit_results)}")
print(f"DC Removal Max Mean Residual:       {max(abs(r['mean_residual']) for r in dc_audit_results):.6e}")

# --------------------------------------------------------------------------
# AREA 5: Min-Max Normalization Audit
# --------------------------------------------------------------------------
print("\n--- Auditing Area 5: Min-Max Normalization (minmax_scale) ---")

minmax_audit_results = []
norm_test_cases = [
    ("standard_sine", np.sin(np.linspace(0, 4*np.pi, 500)), (0.0, 1.0)),
    ("negative_range", np.sin(np.linspace(0, 4*np.pi, 500)), (-1.0, 1.0)),
    ("custom_range", np.random.randn(500), (10.0, 50.0)),
    ("large_magnitude", np.random.randn(500) * 1e6, (0.0, 1.0)),
    ("small_magnitude", np.random.randn(500) * 1e-6, (0.0, 1.0)),
]

for name, sig, (a, b) in norm_test_cases:
    out = l_sig.minmax_scale(sig, (a, b), "error")
    min_out = float(np.min(out))
    max_out = float(np.max(out))
    min_ok = abs(min_out - a) < 1e-12
    max_ok = abs(max_out - b) < 1e-12
    bounds_ok = np.all((out >= a - 1e-12) & (out <= b + 1e-12))

    minmax_audit_results.append({
        "case_name": name,
        "target_range": [a, b],
        "actual_min": min_out,
        "actual_max": max_out,
        "bounds_satisfied": bool(bounds_ok),
        "status": "PASS" if min_ok and max_ok and bounds_ok else "FAIL"
    })

flat_sig = np.full(500, 5.0)
deg_policies = ["error", "zero", "midpoint"]

deg_audit_results = []
for pol_name in deg_policies:
    try:
        res = l_sig.minmax_scale(flat_sig, (0.0, 1.0), pol_name)
        err = None
        min_v, max_v = float(np.min(res)), float(np.max(res))
    except Exception as e:
        res = None
        err = str(e)
        min_v, max_v = None, None

    deg_audit_results.append({
        "policy": pol_name,
        "raised_error": err is not None,
        "error_msg": err,
        "output_min": min_v,
        "output_max": max_v,
        "expected_behavior_satisfied": (
            (pol_name == "error" and err is not None) or
            (pol_name == "zero" and res is not None and np.allclose(res, 0.0)) or
            (pol_name == "midpoint" and res is not None and np.allclose(res, 0.5))
        )
    })

print(f"Min-Max Standard Cases Passed:      {sum(1 for r in minmax_audit_results if r['status'] == 'PASS')}/{len(minmax_audit_results)}")
print(f"Degenerate Policy Audit Handled:    {sum(1 for r in deg_audit_results if r['expected_behavior_satisfied'])}/3")

# --------------------------------------------------------------------------
# AREA 6: PulseLM End-to-End Pipeline Audit vs Independent SciPy Reference
# --------------------------------------------------------------------------
print("\n--- Auditing Area 6: PulseLM End-to-End Pipeline vs SciPy Reference ---")

def independent_scipy_pulselm(raw_signal, fs_in):
    resampled = scipy_signal.resample_poly(raw_signal, 125, int(fs_in), window=('kaiser', 5.0))
    sos_8hz = scipy_signal.butter(4, 8.0, btype='lowpass', fs=125.0, output='sos')
    filtered = scipy_signal.sosfiltfilt(sos_8hz, resampled)
    w_len = 1250
    n_segs = len(filtered) // w_len
    segments = [filtered[i*w_len : (i+1)*w_len] for i in range(n_segs)]
    out_segments = []
    for seg in segments:
        dc_free = seg - np.mean(seg)
        s_min = np.min(dc_free)
        s_max = np.max(dc_free)
        diff = s_max - s_min
        scaled = np.full_like(dc_free, 0.5) if diff < 1e-12 else (dc_free - s_min) / diff
        out_segments.append(scaled)
    return out_segments

pipeline_audit_results = []
test_sampling_rates = [250.0, 500.0, 1000.0]

for fs in test_sampling_rates:
    n_samples = int(fs * 35.0)
    t_synth = np.arange(n_samples) / fs
    ppg_synth = (
        1.0 * np.sin(2 * np.pi * 1.2 * t_synth) +
        0.3 * np.sin(2 * np.pi * 2.4 * t_synth) +
        0.5 * np.sin(2 * np.pi * 0.2 * t_synth) +
        0.2 * np.sin(2 * np.pi * 50.0 * t_synth) +
        150.0
    )

    scipy_windows = independent_scipy_pulselm(ppg_synth, fs)
    lamina_windows = l_ppg.preprocess_pulselm(ppg_synth, fs)

    count_match = len(lamina_windows) == len(scipy_windows)
    segment_errors = []
    for l_win, s_win in zip(lamina_windows, scipy_windows):
        segment_errors.append(measure_errors(l_win, s_win))

    max_abs = max([e["max_abs_error"] for e in segment_errors]) if segment_errors else 0.0
    rmse_val = float(np.mean([e["rmse"] for e in segment_errors])) if segment_errors else 0.0

    pipeline_audit_results.append({
        "input_fs": fs,
        "duration_sec": 35.0,
        "input_length": n_samples,
        "expected_window_count": len(scipy_windows),
        "actual_window_count": len(lamina_windows),
        "count_match": count_match,
        "max_abs_error": max_abs,
        "mean_rmse": rmse_val,
        "status": "PASS" if count_match and max_abs < 1e-3 else "FAIL"
    })

with open(os.path.join(PIPELINE_DIR, "pipeline_differential_results.json"), "w") as f:
    json.dump(pipeline_audit_results, f, indent=2)

print(f"PulseLM Pipeline Audited Sampling Rates: {test_sampling_rates}")
for r in pipeline_audit_results:
    print(f"  Fs={r['input_fs']} Hz: Segments={r['actual_window_count']}, Max Abs Error={r['max_abs_error']:.6e}, RMSE={r['mean_rmse']:.6e}, Status={r['status']}")

# --------------------------------------------------------------------------
# AREA 7: Pathological / Edge-Case Error Classification
# --------------------------------------------------------------------------
print("\n--- Auditing Area 7: Pathological & Edge-Case Error Classification ---")

edge_case_results = []
edge_cases_to_test = [
    ("empty_resample", lambda: l_sig.resample_poly([], 1, 2), "EMPTY_SIGNAL"),
    ("zero_rate_resample", lambda: l_sig.resample_poly([1.0, 2.0], 0, 2), "INVALID_SAMPLING_RATE"),
    ("nan_resample", lambda: l_sig.resample_poly([1.0, float('nan')], 1, 2), "NON_FINITE_INPUT"),
    ("empty_filter", lambda: l_sig.filtfilt([], 125.0, high_cutoff=8.0, order=4, btype="lowpass"), "EMPTY_SIGNAL"),
    ("zero_rate_filter", lambda: l_sig.filtfilt([1.0, 2.0, 3.0], 0.0, high_cutoff=8.0, order=4, btype="lowpass"), "INVALID_SAMPLING_RATE"),
    ("invalid_cutoff_above_nyquist", lambda: l_sig.filtfilt(np.ones(100), 100.0, high_cutoff=60.0, order=4, btype="lowpass"), "INVALID_CUTOFF"),
    ("short_signal_filter", lambda: l_sig.filtfilt(np.ones(10), 100.0, high_cutoff=8.0, order=4, btype="lowpass"), "INSUFFICIENT_SAMPLES"),
    ("empty_segment", lambda: l_sig.segment_signal([], 10, 10), "EMPTY_SIGNAL"),
    ("zero_window_segment", lambda: l_sig.segment_signal(np.ones(10), 0, 10), "INVALID_WINDOW_SIZE"),
    ("empty_dc", lambda: l_sig.remove_dc([]), "EMPTY_SIGNAL"),
    ("nan_dc", lambda: l_sig.remove_dc([1.0, float('nan')]), "NON_FINITE_INPUT"),
    ("empty_minmax", lambda: l_sig.minmax_scale([], (0.0, 1.0)), "EMPTY_SIGNAL"),
    ("degenerate_error_minmax", lambda: l_sig.minmax_scale(np.ones(100), (0.0, 1.0), "error"), "DEGENERATE_SIGNAL"),
]

for name, test_fn, exp_class in edge_cases_to_test:
    try:
        test_fn()
        outcome = "INCORRECT_NO_ERROR"
        err_msg = None
    except Exception as e:
        outcome = "CORRECT_ERROR_RAISED"
        err_msg = str(e)

    edge_case_results.append({
        "case_name": name,
        "expected_error_class": exp_class,
        "outcome": outcome,
        "error_message": err_msg,
        "status": "PASS" if outcome == "CORRECT_ERROR_RAISED" else "FAIL"
    })

print(f"Edge Case Robustness Passed:        {sum(1 for r in edge_case_results if r['status'] == 'PASS')}/{len(edge_case_results)}")

# --------------------------------------------------------------------------
# AREA 8: Spec Provenance & SHA-256 Hash Verification
# --------------------------------------------------------------------------
print("\n--- Auditing Spec Provenance & Hash Verification ---")

pipeline_inst = l_ppg.PulseLmPipeline()
hash_val = pipeline_inst.compute_sha256_hash()
print(f"PulseLM Default Spec SHA-256 Hash: {hash_val}")

audit_summary_artifact = {
    "commit": "179d69f14b9fb4b09769cc28d987302478c7c5e2",
    "filter_max_abs_error": max_filter_err,
    "resample_max_abs_error": max_resample_err,
    "resample_alias_attenuation_db": attenuation_db,
    "pipeline_max_abs_error": max([r["max_abs_error"] for r in pipeline_audit_results]),
    "spec_sha256_hash": hash_val,
}

with open(os.path.join(REPORT_DIR, "audit_summary.json"), "w") as f:
    json.dump({
        "summary": audit_summary_artifact,
        "edge_cases": edge_case_results
    }, f, indent=2)

print("\n==========================================================================")
print("AUDIT EXECUTED SUCCESSFULLY. Artifacts stored in validation/pulselm/")
print("==========================================================================")
