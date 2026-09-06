# Lamina Architectural Remediation Plan & Engineering Strategy (Final Approved)

**Author:** Staff-level Senior Scientific Software Engineer  
**Date:** September 6, 2026  
**Status:** APPROVED FOR EXECUTION  
**Target Repository:** `invisible-illness-project/lamina`  
**Protobuf Repository:** `~/development/roeh-health/sensor_messages` (branch: `support-lamina-upgrade`)  
**Base Commit:** `main` at `7538b6a`

---

## 1. Absolute Engineering Principle

> **Lamina should optimize for scientifically correct signal semantics and reproducible behavior across acquisition conditions — not for reproducing a desired benchmark number.**

---

## 2. Executive Summary & Approved Strategy

This document defines the final, Staff-level engineering remediation plan for **Lamina v0.1.0**. Following comprehensive empirical validation across 18 public physiological datasets and multi-stage review, this plan establishes a 8-phase execution structure that prioritizes scientific data contracts, explicit sampling-rate invariants, decoupled interval quality architectures, and cross-language protobuf alignment.

### Approved Core Directives

1. **Scientific Data Contracts (`docs/architecture/signal-contracts.md`):** Establish formal, documented scientific data contracts for Sampling, ECG, PPG, rPPG, and EDA signals before executing domain changes.
2. **Mandatory Root-Cause Gate:** No algorithmic change may be made solely to improve benchmark metrics. Algorithmic modifications require mechanism reproduction, a minimized regression fixture, independent signal-processing justification, and per-record regression testing.
3. **Protobuf & Cross-Language Alignment:** All new signal types, configurations, and polarity flags must be evaluated against `~/development/roeh-health/sensor_messages` (`support-lamina-upgrade`), `lamina_dart`, and Python bindings before finalizing public Rust signatures.
4. **Experimental EDA Passband Evaluation:** At $4.0\text{ Hz}$ EDA (Empatica E4), do not apply a low-pass filter whose cutoff violates Nyquist ($2.0\text{ Hz}$). Experimentally evaluate pass-through, $1.0\text{ Hz}$, $1.5\text{ Hz}$, and $1.8\text{ Hz}$ configurations on SCR morphology, tonic/phasic decomposition, and event timing rather than enforcing a mechanical formula.
5. **rPPG Pulse-Phase Contract:** Define `BvpWaveform` as representing a normalized optical pulse surrogate under Lamina's documented pulse-phase convention to ensure downstream Elgendi peak detector consistency. It does **not** claim direct measurement of absolute arterial blood volume.
6. **Decoupled HRV & NN Architecture:** Decouple beat quality, interval classification, and correction policies (`Beat` $\to$ `BeatQuality` $\to$ `IntervalQuality` $\to$ `CorrectionPolicy` $\to$ `NN` $\to$ `HRV`). Expose explicit user-configurable correction policies (`None`, `Reject`, `Replace`, `Interpolate`, `PercentThreshold`, `Custom`) rather than embedding a fixed $20\%$ filter inside HRV functions.
7. **Per-Record Regression Gates:** Enforce per-record regression tracking alongside suite-level aggregate thresholds to prevent degrading difficult recordings while tuning overall means.

---

## 3. Finding-by-Finding Remediation Matrix

