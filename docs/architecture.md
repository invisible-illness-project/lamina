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
|                       Multimodal Analysis Layer                         |
|  (lamina::multimodal: sync, phase, rsa, coupling, ecg_ppg, eda_assoc)   |
+-------------------------------------------------------------------------+
                                     |
                                     v
+-------------------------------------------------------------------------+
|                           Public Facade API                             |
|  (ecg_findpeaks, rsa, ecg_ppg_timing, cardiorespiratory_phase_coupling) |
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

## 4. Physiological Processing Layer (`lamina::ecg`, `lamina::ppg`, `lamina::eda`, `lamina::rsp`)

Lamina separates generic DSP primitives from domain-specific physiological interpretation.

### 4.1 ECG Pan-Tompkins QRS Detection Pipeline (`ecg_findpeaks_config`)
- **Canonical Reference**: Pan, J., & Tompkins, W. J. (1985). *A real-time QRS detection algorithm*. IEEE Transactions on Biomedical Engineering, BME-32(3), 230-236.
- **Pipeline Architecture**:
  $$\text{Raw ECG} \xrightarrow{\text{5--15Hz BP}} \text{Bandpassed} \xrightarrow{\text{5-Point Deriv}} \text{Derivative} \xrightarrow{\text{Square}} \text{Power} \xrightarrow{\text{150ms Moving Integral}} \text{Integrated} \xrightarrow{\text{Adaptive Dual-Threshold}} \text{R-Peaks}$$
- **Bandpass Filtering**: 2nd/3rd-order Butterworth SOS bandpass filter (default 5–15 Hz) via zero-phase `signal_filtfilt`.
- **5-Point Derivative**: $d[n] = \frac{F_s}{8} (-x[n-2] - 2x[n-1] + 2x[n+1] + x[n+2])$, highlighting QRS slope features while scaling linearly with sampling rate $F_s$.
- **Moving Window Integration**: $W = \text{round}(0.150 \cdot F_s)$ samples (150 ms window) using prefix-sum `signal_smooth_moving_average`.
- **Adaptive Dual Thresholding**: Signal peak ($SPKI$) and Noise peak ($NPKI$) level tracking with primary threshold $THRESHOLD_{I1} = NPKI + 0.25 (SPKI - NPKI)$ and searchback threshold $THRESHOLD_{I2} = 0.5 \cdot THRESHOLD_{I1}$.
- **Physiological Boundaries**: 200 ms ($\text{round}(0.200 \cdot F_s)$ samples) refractory period enforcement and searchback for missed beats when $RR > 1.66 \cdot RR_{\text{avg}}$.
- **R-Peak Fine Alignment**: Validated integrated candidate peaks are mapped back to exact maximum amplitude positions in `filtered_ecg` within a $\pm 150\text{ ms}$ search window.

### 4.2 PPG Elgendi Systolic Peak Detection Pipeline (`ppg_findpeaks_config`)
- **Canonical Reference**: Elgendi, M. et al. (2012). *Systolic Peak Detection in Acceleration Photoplethysmogram Signals Based on Dual Moving Averages*. PLOS ONE, 7(10), e47582.
- **Pipeline Architecture**:
  $$\text{Raw PPG} \xrightarrow{\text{0.5--8.0Hz BP}} \text{Bandpassed} \xrightarrow{\text{Clip \& Square}} \text{Enhanced} \xrightarrow{\text{Dual MAs}} (MA_{\text{peak}}, MA_{\text{beat}}) \xrightarrow{\text{Adaptive Block Thresh}} \text{Systolic Peaks}$$
- **Bandpass Filtering**: 3rd-order Butterworth SOS bandpass filter (default 0.5–8.0 Hz) via zero-phase `signal_filtfilt`.
- **Signal Enhancement**: Non-linear clipping & squaring ($S[n] = \max(0, x[n])^2$) emphasizing systolic pulse waves over diastolic ripples.
- **Dual Moving Averages**:
  - Short MA ($W_{\text{peak}} \approx 111\text{ ms}$, $\text{round}(0.111 \cdot F_s)$) representing systolic peak duration.
  - Long MA ($W_{\text{beat}} \approx 667\text{ ms}$, $\text{round}(0.667 \cdot F_s)$) representing cardiac beat duration.
