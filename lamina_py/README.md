# Lamina Python Bindings (`lamina_py`)

High-performance CPython bindings for **Lamina**, a Rust scientific biosignal processing computational engine. 

`lamina_py` provides zero-copy NumPy array interoperability, strict type safety, and fast native execution for physiological signal analysis across electrocardiography (ECG), photoplethysmography (PPG), electrodermal activity (EDA/GSR), respiration (RSP), heart rate variability (HRV), and remote PPG (rPPG).

---

## Architecture

- **Computational Engine**: Core Rust `lamina` crate (`src/`).
- **Native Extension**: `lamina._lamina` (PyO3 + Maturin + NumPy C-API).
- **Idiomatic Python Interface**: `lamina` Python package (`python/lamina/`).

---

## Prerequisites

- **Python**: `3.9` or higher.
- **Rust Toolchain**: `cargo` and `rustc` (stable). Install via [rustup.rs](https://rustup.rs) if missing.

---

## Installation

### Option 1: Development / Editable Installation (Recommended for Local Use)

Clone the repository and build the native extension module in release mode using `maturin`:

```bash
# Navigate to lamina_py directory
cd lamina_py

# Install build dependencies
pip install maturin numpy or uv add maturin numpy

# Build and install the extension into your active virtual environment
maturin develop --release
```

Alternatively, standard editable `pip` installation is supported:

```bash
cd lamina_py
pip install -e .
```

### Option 2: Build Production Wheels

To build redistributable `.whl` binaries:

```bash
cd lamina_py
maturin build --release
pip install target/wheels/lamina-*.whl
```

---

## Quickstart & Usage Examples

### 1. ECG Beat Detection & HRV Analysis

```python
import numpy as np
import lamina

# Sample ECG signal (e.g., 250 Hz acquisition)
raw_ecg = np.sin(np.linspace(0, 50, 2500)) + 0.1 * np.random.randn(2500)
sampling_rate = 250.0

# 1. Bandpass filter and baseline removal
cleaned_ecg = lamina.ecg.clean(raw_ecg, sampling_rate=sampling_rate)

# 2. Detect R-peak sample indices
r_peaks = lamina.ecg.findpeaks(cleaned_ecg, sampling_rate=sampling_rate)
print(f"Detected R-peaks at sample indices: {r_peaks}")

# 3. Convert peaks to inter-beat intervals (in milliseconds)
rr_intervals = lamina.hrv.peaks_to_intervals(r_peaks, sampling_rate=sampling_rate)

# 4. Compute HRV metrics (RMSSD and Mean NN in seconds)
rmssd_val = lamina.hrv.rmssd(rr_intervals)
mean_nn_val = lamina.hrv.mean_nn(rr_intervals)
print(f"RMSSD: {rmssd_val:.2f} ms | Mean NN: {mean_nn_val:.2f} ms")
```

### 2. Electrodermal Activity (EDA / GSR) Decomposition

```python
import lamina

# Clean raw EDA signal (100 Hz sampling rate)
cleaned_eda = lamina.eda.clean(raw_eda, sampling_rate=100.0)

# Decompose into tonic (Skin Conductance Level) and phasic (Skin Conductance Response)
components = lamina.eda.decompose(cleaned_eda, sampling_rate=100.0)
scl_tonic = components.tonic
scr_phasic = components.phasic

# Detect individual SCR peak events
events = lamina.eda.findpeaks_events(cleaned_eda, sampling_rate=100.0)
for ev in events:
    print(f"SCR Event: onset={ev.onset_index}, peak={ev.peak_index}, amp={ev.amplitude:.3f} µS")
```

### 3. Respiration Rate & Breath Cycle Extraction

```python
import lamina

# Clean respiration waveform (100 Hz)
cleaned_rsp = lamina.rsp.clean(raw_rsp, sampling_rate=100.0)

# Extract breath cycles (inspiration, expiration, amplitude, rate in BPM)
cycles = lamina.rsp.cycles(cleaned_rsp, sampling_rate=100.0)
for c in cycles:
    print(f"Breath Cycle: rate={c.respiratory_rate_bpm:.1f} BPM, amp={c.amplitude:.3f}")
```

### 4. General Signal Filtering & Peak Detection

```python
import lamina

# Butterworth zero-phase filtering (bandpass 0.5 - 40 Hz)
filtered = lamina.signal.filtfilt(
    signal, sampling_rate=250.0, low_cutoff=0.5, high_cutoff=40.0, order=2
)

# Generic peak detection with minimum distance constraint
peaks = lamina.signal.findpeaks(filtered, sampling_rate=250.0, min_distance_sec=0.4)
```

---

## API Module Directory

| Submodule | Description | Key Functions & Configs |
| :--- | :--- | :--- |
| `lamina.ecg` | Electrocardiogram cleaning & peak detection | `clean()`, `findpeaks()`, `findpeaks_mask()`, `EcgPeakDetectionConfig` |
| `lamina.ppg` | Photoplethysmography pulse processing | `clean()`, `findpeaks()`, `findpeaks_mask()`, `PpgPeakDetectionConfig` |
| `lamina.eda` | Electrodermal Activity (GSR) decomposition | `clean()`, `decompose()`, `phasic()`, `findpeaks()`, `findpeaks_events()`, `EdaDecompositionConfig` |
| `lamina.rsp` | Respiration cycle & rate extraction | `clean()`, `findpeaks()`, `cycles()`, `rate()`, `RspCleaningConfig`, `RspProcessingConfig` |
| `lamina.hrv` | Heart Rate Variability metrics | `peaks_to_intervals()`, `indices_to_intervals()`, `rmssd()`, `mean_nn()` |
| `lamina.signal` | Zero-phase filtering & digital signal processing | `filter()`, `filtfilt()`, `smooth_moving_average()`, `findpeaks()`, `PeakDetectionConfig` |
| `lamina.complexity` | Nonlinear complexity metrics | `sample_entropy()` |
| `lamina.rppg` | Remote PPG video extraction & BVP estimation | `algorithm()`, `polarity()` |
| `lamina.autonomic` | Stateful autonomic nervous system state estimation | `estimate()` |

---

## Testing & Verification

Run the test suite using `pytest` from the `lamina_py` directory:

```bash
# Ensure native extension is built first
maturin develop --release

# Run pytest suite
pytest
```

To run type checking and linting:

```bash
mypy python/lamina
ruff check python/lamina
```
