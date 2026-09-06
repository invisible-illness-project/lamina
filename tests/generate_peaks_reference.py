import json
import os
import numpy as np
from scipy import signal as scipy_signal

def generate_reference_data():
    fs = 100.0  # 100 Hz sampling rate
    n = 200     # 2 seconds
    t = np.arange(n) / fs

    # Synthetic signal with known multi-amplitude peaks
    # Peaks at indices 20 (ht=2.0), 50 (ht=5.0), 80 (ht=1.5), 120 (ht=6.0), 160 (ht=3.0)
    sig = np.zeros(n)
    sig[20] = 2.0
    sig[50] = 5.0
    sig[80] = 1.5
    sig[120] = 6.0
    sig[160] = 3.0

    # Add background low frequency wave
    sig += 0.5 * np.sin(2 * np.pi * 1.0 * t)

    # 1. Height filter (min height = 2.5)
    peaks_ht, _ = scipy_signal.find_peaks(sig, height=2.5)

    # 2. Distance filter (min distance = 40 samples)
    peaks_dist, _ = scipy_signal.find_peaks(sig, distance=40)

    # 3. Prominence filter (min prominence = 1.0)
    peaks_prom, _ = scipy_signal.find_peaks(sig, prominence=1.0)

    # 4. Combined height + distance + prominence (ht=2.0, dist=30, prom=1.0)
    peaks_comb, _ = scipy_signal.find_peaks(sig, height=2.0, distance=30, prominence=1.0)

    # Sine wave with high-frequency ripple
    ripple_sig = np.sin(2 * np.pi * 2.0 * t) + 0.1 * np.sin(2 * np.pi * 20.0 * t)
    peaks_ripple, _ = scipy_signal.find_peaks(ripple_sig, distance=20, prominence=0.2)

    reference_data = {
        "signal": sig.tolist(),
        "ripple_signal": ripple_sig.tolist(),
        "test_cases": [
            {
                "name": "height_filter",
                "min_height": 2.5,
                "expected_peaks": peaks_ht.tolist()
            },
            {
                "name": "distance_filter",
                "min_distance": 40,
                "expected_peaks": peaks_dist.tolist()
            },
            {
                "name": "prominence_filter",
                "min_prominence": 1.0,
                "expected_peaks": peaks_prom.tolist()
            },
            {
                "name": "combined_filter",
                "min_height": 2.0,
                "min_distance": 30,
                "min_prominence": 1.0,
                "expected_peaks": peaks_comb.tolist()
            },
            {
                "name": "ripple_filter",
                "min_distance": 20,
                "min_prominence": 0.2,
                "expected_peaks": peaks_ripple.tolist()
            }
        ]
    }

    output_path = os.path.join(os.path.dirname(__file__), "golden_peaks.json")
    with open(output_path, "w") as f:
        json.dump(reference_data, f, indent=2)

    print(f"Generated golden peak detection reference dataset -> {output_path}")

if __name__ == "__main__":
    generate_reference_data()
