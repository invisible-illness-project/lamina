import json
import os
import numpy as np
from scipy import signal as scipy_signal

def generate_reference_data():
    output_configs = []

    # Sampling rates to test
    for fs in [100.0, 500.0]:
        n = int(fs * 2.0) # 2 seconds of signal
        t = np.arange(n) / fs

        constant_sig = np.full(n, 5.0)
        impulse_sig = np.zeros(n)
        impulse_sig[n // 2] = 1.0
        low_freq_sig = np.sin(2 * np.pi * 1.0 * t)   # 1 Hz
        high_freq_sig = np.sin(2 * np.pi * (fs * 0.25) * t)
        mixed_sig = low_freq_sig + 0.5 * high_freq_sig
        
        np.random.seed(42)
        noise_sig = np.random.randn(n)

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

        orders = [1, 2, 3, 4, 5, 6]

        # 1. Lowpass
        fc_lp = 5.0 if fs == 100.0 else 25.0
        for order in orders:
            sos = scipy_signal.butter(order, fc_lp, btype='lowpass', fs=fs, output='sos')
            for sig_name in ["mixed", "noise", "step", "constant"]:
                sig_arr = np.array(test_signals[sig_name])
                filtered = scipy_signal.sosfiltfilt(sos, sig_arr)
                output_configs.append({
                    "filter_type": "lowpass",
                    "cutoff": [fc_lp],
                    "order": order,
                    "fs": fs,
                    "signal_name": sig_name,
                    "input_signal": test_signals[sig_name],
                    "expected_output": filtered.tolist(),
                    "sos_coefficients": sos.tolist(),
                })

        # 2. Highpass
        fc_hp = 0.5 if fs == 100.0 else 2.0
        for order in orders:
            sos = scipy_signal.butter(order, fc_hp, btype='highpass', fs=fs, output='sos')
            for sig_name in ["mixed", "noise", "step"]:
                sig_arr = np.array(test_signals[sig_name])
                filtered = scipy_signal.sosfiltfilt(sos, sig_arr)
                output_configs.append({
                    "filter_type": "highpass",
                    "cutoff": [fc_hp],
                    "order": order,
                    "fs": fs,
                    "signal_name": sig_name,
                    "input_signal": test_signals[sig_name],
                    "expected_output": filtered.tolist(),
                    "sos_coefficients": sos.tolist(),
                })

        # 3. Bandpass
        cut_bp = [5.0, 15.0] if fs == 100.0 else [0.5, 40.0]
        for order in orders:
            sos = scipy_signal.butter(order, cut_bp, btype='bandpass', fs=fs, output='sos')
            for sig_name in ["mixed", "noise"]:
                sig_arr = np.array(test_signals[sig_name])
                filtered = scipy_signal.sosfiltfilt(sos, sig_arr)
                output_configs.append({
                    "filter_type": "bandpass",
                    "cutoff": cut_bp,
                    "order": order,
                    "fs": fs,
                    "signal_name": sig_name,
                    "input_signal": test_signals[sig_name],
                    "expected_output": filtered.tolist(),
                    "sos_coefficients": sos.tolist(),
                })

        # 4. Notch / Bandstop
        cut_notch = [18.0, 22.0] if fs == 100.0 else [48.0, 52.0]
        for order in orders:
            sos = scipy_signal.butter(order, cut_notch, btype='bandstop', fs=fs, output='sos')
            for sig_name in ["mixed", "noise"]:
                sig_arr = np.array(test_signals[sig_name])
                filtered = scipy_signal.sosfiltfilt(sos, sig_arr)
                output_configs.append({
                    "filter_type": "notch",
                    "cutoff": cut_notch,
                    "order": order,
                    "fs": fs,
                    "signal_name": sig_name,
                    "input_signal": test_signals[sig_name],
                    "expected_output": filtered.tolist(),
                    "sos_coefficients": sos.tolist(),
                })

    output_path = os.path.join(os.path.dirname(__file__), "golden_filter.json")
    with open(output_path, "w") as f:
        json.dump(output_configs, f, indent=2)

    print(f"Generated {len(output_configs)} golden filter test configurations -> {output_path}")

if __name__ == "__main__":
    generate_reference_data()
