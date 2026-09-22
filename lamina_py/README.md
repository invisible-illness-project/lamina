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

# Install build dependencies (pip)
pip install maturin numpy

# ...or, if you use uv:
uv add maturin numpy

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
```

Because `lamina_py` lives in a Cargo workspace, the wheel is written to the
**workspace-root** `target/wheels/` directory — not `lamina_py/target/`:

```bash
pip install ../target/wheels/lamina-*.whl
```

---

## Quickstart & Usage Examples

All examples below are self-contained — paste them into a Python session with
`lamina` installed and they run as-is.

### 1. ECG Beat Detection & HRV Analysis

```python
import numpy as np
import lamina

# Synthetic ECG: R-spikes (~60 BPM with slight jitter) + baseline wander + noise.
sampling_rate = 250.0
t = np.arange(0.0, 10.0, 1.0 / sampling_rate)
rng = np.random.default_rng(42)
ecg = np.zeros_like(t)
for bt in np.cumsum(rng.uniform(0.9, 1.05, size=12)):
    if bt >= 9.5:
        continue
    ecg += np.exp(-((t - bt) ** 2) / (2 * 0.04 ** 2))  # ~95 ms QRS complex
ecg += 0.05 * np.sin(2 * np.pi * 0.3 * t) + 0.02 * rng.standard_normal(len(t))

# 1. Bandpass filter and baseline removal
cleaned_ecg = lamina.ecg.clean(ecg, sampling_rate=sampling_rate)

# 2. Detect R-peak sample indices. A 300 ms refractory period prevents the
#    detector from double-firing on a single QRS complex.
cfg = lamina.ecg.EcgPeakDetectionConfig(refractory_period_sec=0.3)
r_peaks = lamina.ecg.findpeaks(cleaned_ecg, sampling_rate=sampling_rate, config=cfg)
print(f"Detected {len(r_peaks)} R-peaks at: {np.round(r_peaks / sampling_rate, 2)} s")
# Detected 9 R-peaks at: [1.02 1.98 3.01 4.02 4.93 5.98 6.99 8.01 8.93] s

# 3. Convert peaks to inter-beat intervals (in milliseconds)
rr_intervals = lamina.hrv.peaks_to_intervals(r_peaks, sampling_rate=sampling_rate)

# 4. Compute HRV metrics (RMSSD and Mean NN in milliseconds)
rmssd_val = lamina.hrv.rmssd(rr_intervals)
mean_nn_val = lamina.hrv.mean_nn(rr_intervals)
print(f"RMSSD: {rmssd_val:.2f} ms | Mean NN: {mean_nn_val:.2f} ms")
# RMSSD: 77.92 ms | Mean NN: 989.00 ms  (~60.7 BPM)
```

### 2. General DSP: Filtering, Smoothing & Peak Detection (`lamina.signal`)

Every filter in `lamina.signal` is a **zero-phase Butterworth** design applied
forward-backward; `lamina.signal.filter` and `lamina.signal.filtfilt` share the
same code path in this release, so both introduce no phase delay.

```python
import numpy as np
import lamina
from lamina import signal as dsp

# Composite test signal: 2 Hz physiology + 40 Hz interference + 60 Hz mains hum.
fs = 500.0
t = np.arange(0.0, 10.0, 1.0 / fs)
x = (
    np.sin(2 * np.pi * 2.0 * t)        # component we want
    + 0.3 * np.sin(2 * np.pi * 40.0 * t)  # HF interference
    + 0.2 * np.sin(2 * np.pi * 60.0 * t)  # mains hum
)

# 1. Zero-phase Butterworth filtering. btype: "lowpass" (default "bandpass"),
#    "highpass", "bandpass", "notch". Single-cutoff kinds use low_cutoff;
#    bandpass/notch require both low_cutoff and high_cutoff.
low = dsp.filter(x, sampling_rate=fs, low_cutoff=5.0, order=2, btype="lowpass")

# 2. Surgical removal of 60 Hz mains hum, leaving everything else intact.
notched = dsp.filter(x, sampling_rate=fs, low_cutoff=58.0, high_cutoff=62.0,
                     order=2, btype="notch")

