# Lamina Python Bindings

Python bindings for the Lamina Rust scientific signal processing computational engine.

## Overview
Lamina provides high-performance Rust algorithms for physiological signal processing (ECG, PPG, EDA, Respiration), HRV analysis, nonlinear complexity, stateful autonomic estimation, rPPG video signal extraction, and multimodal biosignal feature extraction.

## Architecture
- Computational core: Rust `lamina` crate
- Native extension: `lamina._lamina` (PyO3 + Maturin + NumPy)
- Idiomatic interface: `lamina` Python package

## Usage
```python
import lamina

# Filter ECG signal
cleaned = lamina.ecg.clean(raw_ecg, sampling_rate=250.0)

# Detect R-peaks
r_peaks = lamina.ecg.findpeaks(cleaned, sampling_rate=250.0)

# Compute HRV RMSSD
rmssd = lamina.hrv.rmssd(lamina.hrv.peaks_to_intervals(r_peaks, sampling_rate=250.0))
```
