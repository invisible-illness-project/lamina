import json
import os
import numpy as np
import neurokit2 as nk

def generate_eda_golden():
    reference_cases = []

    test_params = [
        {"fs": 32.0, "scr_rate": 6, "name": "normal_6scr_32hz"},
        {"fs": 64.0, "scr_rate": 10, "name": "active_10scr_64hz"},
        {"fs": 100.0, "scr_rate": 8, "name": "normal_8scr_100hz"},
        {"fs": 250.0, "scr_rate": 12, "name": "active_12scr_250hz"},
        {"fs": 500.0, "scr_rate": 6, "name": "normal_6scr_500hz"},
    ]

    for p in test_params:
        fs = p["fs"]
        scr_rate = p["scr_rate"]
        name = p["name"]

        duration = 10
        eda_signal = nk.eda_simulate(duration=duration, sampling_rate=int(fs), scr_number=scr_rate, drift=0.1, noise=0.01)

        # 1. Clean EDA
        eda_cleaned = nk.eda_clean(eda_signal, sampling_rate=int(fs), method="biosppy")
        # 2. Decompose Phasic
        phasic_dict = nk.eda_phasic(eda_cleaned, sampling_rate=int(fs), method="highpass")
        eda_phasic_sig = phasic_dict["EDA_Phasic"]
        # 3. Detect SCR Peaks
        _, peaks_dict = nk.eda_peaks(eda_phasic_sig, sampling_rate=int(fs), method="neurokit")
        expected_scr_peaks = [int(idx) for idx in peaks_dict["SCR_Peaks"] if not np.isnan(idx)]

        reference_cases.append({
            "name": name,
            "sampling_rate": fs,
            "scr_number": scr_rate,
            "raw_signal": eda_signal.tolist(),
            "expected_scr_peaks": expected_scr_peaks,
        })

    output_path = os.path.join(os.path.dirname(__file__), "golden_eda.json")
    with open(output_path, "w") as f:
        json.dump(reference_cases, f, indent=2)

    print(f"Generated {len(reference_cases)} golden EDA reference test cases -> {output_path}")

if __name__ == "__main__":
    generate_eda_golden()
