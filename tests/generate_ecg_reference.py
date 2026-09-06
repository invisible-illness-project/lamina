import json
import os
import numpy as np
import neurokit2 as nk

def generate_ecg_golden():
    reference_cases = []

    # Test cases: combinations of Fs, HR, noise
    test_params = [
        {"fs": 100.0, "hr": 60, "noise": 0.01, "name": "normal_60bpm_100hz"},
        {"fs": 100.0, "hr": 120, "noise": 0.02, "name": "tachy_120bpm_100hz"},
        {"fs": 100.0, "hr": 45, "noise": 0.01, "name": "brady_45bpm_100hz"},
        {"fs": 500.0, "hr": 70, "noise": 0.01, "name": "normal_70bpm_500hz"},
        {"fs": 500.0, "hr": 140, "noise": 0.02, "name": "tachy_140bpm_500hz"},
    ]

    for p in test_params:
        fs = p["fs"]
        hr = p["hr"]
        noise = p["noise"]
        name = p["name"]

        # 10 seconds of simulated ECG
        duration = 10
        ecg_signal = nk.ecg_simulate(duration=duration, sampling_rate=int(fs), heart_rate=hr, noise=noise)

        # Run Pan-Tompkins via NeuroKit2
        _, rpeaks_dict = nk.ecg_peaks(ecg_signal, sampling_rate=int(fs), method="pantompkins")
        expected_r_peaks = [int(idx) for idx in rpeaks_dict["ECG_R_Peaks"]]

        reference_cases.append({
            "name": name,
            "sampling_rate": fs,
            "heart_rate": hr,
            "noise": noise,
            "signal": ecg_signal.tolist(),
            "expected_r_peaks": expected_r_peaks,
        })

    output_path = os.path.join(os.path.dirname(__file__), "golden_ecg.json")
    with open(output_path, "w") as f:
        json.dump(reference_cases, f, indent=2)

    print(f"Generated {len(reference_cases)} golden ECG reference test cases -> {output_path}")

if __name__ == "__main__":
    generate_ecg_golden()