- **Adaptive Block Thresholding**: $THRESHOLD = MA_{\text{beat}} + \alpha \cdot \bar{S}$, where $\alpha = 0.02$ and $\bar{S} = \text{mean}(S)$. Decision blocks are formed where $MA_{\text{peak}} > THRESHOLD$.
- **Decision Block Filtering**: Canonical Elgendi block size verification ($width \ge W_{\text{peak}}$) rejecting narrow noise spikes.
- **Systolic Peak Selection**: Local maximum within each valid decision block with a 300 ms ($\text{round}(0.300 \cdot F_s)$ samples) pulse wave refractory period.

### 4.3 Electrodermal Activity (EDA/GSR) Signal Processing (`eda_decompose` & `eda_findpeaks_events`)
- **Canonical References**: Dawson, M. E. et al. (2007). *The Electrodermal System*. Handbook of Psychophysiology.
- **Signal Model**:
  $$\text{EDA}(t) = \text{tonic}(t) + \text{phasic}(t)$$
- **Pipeline Architecture**:
  $$\text{Raw EDA} \xrightarrow{\text{5.0Hz Lowpass}} \text{Cleaned} \xrightarrow{\text{0.05Hz Lowpass}} \text{Tonic (SCL)} \xrightarrow{\text{Cleaned - Tonic}} \text{Phasic (SCR)} \xrightarrow{\text{Peak/Onset Analysis}} \text{ScrEvent List}$$
- **Tonic/Phasic Decomposition (`eda_decompose`)**:
  - **Tonic (Skin Conductance Level - SCL)**: Extracted via zero-phase 2nd-order 0.05 Hz low-pass Butterworth filtering (`signal_filtfilt`).
  - **Phasic (Skin Conductance Response - SCR)**: Residual signal $\text{phasic}[n] = \text{cleaned}[n] - \text{tonic}[n]$, providing **exact mathematical reconstruction** ($\text{tonic}[n] + \text{phasic}[n] = \text{cleaned}[n]$ within floating-point epsilon).
- **SCR Event Characterization (`ScrEvent`)**:
  - `onset_index`: Sample position of preceding trough/local minimum.
  - `peak_index`: Sample position of maximum SCR amplitude.
  - `amplitude`: Vertical height $\text{phasic}[i_{\text{peak}}] - \text{phasic}[i_{\text{onset}}]$ in microsiemens ($\mu\text{S}$).
  - `rise_time_sec`: Time interval $(i_{\text{peak}} - i_{\text{onset}}) / F_s$ in seconds.

### 4.4 Respiration (RSP) Signal Processing (`rsp_clean`, `rsp_cycles`, `rsp_rate`)
- **Canonical References**: Khodadad, D. et al. (2018). *Optimized peak detection in respiration signals*. Physiological Measurement. NeuroKit2 RSP Processing Guidelines.
- **Signal Model**:
  $$\text{RSP}(t) = \text{respiratory\_component}(t) + \text{baseline\_drift}(t) + \text{noise}(t)$$
- **Pipeline Architecture**:
  $$\text{Raw RSP} \xrightarrow{\text{0.05--0.50Hz BP}} \text{Cleaned RSP} \xrightarrow{\text{Extrema Detection}} (i_k, e_k) \xrightarrow{\text{Cycle Pairing}} \text{RespirationCycle List} \xrightarrow{\text{Rate Calculation}} \text{RSP Rate Array}$$
- **Respiration Cleaning (`rsp_clean_config`)**:
  - 3rd-order Butterworth SOS bandpass filter (default 0.05–0.50 Hz, corresponding to 3–30 breaths/min) via zero-phase `signal_filtfilt`.
  - Removes baseline drift ($<0.05\text{ Hz}$) and high-frequency motion/muscular noise ($>0.50\text{ Hz}$).
- **Peak / Trough Detection & Cycle Pairing (`rsp_cycles_config`)**:
  - Inspiratory extrema ($i_k$) detected as positive peaks on `cleaned_rsp` using generic peak detection (`signal_findpeaks_config`) with minimum distance $W_{\text{min\_dist}} = \text{round}(\text{min\_breath\_interval} \cdot F_s)$.
  - Expiratory extrema ($e_k$) detected as troughs by applying peak detection to inverted signal $-1.0 \times \text{cleaned\_rsp}$.
  - Sequential cycle pairing constructs `RespirationCycle`:
    - Finds expiratory trough $e_k$ strictly between consecutive inspiratory peaks $i_k < e_k < i_{k+1}$.
    - Calculates breath duration $T_k = (i_{k+1} - i_k) / F_s$ (sec), instantaneous rate $\text{BPM}_k = 60.0 / T_k$, and peak-to-trough amplitude $A_k = x[i_{k+1}] - x[e_k]$.
