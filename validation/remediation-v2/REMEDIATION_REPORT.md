# Lamina v2 Remediation Report

## Executive Summary

This report documents the remediation of the remaining findings identified in the independent **Lamina Revalidation v2** suite (`validation/revalidation-v2/REVALIDATION_REPORT.md`). All changes were executed under strict signal-processing contracts and verified against the complete 12-phase regression framework.

### Summary of Results
- **BIDMC Respiration Remediation**: Corrected candidate inspiratory peak extraction to require positive expansion excursions above the central signal baseline ($x[i] > \bar{x}$). `bidmc05` F1 improved from **0.6575** to **1.0000** (TP=48, FP=0, FN=0). Mean per-record BIDMC F1 reached **0.977525** (exceeding the regression gate of $\ge 0.9440$).
- **ECG Baseline Protection**: 100% preserved. MIT-BIH 48-record mean F1 remains **0.993686** (gate $\ge 0.9820$), total FP remains **499** (gate $\le 750$). Record 228 recall remains **0.981491** (F1 = **0.989200**). Record 123 (**0.999011**) and Record 232 (**0.999158**) remain 100% restored. Six-case matrix alignment remains 100% OK.
- **Wrist ECG `s6`**: Longest detection gap remains **1.80 s** ($\le 5.0\text{ s}$ gate; 90-second blackout 100% eliminated), Recall = **0.970149**, F1 = **0.926916**.
- **HRV Empty NN Contract**: Established precise API semantics for empty container vs 0-NN post-filter results. `clean_rr_intervals` returns `Ok(Array1::zeros(0))` when filtering purges 100% of non-normal intervals from a non-empty container, and `Err(SignalError::EmptySignal)` when passed an empty container. Downstream `hrv_mean_nn` and `hrv_rmssd` return `Err(SignalError::InsufficientPeaks { required, provided: 0 })`.
- **Natural Cubic Spline Verification**: 100% verified. Confirmed mathematical invariants across 2, 3, 4, 5+ points, uniform/non-uniform spacing, monotonic/curved signals, clamped endpoints, linear fallback when valid points $<4$, and zero NaNs/Infs.
- **rPPG Polarity & AutoDetect Contract**: Documented `SignalPolarity::Inverted` as the normative production path. Clarified `AutoDetect` skewness-based heuristic behavior and operational boundaries.
- **Protobuf / Cross-Language Integrity**: 20/20 `CorrectionPolicy` and 9/9 `RppgSignal` metadata round-trips pass across Rust, Python, and Dart.
- **API & Numerical Hygiene**: 14/14 probe ops produced zero panics and zero unhandled crashes.

---

## 1. Phase 1 — BIDMC Respiration Remediation

### Finding
In the v2 validation, BIDMC mean per-record F1 was **0.943736** (unrounded gate $\ge 0.9440$). The discrepancy was concentrated in `bidmc05` ($F1 = 0.6575$, 50 FP cycles).

### Root Cause Analysis
During expiration in shallow/irregular breathing, minor sub-baseline ripples ($x[i] \le 0$) occur in the expiratory trough. In `rsp_cycles_config`, `PeakDetectionConfig` was initialized with `min_distance` only without a `min_height` constraint. Sub-baseline trough ripples were selected as candidate peaks. When paired with adjacent deep troughs, the rise from the deep trough to the sub-zero ripple exceeded `min_amplitude`, causing false positive respiration cycle detections.

### General Signal-Contract Remediation
Inspiratory peaks represent respiratory expansion maxima that rise above the central respiratory baseline ($\bar{x} = \text{mean}(\text{cleaned})$). Sub-baseline local extrema ($x[i] \le \bar{x}$) are expiratory trough ripples, not inspiratory peaks.

`src/rsp/peaks.rs` was updated to configure candidate peak extraction with:
```rust
let mean_val = cleaned.mean().unwrap_or(0.0);
let peak_cfg = PeakDetectionConfig::new()
    .with_min_distance(min_dist_samples)
    .with_min_height(mean_val);
let candidate_peaks = signal_findpeaks_config(&cleaned, &peak_cfg)?;
```
This is invariant to DC offset and applies equally to zero-mean bandpassed signals ($\bar{x} \approx 0.0$) and precleaned signals with arbitrary DC offsets.

