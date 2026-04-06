import json
import os
import neurokit2 as nk

def generate_golden_data():
    # Attempt to fetch sample datasets using neurokit natively
    print("Generating pure ECG data...")
    ecg = nk.ecg_simulate(duration=10, sampling_rate=100, heart_rate=70)
    
    # Process it via python's neurokit
    print("Processing ECG via NeuroKit2...")
    signals, info = nk.ecg_process(ecg, sampling_rate=100)
    
    # We want to export the raw signal and expected peak indices to test Rust functionality
    expected_peaks = info["ECG_R_Peaks"].tolist() if "ECG_R_Peaks" in info else []
    
    golden_data = {
        "sampling_rate": 100,
        "raw_signal": ecg.tolist(),
        "cleaned_signal": signals["ECG_Clean"].tolist(),
        "expected_r_peaks": expected_peaks
    }
    
    output_path = os.path.join(os.path.dirname(__file__), "golden_ecg.json")
    with open(output_path, "w") as f:
        json.dump(golden_data, f)
        
    print(f"Golden dataset written to {output_path}")

if __name__ == "__main__":
    generate_golden_data()
