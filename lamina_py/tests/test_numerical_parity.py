import os
import json
import numpy as np
import pytest
import lamina

GOLDEN_ECG_PATH = os.path.join(os.path.dirname(__file__), "..", "..", "tests", "golden_ecg.json")
GOLDEN_PPG_PATH = os.path.join(os.path.dirname(__file__), "..", "..", "tests", "golden_ppg.json")
GOLDEN_EDA_PATH = os.path.join(os.path.dirname(__file__), "..", "..", "tests", "golden_eda.json")

def test_ecg_golden_numerical_parity():
    if not os.path.exists(GOLDEN_ECG_PATH):
        pytest.skip("golden_ecg.json not found")

    with open(GOLDEN_ECG_PATH, "r") as f:
        cases = json.load(f)

    for case in cases:
        fs = case["sampling_rate"]
        signal = np.array(case["signal"], dtype=np.float64)
        expected_r_peaks = case["expected_r_peaks"]

        cleaned = lamina.ecg.clean(signal, sampling_rate=fs)
        peaks = lamina.ecg.findpeaks(cleaned, sampling_rate=fs)

        tol_samples = int(round(0.150 * fs))
        tp = 0
        for exp in expected_r_peaks:
            found = any(abs(int(p) - int(exp)) <= tol_samples for p in peaks)
            if found:
                tp += 1
        
        recall = tp / len(expected_r_peaks)
        assert recall >= 0.75, f"Case {case['name']} failed recall check (recall = {recall:.2f})"

def test_ppg_golden_numerical_parity():
    if not os.path.exists(GOLDEN_PPG_PATH):
        pytest.skip("golden_ppg.json not found")

    with open(GOLDEN_PPG_PATH, "r") as f:
        cases = json.load(f)

    for case in cases:
        fs = case["sampling_rate"]
        signal = np.array(case["signal"], dtype=np.float64)
        expected_peaks = case["expected_systolic_peaks"]

        cleaned = lamina.ppg.clean(signal, sampling_rate=fs)
        peaks = lamina.ppg.findpeaks(cleaned, sampling_rate=fs)

        tol_samples = int(round(0.150 * fs))
        tp = 0
        for exp in expected_peaks:
            found = any(abs(int(p) - int(exp)) <= tol_samples for p in peaks)
            if found:
                tp += 1

        recall = tp / len(expected_peaks)
        assert recall >= 0.70, f"PPG Case {case['name']} failed recall check (recall = {recall:.2f})"

def test_eda_golden_numerical_parity():
    if not os.path.exists(GOLDEN_EDA_PATH):
        pytest.skip("golden_eda.json not found")

    with open(GOLDEN_EDA_PATH, "r") as f:
        cases = json.load(f)

    for case in cases:
        fs = case["sampling_rate"]
        signal = np.array(case["raw_signal"], dtype=np.float64)

        cleaned = lamina.eda.clean(signal, sampling_rate=fs)
        comp = lamina.eda.decompose(cleaned, sampling_rate=fs)

        assert len(comp.tonic) == len(signal)
        assert len(comp.phasic) == len(signal)