- **Physiological Validation & Safeguards**:
  - Noise floor gate: checks peak-to-peak amplitude $A_{\text{p2p}} \ge 10^{-12}$; flat/constant signals return empty cycle lists.
  - Duration bounds: validates $1.50\text{ s} \le T_k \le 15.0\text{ s}$ (4–40 bpm).
  - Minimum amplitude threshold: enforces $A_k \ge \text{min\_amplitude}$.
- **Respiratory Rate Array Extraction (`rsp_rate_config`)**:
  - Calculates instantaneous rate array matching input signal length $N$.
  - Piecewise constant interpolation between cycle boundaries with nearest-neighbor extrapolation at boundaries.
  - Handles invalid/empty cycles safely by returning 0.0 array.
- **Respiratory Rate Variability (RRV)**:
  - Extracted directly from cycle interval series $I_k = (i_{k+1} - i_k) / F_s$, enabling mean rate, mean breath interval, SDNN/SDRR, and RMSSD computation.

### 4.5 Offline (Non-Causal Zero-Phase) vs. Real-Time (Causal Streaming) Architectural Boundaries

Lamina explicitly distinguishes between offline retrospective signal processing and streaming real-time execution:

- **Offline Processing (`signal_filtfilt`, `ecg_findpeaks`, `ppg_findpeaks`, `eda_decompose`, `rsp_cycles`)**:
  - Employs zero-phase forward-backward filtering (`signal_filtfilt`) to eliminate phase distortion and group delay.
  - Non-causal: requires the complete signal array to perform end reflection padding ($3 \times \text{order}$).
  - Retrospectively estimates global signal/noise levels ($SPKI$, $NPKI$, $\bar{S}$, peak-to-peak amplitude) across the full waveform.
- **Streaming Real-Time Processing (Architectural Model for Future Modules)**:
  - Requires single-pass causal IIR filtering (stateful `SosFilter` instances) with fixed group delay.
  - Bounded latency $\Delta t \le \text{window\_size}$.
  - Incremental running estimate updates ($SPKI$, $NPKI$) updated beat-by-beat without retrospective searchback.

### 4.6 Scientific Validation & Clinical Disclaimer

- **Validation Methodology**: Tested against NeuroKit2 reference implementations (`nk.ecg_peaks`, `nk.ppg_peaks`, `nk.eda_peaks`, `nk.rsp_process`) across multiple sampling frequencies ($32, 50, 64, 100, 128, 250, 500, 1000\text{ Hz}$).
- **Event Parity Metrics**: Evaluated with standard time-domain tolerances ($\Delta t \le 150\text{ ms}$ for ECG/PPG, $\Delta t \le 250\text{ ms}$ for EDA/RSP).
- **Disclaimer**: Lamina is a general-purpose scientific signal-processing library designed for research and physiological data analysis. **It is not a medical device, nor has it been cleared by regulatory authorities (FDA, CE) for clinical diagnosis or monitoring.**

---

## 5. Multimodal Physiological Analysis Layer (`lamina::multimodal`)

Lamina provides a cross-modality synthesis layer that operates directly on physical time coordinates, derived events, and phase representations without duplicating low-level DSP filtering or peak detection.

### 5.1 Physical Synchronization Model (`sync`)
- **Independent Sampling Rates & Offsets**: Operates seamlessly across heterogeneous sampling frequencies ($F_s$) and hardware acquisition start offsets ($\text{offset\_sec}$).
- **Timestamp Transformation**: Maps 0-indexed sample $n$ to physical time $t$:
  $$t = \text{offset\_sec} + \frac{n}{F_s}$$
- **Event Representation (`TimedEvent`)**: Pairs discrete modality events (`Ecg`, `Ppg`, `Eda`, `Rsp`) with sample indices and physical timestamps.

### 5.2 Respiratory Phase Mapping (`phase`)
- **Continuous Phase Convention ($\phi \in [0, 2\pi)$)**:
  - Inspiration phase $[0, \pi)$: maps timestamp $t \in [t_{\text{insp1}}, t_{\text{exp}}]$ linearly from $0$ to $\pi$.
  - Expiration phase $[\pi, 2\pi)$: maps timestamp $t \in (t_{\text{exp}}, t_{\text{insp2}}]$ linearly from $\pi$ to $2\pi$.
- **Boundary Safety**: Timestamps outside complete respiratory cycles return `None` without panics or silent invalid values.