# 3. Moving-average smoothing (alternative to IIR filtering).
smoothed = dsp.smooth_moving_average(notched, window_size=25)

# 4. Peak detection with distance, height, and prominence constraints.
peaks = dsp.findpeaks(low, sampling_rate=fs, min_distance_sec=0.3,
                      min_prominence=0.5)
print(f"{len(peaks)} peaks; first at samples {peaks[:4]}")
# 20 peaks; first at samples [ 64 313 563 813]  (2 Hz -> 1 peak every 250 samples)

# 5. Same detector, boolean-mask output (index-consistent with findpeaks).
mask = dsp.findpeaks_mask(low, sampling_rate=fs, min_distance_sec=0.3,
                          min_prominence=0.5)
assert np.array_equal(np.flatnonzero(mask), peaks)
```

Measured behavior of the snippet above (steady state, order-2 filter):

| Stage | 2 Hz (kept) | 40 Hz | 60 Hz |
| :--- | :--- | :--- | :--- |
| Raw | 222.2 | 18.18 | 12.12 |
| After 5 Hz lowpass | 216.7 | **0.004** | — |
| After 58–62 Hz notch | — | — | **0.0004** |

### 3. Electrodermal Activity (EDA / GSR) Decomposition

```python
import numpy as np
import lamina

# Synthetic EDA: slow tonic drift + two skin conductance responses (SCRs).
t = np.linspace(0.0, 20.0, 2000)
raw_eda = (
    2.0 + 0.05 * t
    + 0.5 * np.exp(-((t - 6.0) ** 2) / 0.5)
    + 0.8 * np.exp(-((t - 14.0) ** 2) / 0.5)
)

# Clean raw EDA signal (100 Hz sampling rate)
cleaned_eda = lamina.eda.clean(raw_eda, sampling_rate=100.0)

# Decompose into tonic (Skin Conductance Level) and phasic (Skin Conductance Response)
components = lamina.eda.decompose(cleaned_eda, sampling_rate=100.0)
scl_tonic = components.tonic
scr_phasic = components.phasic
print(f"Tonic SCL: {scl_tonic.min():.3f}-{scl_tonic.max():.3f} µS rising")

# Detect individual SCR peak events on the phasic component
# (findpeaks_events expects a phasic signal — feeding it the raw cleaned
# signal makes onsets fire on the tonic drift, several seconds early)
events = lamina.eda.findpeaks_events(scr_phasic, sampling_rate=100.0)
for ev in events:
    print(f"SCR Event: onset={ev.onset_index}, peak={ev.peak_index}, amp={ev.amplitude:.3f} µS")
# Tonic SCL: 2.072-2.891 µS rising
# SCR Event: onset=500, peak=599, amp=0.423 µS
# SCR Event: onset=1302, peak=1400, amp=0.687 µS
```

### 4. Respiration Rate & Breath Cycle Extraction

```python
import numpy as np
import lamina

# Synthetic respiration: 0.25 Hz (15 breaths/min) + noise.
t = np.arange(0.0, 60.0, 0.01)
raw_rsp = np.sin(2 * np.pi * 0.25 * t) + 0.05 * np.random.default_rng(1).standard_normal(len(t))

# Clean respiration waveform (100 Hz)
cleaned_rsp = lamina.rsp.clean(raw_rsp, sampling_rate=100.0)

# Extract breath cycles (inspiration, expiration, amplitude, rate in BPM)
cycles = lamina.rsp.cycles(cleaned_rsp, sampling_rate=100.0)
for c in cycles[:3]:
    print(f"Breath Cycle: rate={c.respiratory_rate_bpm:.1f} BPM, amp={c.amplitude:.3f}")
# Breath Cycle: rate=15.2 BPM, amp=2.004
# Breath Cycle: rate=15.0 BPM, amp=2.015
# Breath Cycle: rate=15.0 BPM, amp=1.997
```

### 5. Multimodal Features & Autonomic State Estimation

```python
import numpy as np
import lamina

# ~2 min of heartbeats at ~60 BPM with realistic beat-to-beat jitter
# (fs=100 Hz, so inter-beat intervals of 85-115 samples).
rng = np.random.default_rng(7)
beats = np.cumsum(rng.uniform(85, 115, size=130).round().astype(int)).tolist()

