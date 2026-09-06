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

### 1.2 Design Principles

1. **Facade Encapsulation**: Public APIs accept native Rust types (`ndarray::Array1<f64>` or slices) and basic scalar parameters. Internal third-party filter states, peak builders, or planner objects are strictly encapsulated.
2. **Standard Data Structure**: `ndarray::Array1<f64>` remains the baseline vector representation across the library for zero-copy slicing, vector math, and linear algebra interoperability.
3. **No Hidden Global State**: All processing algorithms are pure, stateless functions (or explicit pipeline structs where real-time streaming state is needed in the future).
4. **Predictable Fallibility**: Functions validate input parameters (sampling rates, signal lengths, cutoff frequencies) and return `Result<T, SignalError>` rather than panicking or producing silent NaN values.

---

## 2. Dependency Audit & Decision Matrix

### 2.1 Candidate Evaluation Summaries

#### A. `biquad` (v0.4.2) — Digital Filtering Engine
* **Supported Filter Types**: Lowpass, Highpass, Bandpass, Notch, Peak, Allpass, Low/High Shelf.
* **Butterworth & SOS Support**: Direct Form 1 (DF1) and Direct Form 2 Transposed (DF2T) Second-Order Sections (SOS).
* **Numerical Stability**: High. Second-order sections prevent coefficient quantization issues present in high-order transfer functions ($b, a$).
* **State & Boundary Semantics**: Sample-by-sample state update (`filter.run(sample)`).
* **SciPy `filtfilt` Parity**: `biquad` provides the single-pass difference filter. It does **not** natively provide zero-phase forward-backward filtering or boundary reflection (`padtype='odd'`). Lamina must implement `filtfilt` state handling and boundary extension on top of `biquad`.
* **License / Suitability**: MIT / Apache-2.0. `#![no_std]` compatible. Highly recommended as lower-level DSP primitive.

#### B. `find_peaks` (v0.1.5) — Generic Peak Finder
* **Capabilities**: Supports `min_height`, `min_distance`, `min_prominence`, `min_width`, and valley detection.
* **Semantics**: Matches `scipy.signal.find_peaks` prominence algorithms and returns peak index vectors.
* **License / Suitability**: MIT. Clean `#![no_std]` support with `alloc`. Recommended as internal peak-finding primitive for basic peak detection.

#### C. `rayon` (v1.10.0) — Parallelism
* **Evaluation**: Useful for multi-threaded batch operations (e.g. matrix-like pairwise embedding comparisons in Sample Entropy).
* **Decision**: Make optional via a `parallel` feature flag. Do not mandate `rayon` for low-latency or embedded standard builds.

#### D. `cardio-rs` & `hrv-algos` — External Physiological Crates
* **Evaluation**: `cardio-rs` and `hrv-algos` provide standalone HRV algorithms. However, importing them as dependencies would introduce external domain types, conflicting design patterns, and duplicate dependency graphs.
* **Decision**: Reject as external dependencies. Keep HRV calculations native in Lamina to maintain API consistency, minimal dependencies, and control over numerical contracts.

### 2.2 Dependency Decision Table

| Capability | Current Lamina | Candidate | Decision | Reason |
| :--- | :--- | :--- | :--- | :--- |
| **IIR Filter Design** | Manual FFT formula | `biquad` | **Adopt (Primitive)** | Provides stable Biquad/SOS coefficients & sample state logic; `#![no_std]` ready. |
| **Zero-Phase Filtering** | Unpadded FFT bin scaling | Custom + `biquad` | **Custom (Lamina)** | `filtfilt` forward-backward reflection & boundary padding is domain logic owned by Lamina. |
| **Peak Detection** | 3-point local maxima | `find_peaks` | **Adopt (Primitive)** | High quality prominence, distance, & height handling matching SciPy semantics. |
| **Smoothing** | $O(N \cdot W)$ moving average | Custom | **Custom (Lamina)** | Optimized $O(N)$ sliding window accumulator natively implemented in `src/signal/smooth.rs`. |
| **Parallelism** | None (Single-threaded) | `rayon` | **Optional Feature** | Guard with `#[cfg(feature = "parallel")]` for heavy non-linear metrics like Sample Entropy. |
| **ECG / PPG / EDA** | Prototype heuristics | Custom | **Custom (Lamina)** | Physiological algorithms (Pan-Tompkins, Elgendi, cvxEDA/Highpass, SCR) must be owned by Lamina. |
| **HRV Analysis** | Basic time-domain | `cardio-rs` / `hrv-algos` | **Reject Dependency** | Native Lamina implementation guarantees clean API contracts, zero extraneous dependencies, and `no_std` flexibility. |

---

## 3. Error Semantics & Input Validation Strategy

Lamina introduces a dedicated error type: `SignalError`.