### 5.3 Respiratory Sinus Arrhythmia (RSA) (`rsa`)
- **Operational Definition**: Evaluates peak within-cycle heart rate modulation across valid respiration cycles:
  $$\Delta \text{BPM}_k = \max_{i \in \text{insp}} \text{BPM}_i - \min_{e \in \text{exp}} \text{BPM}_e$$
  $$\Delta \text{RR}_k = \max_{e \in \text{exp}} \text{RR}_e - \min_{i \in \text{insp}} \text{RR}_i$$
- **Beat-Level Mapping (`CardiacRespiratoryEvent`)**: Binds each ECG R-peak to continuous respiratory phase, instantaneous R-R interval ($\text{seconds}$), and instantaneous heart rate ($\text{BPM}$).

### 5.4 Cardiorespiratory Phase Coupling (`coupling`)
- **Circular Statistics**:
  $$\bar{C} = \frac{1}{N} \sum_{k=1}^N \cos(\phi_k), \quad \bar{S} = \frac{1}{N} \sum_{k=1}^N \sin(\phi_k)$$
  $$R = \sqrt{\bar{C}^2 + \bar{S}^2}, \quad \bar{\phi} = \operatorname{atan2}(\bar{S}, \bar{C}) \pmod{2\pi}$$
- **Invariants**: Resultant vector length $R \in [0.0, 1.0]$ measures phase concentration ($R \approx 1.0$ for concentrated phases, $R \approx 0.0$ for uniform distributions) and is invariant under constant rotational shifts.

### 5.5 ECG-to-PPG Pulse Delay Timing (`ecg_ppg`)
- **Deterministic 1-to-1 Matching**: Pairs each ECG R-peak with at most one following PPG systolic pulse wave within a configurable time window $[t_{\text{min}}, t_{\text{max}}]$ (default $0.10\text{--}0.60\text{ s}$).
- **Terminology Precision**: Measured delay represents observable ECG-PPG peak delay and is explicitly distinguished from calibrated arterial pulse transit time (PTT).

### 5.6 EDA Temporal Associations (`eda_assoc`)
- **Event Linking**: Associates `ScrEvent` peaks with preceding/nearest ECG R-peak timestamps and current respiratory phase without asserting causal physiological mechanisms.

### 5.7 Multimodal Signal Quality (`quality`)
- **Transparent Rule-Based Scoring**: Evaluates modality validity and issues (`EmptySignal`, `UnplausibleHeartRate`, `UnplausibleRespirationRate`, `ExtremeArtifact`) yielding normalized quality scores in $[0.0, 1.0]$.

---

## 6. Dependency Decision Table

| Capability | Current Lamina | Candidate | Decision | Reason |
| :--- | :--- | :--- | :--- | :--- |
| **IIR Filter Design** | Manual FFT formula | `biquad` | **Adopt (Primitive)** | Provides stable Biquad/SOS coefficients & sample state logic; `#![no_std]` ready. |
| **Zero-Phase Filtering** | Unpadded FFT bin scaling | Custom + `biquad` | **Custom (Lamina)** | `filtfilt` forward-backward reflection & boundary padding is domain logic owned by Lamina. |
| **Peak Detection** | 3-point local maxima | `find_peaks` | **Adopt (Primitive)** | High quality prominence, distance, & height handling matching SciPy semantics (`v0.1.5`). |
| **Smoothing** | $O(N \cdot W)$ moving average | Custom | **Custom (Lamina)** | Optimized $O(N)$ sliding window accumulator natively implemented in `src/signal/smooth.rs`. |
| **Parallelism** | None (Single-threaded) | `rayon` | **Optional Feature** | Guard with `#[cfg(feature = "parallel")]` for heavy non-linear metrics like Sample Entropy. |
| **ECG / PPG / EDA** | Prototype heuristics | Custom | **Custom (Lamina)** | Physiological algorithms (Pan-Tompkins, Elgendi, Lowpass/Highpass SCR) must be owned by Lamina. |
| **HRV Analysis** | Basic time-domain | `cardio-rs` / `hrv-algos` | **Reject Dependency** | Native Lamina implementation guarantees clean API contracts, zero extraneous dependencies, and `no_std` flexibility. |

---

## 7. Error Semantics

Lamina uses a central `SignalError` type:
- `EmptySignal`
- `InvalidSamplingRate(f64)`
- `InvalidCutoffFrequency(String)`
- `InsufficientSamples { required: usize, provided: usize }`
- `InvalidWindowSize(usize)`
- `InvalidFilterOrder(usize)`
- `NonFiniteInput`
- `InsufficientPeaks { required: usize, provided: usize }`
