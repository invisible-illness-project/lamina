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
|   |                   |  |    lamina::rppg   |  |                    |  |
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

### 4.5 Remote Photoplethysmography (rPPG) Signal Substrate (`lamina::rppg`)
- **Canonical References**: Verkruysse et al. (2008); de Haan & Jeanne (2013, CHROM); Wang et al. (2017, POS); Elgendi et al. (2026 rPPG Roadmap).
- **Pipeline Architecture**:
  $$\text{VideoStream} \xrightarrow{\text{ROI Provider}} \text{OpticalSignal} \xrightarrow{\text{Timestamp Slicing}} \text{Window-Local Preprocess} \xrightarrow{\text{CHROM / POS}} \text{Quality & Overlap-Add} \xrightarrow{\text{valid\_segments}} \text{ppg\_clean / ppg\_findpeaks}$$
- **Physical Timestamp Windowing**: Slices optical signals into physical time windows using $O(\log N)$ binary search (`timestamp_range`), enforcing window-local linear detrending and channel mean normalization before optical pulse projections.
- **First-Class Quality & Gap Invalidation**: Evaluates segment quality across ROI sufficiency, motion displacement, illumination stability, and spectral periodicity (using discrete lag bounds $\lceil F_s / f_{\text{high}} \rceil$ to $\lfloor F_s / f_{\text{low}} \rfloor$). Invalid processing windows (internal gaps $> \text{max\_gap\_sec}$ or low coverage) emit non-finite missing data samples (`f64::NAN`).
- **Non-Overlapping Piecewise Elementary Quality Integration**: Aggregates segment quality across overlapping window hops by integrating mean quality over non-overlapping elementary intervals, preventing window multiplicity bias.
- **Downstream PPG Integration**: `RppgSignal` provides `.valid_segments(max_gap_sec)` to extract contiguous non-NaN `RppgSegment` slices paired with `.to_ndarray()` and `.resample_uniform(target_fs, max_gap_sec)` for direct input to `lamina::ppg` (`ppg_clean`, `ppg_findpeaks`). See [`docs/rppg.md`](file:///home/eddiem3/development/roeh-health/lamina/docs/rppg.md).

### 4.6 Offline (Non-Causal Zero-Phase) vs. Real-Time (Causal Streaming) Architectural Boundaries

Lamina explicitly distinguishes between offline retrospective signal processing and streaming real-time execution:

- **Offline Processing (`signal_filtfilt`, `ecg_findpeaks`, `ppg_findpeaks`, `eda_decompose`, `rsp_cycles`)**:
  - Employs zero-phase forward-backward filtering (`signal_filtfilt`) to eliminate phase distortion and group delay.
  - Non-causal: requires the complete signal array to perform end reflection padding ($3 \times \text{order}$).
  - Retrospectively estimates global signal/noise levels ($SPKI$, $NPKI$, $\bar{S}$, peak-to-peak amplitude) across the full waveform.
- **Streaming Real-Time Processing (Architectural Model for Future Modules)**:
  - Requires single-pass causal IIR filtering (stateful `SosFilter` instances) with fixed group delay.
  - Bounded latency $\Delta t \le \text{window\_size}$.
  - Incremental running estimate updates ($SPKI$, $NPKI$) updated beat-by-beat without retrospective searchback.

### 4.7 Scientific Validation & Clinical Disclaimer

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

## 6. Windowed Multimodal Physiological Feature Extraction Layer (`lamina::features`)

Lamina provides a configurable windowed feature extraction engine that transforms continuous waveforms and event-level outputs into timestamped physiological feature vectors (`MultimodalFeatureVector`) across sliding time windows.

### 6.1 Window Generation & Time Boundaries (`window`)
- **Half-Open Boundaries $[t_{\text{start}}, t_{\text{end}})$**: Sliding feature windows (`FeatureWindow`) are generated using exact physical timestamps (seconds).
- **Physical to Sample Index Mapping (`time_range_to_sample_range`)**: Maps physical interval $[t_{\text{start}}, t_{\text{end}})$ to discrete sample indices $[i_{\text{start}}, i_{\text{end}})$ under $t_i = \text{offset\_sec} + \frac{i}{F_s}$ with strict half-open inclusion/exclusion without floating-point epsilons.
- **Monotonic Range Lookup (`EventCursor`)**: Monotonic two-pointer index advancement resolves window event bounds in $O(N + W)$ total time across the recording, eliminating per-window $O(N)$ full-recording scans.

### 6.2 Modality Feature Extraction & Boundary Conventions
- **Cardiac Features (`cardiac`)**:
  - **Pre-Resolved Window Bounds**: `cardiac_features_range()` accepts pre-resolved `(start_idx, end_idx)` bounds into the R-peak series, eliminating per-window scans over the entire recording.
  - **Terminating R-Peak Boundary Convention**: An RR interval $(R_{k-1}, R_k)$ belongs to window $[t_{\text{start}}, t_{\text{end}})$ if and only if terminating peak $t(R_k) \in [t_{\text{start}}, t_{\text{end}})$. When $start\_idx > 0$, peak $R_{start\_idx - 1}$ is retained so the left-boundary-crossing RR interval remains available without orphan intervals or double-counting.
  - **Explicit HRV Statistics**: SDNN is explicitly calculated as the population standard deviation ($\sqrt{\frac{1}{M} \sum (RR_m - \overline{RR})^2}$); `rr_std_ms` is documented as an alias to `sdnn_ms`.
- **EDA Features (`eda`)**: Mean/median/std Tonic SCL ($\mu\text{S}$), mean/std Phasic SCR ($\mu\text{S}$), SCR event count, normalized SCR rate ($\text{events/min}$), and mean/median SCR amplitude ($\mu\text{S}$) & rise time ($\text{seconds}$). Enforces $N_{\text{tonic}} == N_{\text{phasic}}$ signal dimension equality (`SignalError::DimensionMismatch`).
- **Respiration Features (`respiration`)**: Mean/median/std respiratory rate ($\text{BPM}$), mean cycle duration ($\text{seconds}$), cycle count, and amplitude statistics ($\text{peak-to-trough}$).
- **Multimodal Coupling Features (`coupling`)**: Precomputes recording-level observation streams (`pulse_delays`, `cr_phases`, `eda_assocs`) once (`PrecomputedCoupling`) and aggregates them per window via pre-resolved range bounds `(r_range, c_range, delay_range, phase_range, assoc_range)`. RSA amplitude modulation ($\Delta \text{BPM}$, $\Delta \text{RR}_{\text{sec}}$) is evaluated as a window-local coupling metric over window-bounded beats and cycles. Also provides cardiorespiratory phase concentration ($R \in [0.0, 1.0]$), mean phase ($\bar{\phi}$), mean/std ECG-PPG pulse delay ($\text{seconds}$), and SCR cardiorespiratory association counts.

### 6.3 Event Ordering Invariants & Fallible Validation
- **Chronological Ordering Contract**: All event timestamp and sample index series (`ecg_r_peaks`, `ppg_peaks`, `eda_scr_events`, `rsp_cycles`) must be sorted in non-decreasing chronological order.
- **Validation**: Enforced at `MultimodalInput` construction / boundary validation points via `validate_sorted_slice`, returning `Err(SignalError::UnsortedEvents)` if unsorted. Performed once per recording ($O(N)$), avoiding repeated per-window checks.

### 6.4 Algorithmic Complexity Breakdown
Complexity is distinguished by layer (range lookup vs. per-window aggregation):

| Operation | Complexity | Description |
| :--- | :---: | :--- |
| Window generation | $O(W)$ | Computes $W$ feature windows from time bounds |
| Monotonic event range lookup | $O(N + W)$ total | Monotonic `EventCursor` traversal across $W$ windows |
| Binary-search range lookup | $O(\log N)$ per window | Fallback binary search lookup for standalone single-window helper calls |
| Count / rate statistics | $O(1)$ per window | Window-bounded element count ($e - s$) and rate calculations, given pre-resolved range bounds $(s, e)$ |
| Cardiac HRV statistics | $O(K)$ per window | Iterates over $K$ window-bounded beats/intervals |
| Median / order statistics | $O(K \log K)$ per window | Sorting $K$ window observations for median estimation |
| Coupling feature aggregation | $O(K_c)$ per window | Aggregating $K_c$ pre-matched coupling observations in window (RSA computed over window beats/cycles) |

### 6.5 Feature Quality Assessment & Coverage Semantics (`quality`)
- **Temporal Availability Coverage**: Evaluates modality coverage strictly as the fraction of temporal overlap between window $[t_{\text{start}}, t_{\text{end}})$ and modality recording bounds:
  $$\text{coverage}_{\text{modality}} = \frac{\text{overlap}(\text{window}, \text{modality bounds})}{\text{window duration}}$$
  *Clarification*: This measures temporal data availability, **not** signal quality or artifact-free physiological quality.
- **Separate Quality Checks**: Physiological event sufficiency is assessed separately via configurable thresholds (`min_beats`, `min_respiration_cycles`, `min_scr_events`).
- **Transparent Quality Summary (`FeatureQuality`)**: Tracks temporal coverage, modality validity flags (`cardiac_valid`, `eda_valid`, `respiration_valid`, `coupling_valid`), usable feature count vs schema total ($30$), and specific issue codes (`InsufficientBeats`, `InsufficientRespirationCycles`, `InsufficientScrEvents`, `LowCoverage`).

---

## 7. Dependency Decision Table

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

## 8. Error Semantics

Lamina uses a central `SignalError` type:
- `EmptySignal`
- `InvalidSamplingRate(f64)`
- `InvalidCutoffFrequency(String)`
- `InsufficientSamples { required: usize, provided: usize }`
- `InvalidWindowSize(usize)`
- `InvalidFilterOrder(usize)`
- `NonFiniteInput`
- `InsufficientPeaks { required: usize, provided: usize }`
