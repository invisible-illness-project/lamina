import json
import os
import numpy as np
from scipy import signal as scipy_signal

def generate_reference_data():
    fs = 100.0  # 100 Hz sampling rate
    n = 200     # 2 seconds of signal
    t = np.arange(n) / fs

    # Signals
    constant_sig = np.full(n, 5.0)
    impulse_sig = np.zeros(n)
    impulse_sig[n // 2] = 1.0
    low_freq_sig = np.sin(2 * np.pi * 1.0 * t)   # 1 Hz
    high_freq_sig = np.sin(2 * np.pi * 25.0 * t) # 25 Hz
    mixed_sig = low_freq_sig + 0.5 * high_freq_sig
    
    np.random.seed(42)
    noise_sig = np.random.randn(n)

    # Step signal for boundary test
    step_sig = np.zeros(n)
    step_sig[n // 4:] = 2.0

    test_signals = {
        "constant": constant_sig.tolist(),
        "impulse": impulse_sig.tolist(),
        "low_freq": low_freq_sig.tolist(),
        "high_freq": high_freq_sig.tolist(),
        "mixed": mixed_sig.tolist(),
        "noise": noise_sig.tolist(),
        "step": step_sig.tolist(),
    }

    configurations = []

    # 1. Lowpass 5Hz, order 2, 4, 6
    for order in [2, 4, 6]:
        sos = scipy_signal.butter(order, 5.0, btype='lowpass', fs=fs, output='sos')
        for sig_name, sig in test_signals.items():
            sig_arr = np.array(sig)
            filtered = scipy_signal.sosfiltfilt(sos, sig_arr)
            configurations.append({
                "filter_type": "lowpass",
                "cutoff": [5.0],
                "order": order,
                "fs": fs,
                "signal_name": sig_name,
                "input_signal": sig,
                "expected_output": filtered.tolist(),
                "sos_coefficients": sos.tolist(),
            })

    # 2. Highpass 0.5Hz, order 2, 4
    for order in [2, 4]:
        sos = scipy_signal.butter(order, 0.5, btype='highpass', fs=fs, output='sos')
        for sig_name in ["mixed", "step", "noise"]:
            sig_arr = np.array(test_signals[sig_name])
            filtered = scipy_signal.sosfiltfilt(sos, sig_arr)
            configurations.append({
                "filter_type": "highpass",
                "cutoff": [0.5],
                "order": order,
                "fs": fs,
                "signal_name": sig_name,
                "input_signal": test_signals[sig_name],
                "expected_output": filtered.tolist(),
                "sos_coefficients": sos.tolist(),
            })

    # 3. Bandpass 0.5 - 8.0 Hz, order 2, 4
    for order in [2, 4]:
        sos = scipy_signal.butter(order, [0.5, 8.0], btype='bandpass', fs=fs, output='sos')
        for sig_name in ["mixed", "noise"]:
            sig_arr = np.array(test_signals[sig_name])
            filtered = scipy_signal.sosfiltfilt(sos, sig_arr)
            configurations.append({
                "filter_type": "bandpass",
                "cutoff": [0.5, 8.0],
                "order": order,
                "fs": fs,
                "signal_name": sig_name,
                "input_signal": test_signals[sig_name],
                "expected_output": filtered.tolist(),
                "sos_coefficients": sos.tolist(),
            })

    # 4. Bandstop/Notch 18-22 Hz, order 2
    sos_notch = scipy_signal.butter(2, [18.0, 22.0], btype='bandstop', fs=fs, output='sos')
    for sig_name in ["mixed", "noise"]:
        sig_arr = np.array(test_signals[sig_name])
        filtered = scipy_signal.sosfiltfilt(sos_notch, sig_arr)
        configurations.append({
            "filter_type": "notch",
            "cutoff": [18.0, 22.0],
            "order": 2,
            "fs": fs,
            "signal_name": sig_name,
            "input_signal": test_signals[sig_name],
            "expected_output": filtered.tolist(),
            "sos_coefficients": sos_notch.tolist(),
        })

    output_path = os.path.join(os.path.dirname(__file__), "golden_filter.json")
    with open(output_path, "w") as f:
        json.dump(configurations, f, indent=2)

    print(f"Generated {len(configurations)} golden filter test configurations -> {output_path}")

if __name__ == "__main__":
    generate_reference_data()
