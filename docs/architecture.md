# Lamina Architectural & Scientific Foundation

This document defines the architectural boundary, error semantics, scientific contracts, dependency decision matrix, and verification framework for **Lamina** — a safe, high-performance Rust library for physiological signal processing (ECG, PPG, EDA, RSP, HRV, and complexity).

---

## 1. Architectural Boundary

Lamina separates third-party DSP and numerical primitives from domain-specific physiological signal processing logic.

### 1.1 Architectural Layering

```
+-------------------------------------------------------------------------+
|                              External Crates                            |
|  (ndarray, statrs, biquad, find_peaks, realfft, rayon, criterion)      |
+-------------------------------------------------------------------------+
                                     |
                                     v
+-------------------------------------------------------------------------+
|                          Lamina DSP & Math Core                         |
|  (lamina::signal, lamina::error, lamina::filtfilt, lamina::smooth)      |
+-------------------------------------------------------------------------+
                                     |
                                     v
+-------------------------------------------------------------------------+
|                        Physiological Domain Layer                       |
|   +-------------------+  +-------------------+  +--------------------+  |
|   |    lamina::ecg    |  |    lamina::ppg    |  |    lamina::eda     |  |
|   +-------------------+  +-------------------+  +--------------------+  |
|   |    lamina::rsp    |  |    lamina::hrv    |  | lamina::complexity |  |
|   +-------------------+  +-------------------+  +--------------------+  |
+-------------------------------------------------------------------------+
                                     |
                                     v
+-------------------------------------------------------------------------+
|                           Public Facade API                             |
|  (ecg_find_r_peaks, ppg_find_systolic_peaks, hrv_rmssd, eda_phasic...)  |
+-------------------------------------------------------------------------+
```

### 1.2 Physiological Safety Boundary Principle

The generic peak detector in `src/signal/peaks.rs` **must not** encode domain-specific physiological assumptions.
- No refractory periods, heart rate limits, respiratory bounds, or SCR rise-time thresholds in `lamina::signal::peaks`.
- Physiological constraints belong exclusively in downstream modules (`lamina::ecg`, `lamina::ppg`, `lamina::eda`, `lamina::rsp`).

---

## 2. Digital Filtering & Zero-Phase `signal_filtfilt` Foundation

Lamina implements zero-phase IIR digital filtering via Second-Order Sections (SOS) Direct Form II Transposed (DF2T) and SciPy-style odd reflection padding.

### 2.1 Filter Design (`FilterSpec` & `SosFilter`)

- **Filter Types**: `FilterKind::LowPass`, `HighPass`, `BandPass`, `Notch`.
- **Validation**:
  - Sampling rate $F_s > 0.0$ and finite.
  - Cutoff frequencies $0 < f_c < \frac{F_s}{2}$ (Nyquist limit).
  - Filter order $N \ge 1$.
  - Lowcut < Highcut for Bandpass/Notch.

### 2.2 SciPy Parity Empirical Verification Results
- **Task 5A Filtering-Operation Parity**: Max Abs Error $L_\infty < 7.58 \times 10^{-13}$, Average RMS Error $< 3.67 \times 10^{-14}$.
- **Phase 0 Follow-up Note**: Task 1's `FilterSpec::bandpass` generates single-peak resonator sections via `biquad`. A dedicated $2N$-pole Butterworth bandpass $s$-to-$z$ bilinear transform SOS coefficient generator will be added in Task 3 to match SciPy Butterworth bandpass coefficients 1-to-1.

---

## 3. Generic Peak Detection Foundation (`signal_findpeaks_config`)

Generic peak detection encapsulates the [`find_peaks`](https://crates.io/crates/find_peaks) crate (`v0.1.5`) behind Lamina's public facade API.

### 3.1 Parameter Semantics
- **Index Semantics**: 0-indexed sample positions (`Vec<usize>`) relative to the original input array, returned in ascending chronological order.
- **Distance (`min_distance`)**: Minimum inter-peak spacing in **samples** (`usize`).
- **Height (`min_height`)**: Absolute minimum amplitude cutoff (`f64`).
- **Prominence (`min_prominence`)**: Minimum peak prominence (`f64`).
- **Width (`min_width`)**: Minimum horizontal peak width in **samples** (`usize`).
- **Threshold (`threshold`)**: Minimum vertical distance to immediate neighboring samples ($x[i] - \max(x[i-1], x[i+1]) \ge \text{threshold}$).

### 3.2 SciPy Parity Verification Results
- Verified 100% index parity against `scipy.signal.find_peaks` across multi-amplitude signals, noisy ripple signals, height bounds, distance bounds, and prominence bounds (`tests/golden_peaks.json`).

---

## 4. Dependency Decision Table

| Capability | Current Lamina | Candidate | Decision | Reason |
| :--- | :--- | :--- | :--- | :--- |
| **IIR Filter Design** | Manual FFT formula | `biquad` | **Adopt (Primitive)** | Provides stable Biquad/SOS coefficients & sample state logic; `#![no_std]` ready. |
| **Zero-Phase Filtering** | Unpadded FFT bin scaling | Custom + `biquad` | **Custom (Lamina)** | `filtfilt` forward-backward reflection & boundary padding is domain logic owned by Lamina. |
| **Peak Detection** | 3-point local maxima | `find_peaks` | **Adopt (Primitive)** | High quality prominence, distance, & height handling matching SciPy semantics (`v0.1.5`). |
| **Smoothing** | $O(N \cdot W)$ moving average | Custom | **Custom (Lamina)** | Optimized $O(N)$ sliding window accumulator natively implemented in `src/signal/smooth.rs`. |
| **Parallelism** | None (Single-threaded) | `rayon` | **Optional Feature** | Guard with `#[cfg(feature = "parallel")]` for heavy non-linear metrics like Sample Entropy. |
| **ECG / PPG / EDA** | Prototype heuristics | Custom | **Custom (Lamina)** | Physiological algorithms (Pan-Tompkins, Elgendi, cvxEDA/Highpass, SCR) must be owned by Lamina. |
| **HRV Analysis** | Basic time-domain | `cardio-rs` / `hrv-algos` | **Reject Dependency** | Native Lamina implementation guarantees clean API contracts, zero extraneous dependencies, and `no_std` flexibility. |

---

## 5. Error Semantics

Lamina uses a central `SignalError` type:
- `EmptySignal`
- `InvalidSamplingRate(f64)`
- `InvalidCutoffFrequency(String)`
- `InsufficientSamples { required: usize, provided: usize }`
- `InvalidWindowSize(usize)`
- `InvalidFilterOrder(usize)`
- `NonFiniteInput`
- `InsufficientPeaks { required: usize, provided: usize }`