### Regression Tests Added
- `test_rsp_sub_baseline_ripple_rejection`: Verifies that sub-baseline expiratory trough ripples are rejected and only main inspiratory expansion peaks above the signal mean are detected.
- `test_rsp_dc_offset_invariance`: Verifies that adding a +100.0 DC offset to a precleaned signal produces identical inspiratory and expiratory cycle indices.

### Per-Record BIDMC Before / After Results

| Record | Ref Peaks | Det Before | Det After | TP | FP | FN | Precision | Recall | F1 Before | F1 After | Delta F1 |
|---|---|---|---|---|---|---|---|---|---|---|---|
| `bidmc01` | 165 | 169 | 169 | 165 | 4 | 0 | 0.976331 | 1.000000 | 0.988024 | 0.988024 | 0.000000 |
| `bidmc02` | 118 | 124 | 120 | 116 | 4 | 2 | 0.966667 | 0.983051 | 0.966942 | 0.974790 | +0.007848 |
| `bidmc03` | 136 | 141 | 141 | 135 | 6 | 1 | 0.957447 | 0.992647 | 0.974729 | 0.974729 | 0.000000 |
| `bidmc04` | 126 | 144 | 132 | 125 | 7 | 1 | 0.946970 | 0.992063 | 0.925926 | 0.968992 | +0.043066 |
| `bidmc05` | 48 | 98 | 48 | 48 | 0 | 0 | 1.000000 | 1.000000 | **0.657534** | **1.000000** | **+0.342466** |
| `bidmc06` | 156 | 158 | 156 | 152 | 4 | 4 | 0.974359 | 0.974359 | 0.980892 | 0.974359 | -0.006533 |
| `bidmc07` | 156 | 159 | 158 | 154 | 4 | 2 | 0.974684 | 0.987179 | 0.984127 | 0.980892 | -0.003235 |
| `bidmc08` | 164 | 168 | 168 | 162 | 6 | 2 | 0.964286 | 0.987805 | 0.975904 | 0.975904 | 0.000000 |
| `bidmc09` | 157 | 159 | 159 | 156 | 3 | 1 | 0.981132 | 0.993631 | 0.987342 | 0.987342 | 0.000000 |
| `bidmc10` | 142 | 150 | 147 | 141 | 6 | 1 | 0.959184 | 0.992958 | 0.965753 | 0.975779 | +0.010026 |
| `bidmc11` | 113 | 122 | 117 | 110 | 7 | 3 | 0.940171 | 0.973451 | 0.944681 | 0.956522 | +0.011841 |
| `bidmc12` | 146 | 150 | 150 | 144 | 6 | 2 | 0.960000 | 0.986301 | 0.972973 | 0.972973 | 0.000000 |
| **Mean** | — | — | — | — | — | — | **0.966770** | **0.988620** | **0.943736** | **0.977525** | **+0.033789** |

---

## 2. Phase 2 — HRV Empty NN Result Contract

### Contract Specification
- **Empty Container (`n == 0`)**: Passing an empty input container (`rr_intervals.len() == 0`) to `clean_rr_intervals`, `hrv_mean_nn`, or `hrv_rmssd` returns `Err(SignalError::EmptySignal)`.
- **Post-Filter Empty (`n > 0` but 0 NN remaining)**: Passing a non-empty container where 100% of intervals are rejected (e.g. sustained bigeminy `[600.0, 1000.0, 600.0, 1000.0]`):
  - `clean_rr_intervals` returns `Ok(Array1::zeros(0))` (valid input container, 0 NN intervals remaining).
  - `hrv_mean_nn` returns `Err(SignalError::InsufficientPeaks { required: 1, provided: 0 })`.
  - `hrv_rmssd` returns `Err(SignalError::InsufficientPeaks { required: 2, provided: 0 })`.
  - `hrv_rmssd` on 1 NN interval returns `Err(SignalError::InsufficientPeaks { required: 2, provided: 1 })`.

### Tests Added
- `test_hrv_empty_container_and_post_filter_contract` in `tests/hrv_tests.rs`: Exercises true empty inputs vs 100% ectopic series across `RejectInvalid`, `InterpolateLinear`, and `InterpolateCubic`, verifying exact error variants and bridge JSON serialization.