| ID | Title | Status | Severity | Execution Phase | Approved Engineering Action |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **BUG-001** | `ecg_clean` ignores `_method` | `confirmed` | Low | Phase 1 | Implement method string dispatch for genuinely supported pipelines (`"neurokit"`, `"pantompkins"`); return `SignalError::InvalidCutoffFrequency` for unsupported methods. |
| **BUG-002** | `threshold_multiplier >= 1.0` returns wrong error | `confirmed` | Medium | Phase 1 | Return a clear parameter range validation error for out-of-bounds `threshold_multiplier` range $(0.0, 1.0)$. |
| **BUG-003** | `eda_clean` 5 Hz lowpass fails for $F_s \le 10\text{ Hz}$ | `confirmed` | High | Phase 2 | Experimentally evaluate pass-through, $1.0\text{ Hz}$, $1.5\text{ Hz}$, and $1.8\text{ Hz}$ cutoffs at $4\text{ Hz}$ EDA. Expose `EdaCleaningConfig`. |
| **BUG-004** | `eda_findpeaks` / `rsp_findpeaks` hardcode 100 Hz | `confirmed` | Medium | Phase 2 | Require explicit `sampling_rate: f64` parameter in `eda_findpeaks(phasic, fs)` and `rsp_findpeaks(cleaned, fs)`. |
| **BUG-005** | Inconsistent rPPG algorithm output polarity | `confirmed` | Medium | Phase 3 | Define formal rPPG/BVP pulse-phase contract. Expose `SignalPolarity` and `.to_bvp_waveform()` on `RppgSignal` to normalize output phase for downstream peak detection. |
| **BUG-006** | `sample_entropy` returns `Ok(+inf)` on zero matches | `confirmed` | Low | Phase 1 | Document `Ok(f64::INFINITY)` contract for zero template matches; validate tolerance $r > 0$. |
| **BUG-007** | `rsp_cycles_config` re-cleans pre-cleaned input | `confirmed` | Low | Phase 2 | Add `precleaned: Option<bool>` field to `RspProcessingConfig` to allow callers to skip redundant filtering. |
| **BUG-008** | `FeatureQuality.total_feature_count` hardcoded to 30 | `confirmed` | Low | Phase 1 | Dynamically calculate denominator from actual countable feature set ($28$). |
| **BUG-009** | Parity tests panic on missing golden files | `confirmed` | Low | Phase 1 | Check canonical `golden_filter.json` and `golden_peaks.json` fixtures into `tests/`. |
| **BUG-010** | `multimodal_quality` rustdoc names wrong error | `confirmed` | Informational | Phase 1 | Correct rustdoc comment to state `SignalError::EmptySignal`. |
| **BUG-011** | ECG under-detection on tall PVCs (MIT-BIH 228) | `suspected` | Medium | Phase 4 | Root-Cause Gate: Construct minimal reproduction of multiform PVC amplitude disparity, evaluate SPKI/NPKI adaptation hypotheses, test against ECG regression matrix. |
| **BUG-012** | ECG blackout (~90s) on narrow biphasic QRS | `suspected` | Medium | Phase 4 | Root-Cause Gate: Instrument threshold and searchback state on wrist `s6_low_resistance_bike`, verify if derivative scaling or refractory state causes blackout, apply verified fix. |
| **BUG-013** | Paced beat under-detection channel sensitivity | `ambiguous` | Low | Phase 4 | Document paced-beat lead sensitivity; verify Pan-Tompkins behavior on pacemaker impulse spikes. |
| **BUG-014** | High SCR rate on 700 Hz WESAD chest EDA | `ambiguous` | Low | Phase 2 | Evaluate `eda_findpeaks_events` on $700\text{ Hz}$ vs downsampled $10\text{ Hz}$ to ensure rise-time and amplitude thresholds scale correctly with $F_s$. |
| **BUG-015** | Wrist BVP HR gap vs Empatica E4 HR.csv | `ambiguous` | Low | Phase 3 | Document E4 HR reference limitations; verify BVP peak detection against beat-level ground truth. |
| **BUG-016** | ECG/PPG over-counting under exercise motion | `limitation` | Medium | Phase 3 | Document motion artifact limitation for single-channel detectors; outline architecture for accelerometer-coupled PPG artifact rejection. |
| **BUG-017** | Respiration over-detection at ~6 brpm / 12s ceiling | `limitation` | Low | Phase 2 | Increase default `max_breath_interval_sec` to `20.0` ($3\text{ brpm}$) and add secondary peak prominence gating. |
| **BUG-018** | HRV RMSSD computed over raw RR intervals with ectopy | `limitation` | Informational | Phase 5 | Design explicit interval quality & correction architecture (`RR` $\to$ `Quality` $\to$ `CorrectionPolicy` $\to$ `NN` $\to$ `HRV`). Validate NN-derived HRV against annotation ground truth. |
| **BUG-019** | `partial_cmp().unwrap()` panic on NaN in `peaks.rs` | `suspected` | Low | Phase 1 | Replace `partial_cmp(b).unwrap()` with `total_cmp(b)` in `src/ecg/peaks.rs:218`. |
| **BUG-020** | Near-flatline ECG lead II under-detection (BIDMC 03) | `dataset-issue` | Informational | Phase 1 | Document low-voltage lead II anomaly; verify `evaluate_ecg_quality` reports low signal quality. |

---

## 4. Deep-Dive Design Specifications

### 4.1 Sampling-Rate Architecture & Wearable EDA (Phase 2)

#### 1. Explicit Sampling-Rate Parameters
- Eliminate all hidden $100\text{ Hz}$ default assumptions. Functions `eda_findpeaks(phasic, fs)` and `rsp_findpeaks(cleaned, fs)` require explicit `sampling_rate: f64`.
- Validate $F_s > 0.0$ and $F_s \ge 2 \cdot f_{\text{high}}$ across all domain entry points.

