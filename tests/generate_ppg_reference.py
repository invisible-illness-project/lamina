import json
import os
import numpy as np
import neurokit2 as nk

def generate_ppg_golden():
    reference_cases = []

    # Test cases: combinations of Fs, HR, noise
    test_params = [
        {"fs": 100.0, "hr": 60, "name": "normal_60bpm_100hz"},
        {"fs": 100.0, "hr": 120, "name": "tachy_120bpm_100hz"},
        {"fs": 100.0, "hr": 45, "name": "brady_45bpm_100hz"},
        {"fs": 500.0, "hr": 75, "name": "normal_75bpm_500hz"},
        {"fs": 500.0, "hr": 130, "name": "tachy_130bpm_500hz"},
    ]

    for p in test_params:
        fs = p["fs"]
        hr = p["hr"]
        name = p["name"]

        duration = 10
        ppg_signal = nk.ppg_simulate(duration=duration, sampling_rate=int(fs), heart_rate=hr)

        # Run Elgendi via NeuroKit2
        _, ppg_peaks_dict = nk.ppg_peaks(ppg_signal, sampling_rate=int(fs), method="elgendi")
        expected_systolic_peaks = [int(idx) for idx in ppg_peaks_dict["PPG_Peaks"]]

        reference_cases.append({
            "name": name,
            "sampling_rate": fs,
            "heart_rate": hr,
            "signal": ppg_signal.tolist(),
            "expected_systolic_peaks": expected_systolic_peaks,
        })

    output_path = os.path.join(os.path.dirname(__file__), "golden_ppg.json")
    with open(output_path, "w") as f:
        json.dump(reference_cases, f, indent=2)

    print(f"Generated {len(reference_cases)} golden PPG reference test cases -> {output_path}")

if __name__ == "__main__":
    generate_ppg_golden()