---

## 3. Phase 3 — Natural Cubic Spline Verification

### Mathematical Contract Verification
Verified `natural_cubic_spline_interp` in `src/hrv/quality.rs`:
- **2 & 3 points**: Graceful linear fallback executed (`clean_rr_intervals` falls back to `InterpolateLinear` when valid points $<4$).
- **4+ points**: Natural cubic spline moments $M_0 = M_{n-1} = 0$ solved via $O(N)$ Thomas algorithm. Smooth non-linear curvature confirmed (differs from linear by up to 12.9899 ms on curved fixtures).
- **Endpoint Clamping**: $qx \le x_0 \implies y_0$ and $qx \ge x_{n-1} \implies y_{n-1}$ verified.
- **Finite Output**: Zero NaNs/Infs across all test cases.

### Tests Added
- `test_natural_cubic_spline_contract` in `tests/hrv_tests.rs`: Exercises $<4$ point linear fallback, 4+ point curvature, and endpoint clamping.

---

## 4. Phase 4 — rPPG Polarity and AutoDetect Contract

### Polarity Architecture & Conventions
- **Physical Absorption Basis**: In tissue oximetry, blood volume pulse expansion increases light absorption, reducing reflected/transmitted light intensity ($I$).
- **Explicit Polarity (`SignalPolarity::Inverted`)**: Normative production default (`RppgConfig::default().polarity = Inverted`). Negates extracted raw optical surrogate waveforms to produce positive systolic peak excursions.
- **Explicit Polarity (`SignalPolarity::Normal`)**: Pass-through raw optical surrogate without sign inversion.
- **AutoDetect (`SignalPolarity::AutoDetect`)**: Skewness-based heuristic fallback ($S = \frac{1}{N} \sum (\frac{x-\mu}{\sigma})^3 > 0.3$). Documented as an experimental heuristic; explicit polarity remains normative.

---

## 5. Protected ECG & Fleet Regression Summary

| Suite / Dataset | Baseline Metric | Target Gate | Current Candidate | Status |
|---|---|---|---|---|
| **MIT-BIH 48 Records** | Mean F1 = 0.993686 | F1 $\ge 0.9820$ | **0.993686** | **PASSED** |
| **MIT-BIH Total FP** | FP = 499 | FP $\le 750$ | **499** | **PASSED** |
| **Record 228** | Recall = 0.981491, F1 = 0.989200 | Recall $\ge 0.9500$ | **Recall = 0.981491, F1 = 0.989200** | **PASSED** |
| **Record 123** | F1 = 0.999011 | Restored | **0.999011** | **RESTORED** |
| **Record 232** | F1 = 0.999158 | Restored | **0.999158** | **RESTORED** |
| **Six-Case ECG Matrix** | 6/6 Align OK | 6/6 Align OK | **6/6 Align OK** | **PASSED** |
| **Wrist s6** | F1 = 0.9269, Gap = 1.80 s | Blackout resolved | **F1 = 0.9269, Gap = 1.80 s** | **PASSED** |
| **BIDMC Respiration** | Mean F1 = 0.943736 | F1 $\ge 0.9440$ | **0.977525** | **PASSED** |
| **EDA 4-Hz Wearable** | 100% completion, 0 NaNs | 0 NaNs | **100% completion, 0 NaNs** | **PASSED** |
| **Protobuf Roundtrips** | 20/20 Policy, 9/9 Rppg | 100% pass | **20/20 Policy, 9/9 Rppg** | **PASSED** |
| **API Hygiene Probes** | 14 probes, 0 panics | 0 panics | **14 probes, 0 panics** | **PASSED** |

---

## 6. Git Hygiene & Final Verification

- `cargo fmt -- --check`: **PASSED**
- `cargo clippy --all-targets --all-features -- -D warnings`: **PASSED**
- `cargo test --all-features`: **PASSED** (145/145 Rust unit & integration tests)
- `sensor_messages` Cross-Language Suite: Rust (4/4 pass), Python (3/3 pass), Dart (3/3 pass).

---

## Conclusion

All remaining findings from the Lamina v2 revalidation suite have been remediated with scientifically defensible, minimal, and fully tested signal-processing contracts. All protected ECG and multimodal performance baselines remain intact.
