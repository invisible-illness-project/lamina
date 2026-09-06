# Lamina Formal Scientific Data Contracts & Signal Semantics

**Author:** Staff-level Senior Scientific Software Engineer  
**Date:** September 6, 2026  
**Status:** APPROVED ARCHITECTURAL SPECIFICATION  
**Target Module:** `lamina::*`  
**Protobuf Alignment:** `sensor-messages.proto` (branch: `support-lamina-upgrade`)

---

## 1. Executive Principles

This document defines the formal scientific data contracts, physical unit conventions, signal polarity invariants, and interval quality representations across **Lamina**. These contracts govern Rust domain modules, JSON bridge interfaces, Python bindings (`lamina_bridge`), Dart FFI (`lamina_dart`), and protobuf wire schemas (`sensor-messages.proto`).

---

## 2. Signal Data Contracts

### 2.1 Sampling & Temporal Invariants
- **`sampling_rate` ($F_s$):** Expressed in Hertz ($\text{Hz} = \text{samples/second}$). Must be strictly finite and positive ($F_s > 0.0$).
- **Nyquist Floor Invariant:** For any filtering or processing function operating on cutoff frequency $f_{\text{cutoff}}$, $F_s$ must satisfy $F_s \ge 2 \cdot f_{\text{cutoff}}$. Functions return `SignalError::InvalidCutoffFrequency` when violated.
- **Physical Timestamps ($t$):** Expressed in seconds ($\text{s}$) relative to physical acquisition start:
  $$t_i = \text{offset\_sec} + \frac{i}{F_s}$$
- **Missing Data Representation:** Non-finite values (`NaN`, `+inf`, `-inf`) represent missing/invalid raw samples. Continuous DSP pipelines reject non-finite inputs with `SignalError::NonFiniteInput`. Segment-gated rPPG pipelines emit `f64::NAN` for invalid processing windows without silent zero-filling.

---

### 2.2 Electrocardiography (ECG) Contract (`lamina::ecg`)
- **Raw / Cleaned ECG Waveform ($x[n]$):** Expressed in millivolts ($\text{mV}$) or normalized physical units.
- **R-Peak Event ($R_k$):** 0-indexed sample index $i \in \mathbb{N}$ corresponding to local maximum voltage of the QRS complex, aligned to max amplitude within a $\pm 150\text{ ms}$ search window.
- **R-R Interval ($\text{RR}_k$):** Time duration in milliseconds ($\text{ms}$) between consecutive R-peaks:
  $$\text{RR}_k = \frac{R_k - R_{k-1}}{F_s} \times 1000.0 \quad (\text{ms})$$
- **N-N Interval ($\text{NN}_k$):** Cleaned Normal-to-Normal inter-beat interval in milliseconds ($\text{ms}$), derived by applying explicit interval quality classification and correction policies (`clean_rr_intervals`) to remove ectopic beats and motion artifacts.
- **Protobuf Type:** `CardiacPeakEvent` / `CardiacPeakBatch` (`SensorModality::SENSOR_MODALITY_ECG`).

---

### 2.3 Contact Photoplethysmography (PPG) Contract (`lamina::ppg`)
- **Raw / Cleaned PPG Waveform ($p[n]$):** Expressed in arbitrary relative units or blood volume optical transmission.
- **Systolic Peak Event ($S_k$):** 0-indexed sample index corresponding to local systolic maximum amplitude of the pulse wave, detected via Elgendi dual moving average thresholding.
- **Pulse Interval ($\text{PI}_k$):** Inter-pulse interval in milliseconds ($\text{ms}$) between consecutive systolic peaks:
  $$\text{PI}_k = \frac{S_k - S_{k-1}}{F_s} \times 1000.0 \quad (\text{ms})$$
- **Protobuf Type:** `CardiacPeakEvent` / `CardiacPeakBatch` (`SensorModality::SENSOR_MODALITY_PPG`).

---

### 2.4 Remote Photoplethysmography (rPPG) Contract (`lamina::rppg`)
- **Raw Camera Intensity (`RawIntensity`):** Spatial mean RGB intensity over skin Region of Interest (ROI). Decreases during cardiac systole as tissue blood volume increases.
- **Chrominance / Projection Signal (`RppgSignal`):** Normalized plane-orthogonal (`POS`) or chrominance-based (`CHROM`) optical surrogate waveform.
- **Blood Volume Pulse Waveform (`BvpWaveform`):** Normalized optical pulse surrogate expressed under Lamina's documented pulse-phase convention.
  - **Pulse-Phase Convention:** Positive amplitude deflection represents peak pulse wave expansion (peak systolic blood volume) to ensure downstream `lamina::ppg` Elgendi systolic peak detector consistency.
  - **Non-Clinical Boundary:** `BvpWaveform` represents a normalized optical surrogate; it does **not** claim direct measurement of absolute arterial blood volume ($\text{mL}$) or calibrated arterial pressure ($\text{mmHg}$).