#### 2. Experimental EDA Passband Evaluation ($4.0\text{ Hz}$)
At $4.0\text{ Hz}$ EDA (Empatica E4, Nyquist $= 2.0\text{ Hz}$), do not enforce a mechanical cutoff formula. Execute an experimental evaluation matrix comparing:
- Pass-through (no low-pass filter; Nyquist anti-aliasing inherent at $4.0\text{ Hz}$)
- Low-pass $1.0\text{ Hz}$
- Low-pass $1.5\text{ Hz}$
- Low-pass $1.8\text{ Hz}$

Evaluate the effect of each candidate on:
- SCR pulse morphology and amplitude
- SCR onset and peak detection sensitivity
- Tonic vs. Phasic decomposition reconstruction accuracy ($\text{tonic} + \text{phasic} = \text{cleaned}$)
- Downstream feature extraction

Document the selected configuration with signal-processing rationale in `docs/architecture/signal-contracts.md` and expose `EdaCleaningConfig`.

---

### 4.2 rPPG / PPG Signal Semantics & Polarity Contract (Phase 3)

#### 1. Formal Signal Representation Contract (`docs/architecture/signal-contracts.md`)
Establish explicit type definitions and documented semantic contracts:
- `RawIntensity`: Spatial mean RGB intensity. Decreases during cardiac systole as tissue blood volume increases.
- `RppgSignal`: Normalized chrominance or plane-orthogonal projection waveform.
- `BvpWaveform`: Normalized optical pulse surrogate expressed under Lamina's documented pulse-phase convention. Positive deflection represents the pulse wave peak to ensure downstream Elgendi peak detector consistency. It does **not** imply direct measurement of absolute arterial blood volume.

#### 2. Polarity Configuration & Conversion
- Introduce `enum SignalPolarity { Standard, Inverted }` in `RppgConfig`.
- Provide `.to_bvp_waveform()` on `RppgSignal` that transforms output signals into Lamina's pulse-phase convention based on algorithm identity (`CHROM` vs. `GreenChannel` / `POS`) or user override.

---

### 4.3 ECG Root-Cause Investigation Gate & Regression Matrix (Phase 4)

#### 1. Mandated Root-Cause Gate
Before modifying `src/ecg/peaks.rs`:
1. **Replicate Failure Mechanisms:** Instrument $SPKI$/$NPKI$ adaptation state and searchback triggers on MIT-BIH 228 (tall PVCs) and Wrist `s6_low_resistance_bike` (narrow biphasic blackout).
2. **Minimized Fixtures:** Construct isolated synthetic QRS test fixtures for (a) $3:1$ PVC amplitude disparity and (b) narrow biphasic QRS runs.
3. **Validated Fix:** Implement modifications only if the underlying mechanism is demonstrated on the minimized fixture.

#### 2. Comprehensive ECG Regression Matrix

| Test Case | Dataset / Fixture | Pass Criteria |
| :--- | :--- | :--- |
| **Normal Sinus QRS** | MIT-BIH 100 | $F_1 \ge 0.995$ |
| **Multiform PVC Disparity** | MIT-BIH 228 | Normal beat recall $\ge 0.950$ (baseline $0.602$) |
| **Ventricular Bigeminy** | Autonomic Aging 0554 | $F_1 \ge 0.980$ |
| **Narrow Biphasic QRS** | Wrist `s6_low_resistance_bike` | Zero 30s blackouts; $F_1 \ge 0.900$ |
| **Paced Pulses** | MIT-BIH 104 (Lead V2 & V5) | Lead V2 $F_1 \ge 0.980$ |
| **Noise Gradient** | MIT-BIH NSTDB 118 | $18\text{ dB } F_1 \ge 0.990$; $0\text{ dB } F_1 \ge 0.830$ |

---

### 4.4 Decoupled HRV & Interval Quality Architecture (Phase 5)

#### 1. Five-Stage Decoupled Pipeline
Do not embed a fixed $20\%$ ectopy filter inside HRV calculation. Implement an explicit, decoupled architecture:

$$\text{Beat} \xrightarrow{\text{BeatQuality}} \text{Interval} \xrightarrow{\text{IntervalQuality}} \text{CorrectionPolicy} \xrightarrow{\text{NN Series}} \text{hrv\_rmssd}$$

