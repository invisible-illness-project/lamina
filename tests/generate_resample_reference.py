import json
import os
import numpy as np
from scipy import signal as scipy_signal

def generate_reference_data():
    output_configs = []

    np.random.seed(42)
    fs_in = 500.0
    n = int(fs_in * 2.0) # 2 seconds
    t = np.arange(n) / fs_in

    sig_pulse = np.sin(2 * np.pi * 1.5 * t)
    sig_high = 0.5 * np.sin(2 * np.pi * 100.0 * t) # 100 Hz tone (above 62.5 Hz Nyquist of 125 Hz)
    sig_mixed = sig_pulse + sig_high

    sig_impulse_start = np.zeros(n)
    sig_impulse_start[2] = 1.0

    sig_impulse_mid = np.zeros(n)
    sig_impulse_mid[n // 2] = 1.0

    sig_impulse_end = np.zeros(n)
    sig_impulse_end[n - 3] = 1.0

    test_signals = {
        "mixed": sig_mixed.tolist(),
        "impulse_start": sig_impulse_start.tolist(),
        "impulse_mid": sig_impulse_mid.tolist(),
        "impulse_end": sig_impulse_end.tolist(),
    }

    # Resampling ratios: 500 Hz -> 125 Hz (1:4), 100 Hz -> 250 Hz (5:2), 250 Hz -> 125 Hz (1:2)
    ratios = [(125, 500), (250, 100), (125, 250)]

    for up, down in ratios:
        for name, sig in test_signals.items():
            arr = np.array(sig)
            resampled = scipy_signal.resample_poly(arr, up, down, window=('kaiser', 5.0))
            output_configs.append({
                "up": up,
                "down": down,
                "signal_name": name,
                "input_signal": sig,
                "expected_output": resampled.tolist(),
            })

    output_path = os.path.join(os.path.dirname(__file__), "golden_resample.json")
    with open(output_path, "w") as f:
        json.dump(output_configs, f, indent=2)

    print(f"Generated {len(output_configs)} golden resample test configurations -> {output_path}")

if __name__ == "__main__":
    generate_reference_data()