- **Protobuf Type:** `RppgSignal` / `RppgSegment` (`SensorModality::SENSOR_MODALITY_RPPG`).

---

### 2.5 Electrodermal Activity (EDA) Contract (`lamina::eda`)
- **Raw / Cleaned Conductance ($g[n]$):** Expressed in microsiemens ($\mu\text{S}$).
- **Tonic Component ($\text{SCL}[n]$):** Skin Conductance Level extracted via zero-phase low-pass filtering ($<0.05\text{ Hz}$).
- **Phasic Component ($\text{SCR}[n]$):** Skin Conductance Response residual signal ($\text{cleaned}[n] - \text{tonic}[n]$).
- **Exact Reconstruction Invariant:**
  $$\text{tonic}[n] + \text{phasic}[n] = \text{cleaned}[n] \quad \pm 10^{-12}$$
- **SCR Event (`ScrEvent`):**
  - `onset_index`: Sample position of preceding local minimum.
  - `peak_index`: Sample position of maximum SCR phasic amplitude.
  - `amplitude_us`: Vertical height ($\text{phasic}[i_{\text{peak}}] - \text{phasic}[i_{\text{onset}}]$) in $\mu\text{S}$.
  - `rise_time_sec`: Time interval $(i_{\text{peak}} - i_{\text{onset}}) / F_s$ in seconds.
- **Passband Hygiene ($F_s = 4.0\text{ Hz}$):** At $4.0\text{ Hz}$ wearable EDA (Empatica E4), high-frequency low-pass filtering is dynamically bypassed or set to $1.8\text{ Hz}$ (`0.45 * sampling_rate`), preserving natural anti-aliasing while preventing Nyquist validation failures.
- **Protobuf Type:** `ScrEvent` / `ScrEventBatch` / `EdaFeatures` (`SensorModality::SENSOR_MODALITY_EDA`).

---

### 2.6 Respiration (RSP) Contract (`lamina::rsp`)
- **Cleaned Respiration Signal ($r[n]$):** Bandpass filtered signal ($0.05\text{--}0.50\text{ Hz}$, $3\text{--}30\text{ brpm}$).
- **Respiration Cycle (`RespirationCycle`):**
  - `inspiration_index`: Sample position of positive inspiratory peak ($i_k$).
  - `expiration_index`: Sample position of intervening expiratory trough ($e_k$) strictly satisfying $i_k < e_k < i_{k+1}$.
  - `duration_sec`: Cycle duration $(i_{k+1} - i_k) / F_s$ in seconds.
  - `respiratory_rate_bpm`: Instantaneous rate $60.0 / \text{duration\_sec}$.
  - `amplitude`: Peak-to-trough amplitude $r[i_{k+1}] - r[e_k]$.
- **Protobuf Type:** `RespirationCycle` / `RespirationCycleBatch` / `RespirationFeatures` (`SensorModality::SENSOR_MODALITY_RSP`).

---

### 2.7 Heart Rate Variability & Interval Architecture (`lamina::hrv`)
- **Decoupled Interval Quality & Correction Architecture:**
  ```text
  Beat Event (ECG / PPG)
            ↓
  BeatQuality (Normal, Ectopic, Artifact, Unknown)
            ↓
  IntervalQuality (NormalNN, EctopicRR, ArtifactRR, Missing)
            ↓
  CorrectionPolicy (None, RejectInvalid, InterpolateLinear, InterpolateCubic, PercentThreshold)
            ↓
  Clean N-N Series (clean_rr_intervals)
            ↓
  HRV Calculations (hrv_rmssd, hrv_mean_nn)
  ```
- **Operational Rule:** `hrv_rmssd` and `hrv_mean_nn` take clean **N-N intervals** ($\text{ms}$). Callers must pass intervals through `clean_rr_intervals` when processing recordings containing ectopic beats or motion artifacts.
- **Protobuf Type:** `CardiacFeatures` (`sdnn_ms`, `rmssd_ms`, `rr_mean_ms`, `rr_std_ms`).