#### 2. API Contract
```rust
pub enum BeatQuality { Normal, Ectopic, Artifact, Unknown }
pub enum IntervalQuality { NormalNN, EctopicRR, ArtifactRR, Missing }

pub enum CorrectionPolicy {
    /// Retain all raw intervals (default R-R analysis)
    None,
    /// Remove ectopic and artifact intervals without replacement
    RejectInvalid,
    /// Replace ectopic intervals via linear interpolation
    InterpolateLinear,
    /// Replace ectopic intervals via cubic spline interpolation
    InterpolateCubic,
    /// Filter intervals exceeding relative change threshold
    PercentThreshold(f64),
}

pub struct IntervalCleaningConfig {
    pub policy: CorrectionPolicy,
    pub min_valid_interval_ms: f64, // e.g. 300.0 ms
    pub max_valid_interval_ms: f64, // e.g. 2000.0 ms
}

pub fn clean_rr_intervals(rr: &Array1<f64>, config: &IntervalCleaningConfig) -> Result<Array1<f64>>;
```

#### 3. HRV Success Criterion
Demonstrate that `clean_rr_intervals` appropriately identifies and filters ectopic intervals on synthetic and real bigeminy recordings (e.g. Autonomic Aging 0554), and that resulting NN-derived HRV metrics agree with annotation-derived reference HRV within $\pm 5\%$.

---

## 5. Formal Execution Phases

```text
PHASE 0 — Scientific/API Contract Review & Protobuf Alignment
  ├── Write docs/architecture/signal-contracts.md
  └── Review protobuf definitions in ~/development/roeh-health/sensor_messages (branch: support-lamina-upgrade)
        ↓
PHASE 1 — Low-Risk Correctness
  ├── ecg_clean method string dispatch ("neurokit", "pantompkins")
  ├── ecg_findpeaks_config invalid threshold_multiplier error range validation
  ├── sample_entropy Ok(+inf) contract documentation & r > 0 check
  ├── FeatureQuality dynamic denominator (28)
  ├── Check golden test files (golden_filter.json, golden_peaks.json) into tests/
  ├── Replace partial_cmp().unwrap() with total_cmp() in src/ecg/peaks.rs
  └── Correct multimodal_quality rustdoc
        ↓
PHASE 2 — Sampling-Rate & EDA Verification
  ├── Mandatory sampling_rate in eda_findpeaks and rsp_findpeaks
  ├── Experimental 4-Hz EDA passband evaluation & EdaCleaningConfig
  ├── precleaned: Option<bool> in RspProcessingConfig
  ├── max_breath_interval_sec default increased to 20.0s (3 brpm)
  └── Multi-sampling-rate test suite (4 Hz to 700 Hz)
        ↓
PHASE 3 — rPPG & Signal Semantics Design
  ├── Formal rPPG/BVP pulse-phase contract in docs/architecture/signal-contracts.md
  ├── SignalPolarity parameterization & .to_bvp_waveform() on RppgSignal
  └── Document motion artifact limitations for single-channel detectors
        ↓
PHASE 4 — ECG Root-Cause Investigation & Fixes
  ├── Execute Root-Cause Gate for MIT-BIH 228 and Wrist s6
  ├── Implement verified QRS derivative scaling & SPKI/NPKI threshold adaptation bounds
  └── Evaluate against full 6-case ECG Regression Matrix
        ↓
PHASE 5 — HRV & Interval Architecture Design
  ├── Implement BeatQuality, IntervalQuality, CorrectionPolicy, and clean_rr_intervals
  ├── Document RR vs NN interval semantics
  └── Validate NN-derived HRV against annotation ground truth
        ↓
PHASE 6 — Cross-Language & Protobuf Review
  ├── Align protobuf schemas in ~/development/roeh-health/sensor_messages
  └── Update Dart (lamina_dart) and Python (lamina_bridge) bindings
        ↓
PHASE 7 — Full Dataset Revalidation
  ├── Re-run MIT-BIH, BIDMC, WESAD, Big-Ideas, Wearable Exam Stress, SCAMPS
  └── Report per-record before/after metrics against baseline
```

---

## 6. Regression Gates vs. Success Criteria

- **Suite-Level Regression Gate (Hard Floor):** MIT-BIH 48-record suite mean $F_1 \ge 0.982$, BIDMC respiration cycle $F_1 \ge 0.944$. No statistically or materially meaningful aggregate regression across validated suites.
- **Per-Record Regression Gate:** Flag and inspect any individual record whose $F_1$ or MAE regresses materially ($>1.5\%$) against baseline.
- **Success Criteria:**
  - Big-Ideas and Wearable Exam Stress $4.0\text{ Hz}$ EDA pipeline execution succeeds with $100\%$ completion.
  - MIT-BIH Record 228 normal beat recall increases from $0.602$ to $\ge 0.950$.
  - SCAMPS `GreenChannel` peak $F_1$ increases from $0.245$ to $\ge 0.750$.
  - NN-derived HRV metrics agree with annotation-derived reference HRV within $\pm 5\%$ on arrhythmic recordings.