### 3.1 Error Variants

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum SignalError {
    /// Signal length is empty (0 samples).
    EmptySignal,
    /// Sampling rate is invalid (<= 0.0 or non-finite).
    InvalidSamplingRate(f64),
    /// Cutoff frequency is invalid (<= 0.0, >= Nyquist, or highcut <= lowcut).
    InvalidCutoffFrequency(String),
    /// Signal length is shorter than the minimum required samples for the algorithm.
    InsufficientSamples { required: usize, provided: usize },
    /// Moving window size is invalid (0 or even where odd is required).
    InvalidWindowSize(usize),
    /// Filter order is invalid.
    InvalidFilterOrder(usize),
    /// Signal contains NaN or Infinity values.
    NonFiniteInput,
    /// Insufficient detected peaks or intervals for metric calculation.
    InsufficientPeaks { required: usize, provided: usize },
}
```

### 3.2 Fallibility Policy

* **Public APIs**: Functions that accept user-provided signals, sample rates, window sizes, or cutoff frequencies return `Result<T, SignalError>`.
* **No Panics**: Functions do not panic on invalid inputs (e.g. $F_s \le 0$, empty slice, NaN sample).
* **Propagating Errors**: Internal pipelines propagate error states back to the caller using the `?` operator.

---

## 4. Scientific API Contracts

Every public function in Lamina adheres to strict semantic conventions documented in Rustdoc comments:

### 4.1 Units and Conventions
* **Signal Representation**: `Array1<f64>` (or `&Array1<f64>`).
* **Sampling Rate ($F_s$)**: Expressed in **Hertz (Hz)**. Must be $F_s > 0.0$.
* **Time Units**:
  * Raw time values: **seconds (s)**.
  * Inter-beat / R-R / peak-to-peak intervals: **milliseconds (ms)**.
* **Frequency Units**: Expressed in **Hertz (Hz)**. Must satisfy $0 < f_{\text{low}} < f_{\text{high}} < \frac{F_s}{2}$ (Nyquist limit).
* **Peak Indices**: Returned as `Vec<usize>` or boolean masks (`Array1<bool>`), 0-indexed relative to the input array.

---

## 5. Testing & Reference Parity Strategy

### 5.1 Edge-Case Test Suite
All module algorithms are tested against fundamental edge cases:
- Empty array (`N=0`)
- Constant signal (`[c, c, c, ...]`)
- Single sample (`N=1`)
- Impulse signal (`[0, ..., 1.0, ..., 0]`)
- Non-finite numbers (`NaN`, `Inf`, `-Inf`)
- Invalid sample rates ($F_s = 0.0$, $F_s = -100.0$)

### 5.2 Reference Parity Hierarchy
When validating against Python reference implementations (SciPy / NeuroKit2):

1. **Category 1: Exact Numerical Equivalence**
   - Tolerances: $|x_{\text{rust}} - x_{\text{py}}| < 10^{-6}$.
   - Applied to: HRV time-domain metrics (RMSSD, MeanNN, SDNN), moving average smoothers, standard deviations.
2. **Category 2: Approximate Numerical Equivalence**
   - Tolerances: $|x_{\text{rust}} - x_{\text{py}}| < 10^{-3}$.
   - Applied to: Digital IIR filtering (`filtfilt` outputs), continuous signal envelopes.
3. **Category 3: Event Detection Parity**
   - Tolerances: Detected peak indices match within $\pm \tau$ samples (e.g. $\pm 15$ samples at 100Hz = $\pm 150\text{ ms}$).
   - Applied to: ECG R-peaks, PPG systolic peaks, EDA SCR peaks, RSP breath peaks.
4. **Category 4: Deliberately Different Implementation Semantics**
   - Applied when Lamina explicitly chooses a superior or streaming-friendly algorithm variation. Must be documented in the corresponding module header.

---

## 6. Performance Benchmarking Baseline

Lamina uses `criterion` to establish a performance baseline for core algorithms:

- `moving_average`: Benchmarked across signal sizes ($N = 1,000, 10,000, 100,000$) and window sizes ($W = 5, 51, 151$).
- `peaks`: Benchmarked for 3-point maxima and prominence searches.
- `filtering`: Benchmarked for zero-phase Butterworth filtering.
- `sample_entropy`: Benchmarked for complexity evaluation across $N = 500, 2000$.

---

## 7. Recommended Task Roadmap for Future Agents

Following this foundational release, subsequent agents should implement features in the following dependency order:

1. **Task 1: IIR Filter Engine & SciPy `filtfilt` Parity**
   - Implement `biquad` SOS coefficient generation.
   - Implement forward-backward filtering (`signal_filtfilt`) with odd boundary extension (`padtype='odd'`).
   - Validate against SciPy `scipy.signal.filtfilt` parity test suite.

2. **Task 2: Peak Detection Upgrade (`find_peaks` Primitive Integration)**
   - Integrate `find_peaks` crate to back `signal_findpeaks`.
   - Support `distance`, `prominence`, `height`, and `threshold` parameters in `signal_findpeaks`.

3. **Task 3: Pan-Tompkins ECG & Elgendi PPG Algorithm Upgrade**
   - Upgrade `ecg_findpeaks` to full Pan-Tompkins algorithm (5-15Hz SOS bandpass, derivative, squaring, moving window, dynamic dual thresholding, refractory period).
   - Upgrade `ppg_findpeaks` to Elgendi dual moving average algorithm.
   - Validate against `golden_ecg.json` reference datasets.

4. **Task 4: Phasic EDA Decomposition & Respiratory Rate Gating**
   - Implement cvxEDA / Highpass 0.05Hz EDA Phasic decomposition with SCR amplitude ($\ge 0.01\mu S$) and rise-time constraints.
   - Implement RSP breath peak detection with physiological rate bounds ($0.1 - 0.5\text{ Hz}$).

5. **Task 5: Extended HRV Metrics Suite & Parallel Sample Entropy**
   - Expand `lamina::hrv` to support `SDNN`, `SDSD`, `pNN50`, `pNN20`, and frequency-domain metrics.
   - Add optional `rayon` feature for parallelized `sample_entropy`.
