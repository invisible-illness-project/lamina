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

Lamina implements digital Butterworth filter design and zero-phase IIR digital filtering via Second-Order Sections (SOS) Direct Form II Transposed (DF2T) with SciPy-style odd reflection padding.

### 2.1 Butterworth Filter Design (`FilterSpec` & `SosFilter`)

- **Filter Types**: `FilterKind::LowPass`, `HighPass`, `BandPass`, `Notch`.
- **Design Pipeline**:
  1. **Analog Prototype Poles**: $p_k = \exp\left(j \frac{(2k + 1 + N)\pi}{2N}\right)$ for $k = 0 \dots N-1$. All prototype poles satisfy Left-Half-Plane (LHP) stability ($\operatorname{Re}(s) < 0$).
  2. **Frequency Pre-Warping**: Pre-warped angular frequencies $\omega_p = 2 F_s \tan\left(\frac{\pi f_c}{F_s}\right)$.
  3. **Frequency Transformation ($s$-domain)**:
     - **Lowpass / Highpass**: $N$ poles $\to \lceil N/2 \rceil$ biquad sections. Single real pole/zero for odd orders $N = 1, 3, 5$ is represented with trailing zero coefficients ($a_2 = 0, b_2 = 0$).
     - **Bandpass / Notch (Bandstop)**: $N$-th order prototype yields $2N$ transformed poles $\to N$ biquad SOS sections. `FilterSpec::notch(fs, lowcut, highcut, order)` specifies a 2-cutoff bandstop filter attenuating $[f_{\text{low}}, f_{\text{high}}]$.
  4. **Bilinear Transform ($s \to z$)**: Digital map $z = \frac{2 F_s + s}{2 F_s - s}$. All digital poles satisfy stability $|z_p| < 1.0$.
  5. **SOS Pairing & Section Ordering**: Nearest pole-zero pairing (`pairing='nearest'`) with sections sorted by pole distance to unit circle ($\min |1 - |z_p||$) to maximize dynamic range. Overall gain $k_z$ is placed on Section 0 numerator.

### 2.2 SciPy Parity & Empirical Verification Results
- **Coefficient Design Parity**: Lamina-generated SOS coefficient matrices match SciPy `scipy.signal.butter(..., output='sos')` matrices within Max Abs Error $L_\infty < 1.55 \times 10^{-15}$ across orders $N = 1 \dots 6$.
- **Filtering-Operation Parity**: Given identical SOS coefficients, `filtfilt` matches SciPy `sosfiltfilt` within Max Abs Error $L_\infty < 7.58 \times 10^{-13}$.
- **End-to-End Signal Parity (Primary Acceptance Criterion)**: `FilterSpec \to signal_filtfilt` matches SciPy `scipy.signal.butter \to scipy.signal.sosfiltfilt` across all 132 test configurations ($N = 1 \dots 6$, $F_s = 100\text{ Hz}$ & $500\text{ Hz}$, all filter kinds) with **Max Abs Error $L_\infty = 2.12 \times 10^{-12}$** and **Average RMS Error $= 6.28 \times 10^{-14}$** (validated with threshold $L_\infty < 1.0 \times 10^{-10}$).
- **Worst-Case Numerical Audit Configuration**: Configuration #118 (Bandpass Order 6, $F_s = 500\text{ Hz}$, cutoffs $[0.5, 40.0]\text{ Hz}$, `mixed` signal, sample index #563): Max Abs Error = $5.66 \times 10^{-13}$, RMS Error = $2.49 \times 10^{-13}$, Relative Error = $1.72 \times 10^{-10}$.
- **Frequency Response Invariants**: Tested independently without SciPy: $|H(f_c)| = 1/\sqrt{2} \approx 0.70710678$ ($-3.0103\text{ dB}$) at cutoff frequencies, $|H(0)| = 1.0$ at DC for Lowpass, and expected passband/stopband attenuation.



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
