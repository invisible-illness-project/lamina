import json
import os
import numpy as np
import neurokit2 as nk

def generate_rsp_golden():
    reference_cases = []

    test_params = [
        {"fs": 32.0, "resp_rate": 10.0, "name": "slow_10bpm_32hz"},
        {"fs": 64.0, "resp_rate": 15.0, "name": "normal_15bpm_64hz"},
        {"fs": 100.0, "resp_rate": 12.0, "name": "normal_12bpm_100hz"},
        {"fs": 250.0, "resp_rate": 20.0, "name": "active_20bpm_250hz"},
        {"fs": 500.0, "resp_rate": 24.0, "name": "tachy_24bpm_500hz"},
    ]

    for p in test_params:
        fs = p["fs"]
        resp_rate = p["resp_rate"]
        name = p["name"]

        duration = 20
        rsp_signal = nk.rsp_simulate(duration=duration, sampling_rate=int(fs), respiratory_rate=resp_rate, noise=0.01)

        # Process via NeuroKit2
        signals, info = nk.rsp_process(rsp_signal, sampling_rate=int(fs))
        expected_peaks = [int(idx) for idx in info["RSP_Peaks"] if not np.isnan(idx)]

        reference_cases.append({
            "name": name,
            "sampling_rate": fs,
            "respiratory_rate": resp_rate,
            "raw_signal": rsp_signal.tolist(),
            "expected_peaks": expected_peaks,
        })

    output_path = os.path.join(os.path.dirname(__file__), "golden_rsp.json")
    with open(output_path, "w") as f:
        json.dump(reference_cases, f, indent=2)

    print(f"Generated {len(reference_cases)} golden RSP reference test cases -> {output_path}")

if __name__ == "__main__":
    generate_rsp_golden()