inp = lamina.features.MultimodalInput()
inp.with_ecg(beats, sampling_rate=100.0)

# Window the recording into feature vectors. Windows must contain enough
# beats to be cardiac-valid (a 5 s window at 60 BPM is not); 15 s works.
fcfg = lamina.features.FeatureConfig(
    window=lamina.features.WindowConfig(window_duration_sec=15.0, step_sec=7.5)
)
feats = lamina.features.extract_features(inp, config=fcfg)
print(f"{len(feats)} windows, HR={feats[0].mean_hr_bpm:.1f} BPM")
# 16 windows, HR=60.5 BPM

# Build a baseline from early windows, then estimate later states.
baseline = lamina.autonomic.AutonomicBaseline.from_features(feats[:8])
estimator = lamina.autonomic.AutonomicEstimator(lamina.autonomic.AutonomicEstimatorConfig())
state = estimator.estimate(feats[10], baseline)
print(f"activation={state.activation_score:.3f} regulation={state.regulation_score:.3f}")
# activation=-0.170 regulation=0.645

# Or score a whole series in one call.
series = estimator.estimate_series(feats[8:], baseline)
print(f"{len(series.states)} states, {series.window_duration_sec:.1f}s windows")
# 8 states, 15.0s windows
```

Scores are normalized against the baseline you supply, and fields that cannot
be computed from the available modalities come back as `None` (e.g. EDA-derived
indices for ECG-only input).

---

## API Module Directory

| Submodule | Description | Key Functions & Configs |
| :--- | :--- | :--- |
| `lamina.signal` | Zero-phase Butterworth filtering, smoothing & generic peak detection | `filter()`, `filtfilt()`, `smooth_moving_average()`, `findpeaks()`, `findpeaks_mask()`, `PeakDetectionConfig` |
| `lamina.ecg` | Electrocardiogram cleaning & peak detection | `clean()`, `findpeaks()`, `findpeaks_mask()`, `EcgPeakDetectionConfig` |
| `lamina.ppg` | Photoplethysmography pulse processing | `clean()`, `findpeaks()`, `findpeaks_mask()`, `PpgPeakDetectionConfig` |
| `lamina.eda` | Electrodermal Activity (GSR) decomposition | `clean()`, `decompose()`, `phasic()`, `findpeaks()`, `findpeaks_events()`, `EdaDecompositionConfig` |
| `lamina.rsp` | Respiration cycle & rate extraction | `clean()`, `findpeaks()`, `cycles()`, `rate()`, `RspCleaningConfig`, `RspProcessingConfig` |
| `lamina.hrv` | Heart Rate Variability metrics | `peaks_to_intervals()`, `indices_to_intervals()`, `rmssd()`, `mean_nn()`, `classify_intervals()`, `clean_rr_intervals()`, `CorrectionPolicy` |
| `lamina.complexity` | Nonlinear complexity metrics | `sample_entropy()` |
| `lamina.features` | Windowed multimodal feature extraction | `extract_features()`, `MultimodalInput`, `WindowConfig`, `FeatureConfig` |
| `lamina.multimodal` | Cross-modal coupling, RSA & signal quality | `cardiorespiratory_phase_coupling()`, `multimodal_quality()`, `rsa()`, `ecg_ppg_timing()`, `evaluate_ecg_quality()` |
| `lamina.autonomic` | Stateful autonomic nervous system estimation | `AutonomicEstimator.estimate()` / `.estimate_series()`, `AutonomicBaseline.from_features()`, `AutonomicEstimatorConfig` |
| `lamina.rppg` | Remote PPG video extraction & BVP estimation | `extract_rppg()`, `VideoStream`, `VideoFrame`, `Roi`, `RppgConfig` |

---

## Testing & Verification

Run the test suite using `pytest` from the `lamina_py` directory:

```bash
# Ensure native extension is built first
maturin develop --release

# Run pytest suite
python -m pytest tests -q
```

To run type checking and linting:

```bash
mypy python/lamina
ruff check python/lamina
```
