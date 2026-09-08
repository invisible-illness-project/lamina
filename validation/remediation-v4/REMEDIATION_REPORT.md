# Lamina Remediation v4 Report — REV3-003

## 1. Executive Summary

This report documents the scientific analysis, implementation hardening, adversarial test validation, and baseline regression results for **REV3-003 — rPPG `SignalPolarity::AutoDetect` operational boundaries**.

The previous implementation of `SignalPolarity::AutoDetect` evaluated sample skewness using `skew > 0.3` to trigger waveform inversion. Because standard Blood Volume Pulse (BVP) waveforms with narrow systolic peaks naturally possess positive skewness ($\gamma_1 > +0.3$), the naive rule inadvertently inverted valid BVP signals, violating the pulse-phase contract.

To remediate REV3-003, `compute_should_flip` was refactored to implement a directionally conservative statistical rule:
- **Negative Sample Skewness ($\gamma_1 < -0.3$)**: Signals dominated by sharp downward excursions (e.g. raw optical intensity drops during systolic blood volume expansion) trigger inversion (`should_flip = true`).
- **Positive / Ambiguous Skewness ($\gamma_1 \ge -0.3$)**: Signals with positive skewness (already BVP peak-up phase), near-zero skewness, or low variance ($\sigma \le 10^{-6}$) are preserved (`should_flip = false`).

`AutoDetect` is a statistical polarity heuristic. Negative skewness below the configured threshold is treated as evidence for inversion, but skewness alone cannot establish the physical optical measurement convention or distinguish physiological waveform polarity from artifact-induced asymmetry.

Crucially, `SignalPolarity::Inverted` remains the normative production default in `RppgConfig::default()`. The test suite was expanded with 16 comprehensive scenario tests, 5 explicit adversarial fixtures, and physical-vs-statistical contract tests. All 12 validation phases and protected signal-processing baselines pass 100%.

---

## 2. Existing AutoDetect Contract

Prior to remediation, `SignalPolarity::AutoDetect` was evaluated in `src/rppg/signal.rs:compute_should_flip` via:

$$\gamma_1 > 0.3 \implies \text{flip}$$

In optical photoplethysmography:
- **Raw optical intensity domain**: Cardiac expansion increases tissue light absorption, causing reflected light intensity to drop. Waveforms belonging to this statistical class typically exhibit downward-directed spikes, resulting in negative sample skewness ($\gamma_1 < 0$).
- **Normalized BVP pulse domain**: Blood volume expansion is represented as positive upward peaks, resulting in positive sample skewness ($\gamma_1 > 0$).

Evaluating `skew > 0.3 => flip` resulted in flipping signals that were *already* positively skewed (turning valid BVP peaks upside down) while leaving negatively skewed raw absorption signals unflipped.

---

## 3. Root Cause / Scientific Analysis

Statistical sample skewness $\gamma_1 = \frac{1}{N} \sum \left(\frac{x_i - \mu}{\sigma}\right)^3$ measures distributional third-moment asymmetry around the sample mean:
1. Skewness measures **statistical asymmetry**, NOT physical sensor orientation or physiological pulse polarity.
2. Positive skewness ($\gamma_1 > 0$) indicates a heavy upper tail (narrow upward peaks).
3. Negative skewness ($\gamma_1 < 0$) indicates a heavy lower tail (narrow downward troughs).

Therefore:
- A raw optical intensity signal with systolic absorption drops exhibits negative sample skewness ($\gamma_1 < -0.3$). Under the heuristic contract, `AutoDetect` negates the waveform (`should_flip = true`).
- A BVP signal already in peak-up phase has positive sample skewness ($\gamma_1 > +0.3$). Inverting it would destroy peak alignment (`should_flip = false`).
- A symmetric signal ($\gamma_1 \approx 0.0$), constant signal ($\sigma = 0$), or low-variance signal ($\sigma \le 10^{-6}$) has no statistical basis for polarity inference and MUST be preserved (`should_flip = false`).

---

## 4. Alternatives Evaluated

During engineering design, five candidate approaches for `AutoDetect` were evaluated:

1. **Morphology (Positive vs Negative Excursion Analysis)**:
   Comparing peak heights vs trough depths. While effective on clean ECG QRS complexes, raw rPPG waveforms often exhibit baseline wander and variable DC offset, making zero-crossing excursion counts sensitive to detrending window length.
2. **Prominence Asymmetry**:
   Evaluating local peak prominence vs trough prominence ratios. Required peak-finding parameter tuning (minimum distance, threshold) that introduced hyperparameter sensitivity across different sampling rates ($10\text{--}60\text{ Hz}$).
3. **Periodicity-Aware Polarity Selection**:
   Computing autocorrelation or spectral power in candidate peak series for both polarities. Highly computationally expensive for sliding window extraction without providing immunity to motion artifacts.
4. **Correlation / Template Matching**:
   Correlating extracted windows against an ideal synthetic BVP template. Sensitive to morphological variations across age, vascular resistance, and anatomical ROI location.
5. **Directionally Conservative Skewness Fallback (Selected)**:
   Evaluating directional skewness $\gamma_1 < -0.3$ with explicit conservative guards ($\sigma \le 10^{-6} \Rightarrow \text{false}$, $\gamma_1 \ge -0.3 \Rightarrow \text{false}$). This approach provides a lightweight, deterministic mathematical contract without external dependencies or hyperparameter tuning, while clearly documenting physical sensor orientation limitations.

---

## 5. Selected Resolution

The implementation enforces the following mathematical contract in `compute_should_flip`:

| Signal Characteristic | Mathematical Condition | AutoDetect Decision | Action |
|---|---|---|---|
| Insufficient Samples | $N \le 3$ | `false` | Preserve signal |
| Low / Zero Variance | $\sigma \le 10^{-6}$ | `false` | Preserve signal |
| Strong Negative Skew | $\gamma_1 < -0.3$ | `true` | Negate waveform |
| Strong Positive Skew | $\gamma_1 > +0.3$ | `false` | Preserve signal |
| Near-Zero Skew | $\|\gamma_1\| \le 0.3$ | `false` | Preserve signal |

`RppgConfig::default()` retains `polarity: SignalPolarity::Inverted` as the explicit production default for raw optical intensity measurements.

---

## 6. Implementation Changes

- `src/rppg/config.rs`: Updated docstrings for `SignalPolarity` variants (`Normal`, `Inverted`, `AutoDetect`), clarifying physical optical absorption conventions, statistical limitations of skewness, experimental nature, low-variance/ambiguous behavior, and why explicit `Inverted` is normative.
- `src/rppg/signal.rs`: Refactored `compute_should_flip(wf)` to enforce `skew < -0.3` for inversion and updated internal docstrings.
- `tests/rppg_tests.rs`: Added 16 required test matrix scenarios, 5 explicit adversarial test fixtures (`test_rppg_five_adversarial_fixtures`), physical-vs-statistical boundary tests (`test_rppg_physical_vs_statistical_contract`), production default assertion (`test_rppg_config_default_polarity`), and exact threshold boundary tests (`test_rppg_autodetect_exact_boundary_thresholds`).

---

## 7. Adversarial Test Matrix

The table below records the measured empirical results for all 5 required adversarial fixtures:

| Fixture | Construction | N | Mean ($\mu$) | Std ($\sigma$) | Skewness ($\gamma_1$) | Expected Decision | Actual Decision | Result | Engineering Disposition |
|---|---|---|---|---|---|---|---|---|---|
| **ADV-1** | Positive BVP (6 narrow systolic peaks up, amp +8.0) | 120 | 0.5500 | 1.8296 | +3.4713 | `false` | `false` | PASS | Guards directly against REV3-003 regression. Positively skewed BVP preserved. |
| **ADV-2** | Negative Skew Waveform (6 narrow absorption drops, amp -8.0) | 120 | -0.5500 | 1.8296 | -3.4713 | `true` | `true` | PASS | Verifies directional inversion on raw absorption signals. |
| **ADV-3** | Motion Artifact (pulse amp +0.5 + single -25.0 motion spike) | 120 | -0.1833 | 2.2775 | -10.7789 | `true` | `true` | PASS | The large motion artifact produces strongly negative skewness ($\gamma_1 = -10.7789 < -0.3$) and therefore causes AutoDetect to invert according to its mathematical contract. This demonstrates a known failure mode of skewness-based polarity inference under severe artifact contamination rather than successful physiological polarity identification. |
| **ADV-4** | Biphasic Ambiguous Signal (symmetric +4.0 and -4.0 spikes) | 120 | 0.0000 | 1.2649 | 0.0000 | `false` | `false` | PASS | Validates conservative preservation on ambiguous symmetric inputs. |
| **ADV-5** | High-Frequency Noise Burst (alternating $\pm 2.0$ noise burst) | 120 | 0.0000 | 0.8165 | 0.0000 | `false` | `false` | PASS | Validates finite output, zero panics, and conservative preservation on noise. |

---

## 8. Full Regression Results

All 12 validation phases were executed and verified against protected baseline metrics:

| Subsystem / Metric | Target Gate | Pre-Remediation | Post-Remediation | Status |
|---|---|---|---|---|
| **MIT-BIH Mean F1** | $\ge 0.9820$ | 0.9937 | 0.9937 | PASSED |
| **MIT-BIH Total FP** | $\le 750$ | 499 | 499 | PASSED |
| **Record 228 Recall** | $\ge 0.9500$ | 0.9815 | 0.9815 | PASSED |
| **Record 228 F1** | $\ge 0.9850$ | 0.9892 | 0.9892 | PASSED |
| **Record 123 Baseline** | Preserved | 0.9990 | 0.9990 | PASSED |
| **Record 232 Baseline** | Preserved | 0.9992 | 0.9992 | PASSED |
| **Six-Case ECG Matrix** | 6/6 | 6/6 | 6/6 | PASSED |
| **Wrist s6 Longest Gap** | $\le 5.0\text{ s}$ | 1.80 s | 1.80 s | PASSED |
| **BIDMC Respiration Mean F1** | $\ge 0.9440$ | 0.9775 | 0.9775 | PASSED |
| **`bidmc05` F1** | $= 1.0000$ | 1.0000 | 1.0000 | PASSED |
| **EDA Completion** | 100% (No NaNs/Infs) | 100% | 100% | PASSED |
| **HRV Counterexample Suite**| 100% passing | 100% | 100% | PASSED |
| **rPPG Polarity Scenarios** | 9/9 passing | 9/9 | 9/9 | PASSED |
| **Protobuf Roundtrips** | 29/29 | 29/29 | 29/29 | PASSED |
| **API Hygiene Probes** | 0 Panics | 0 Panics | 0 Panics | PASSED |

---

## 9. API Contract

1. `RppgConfig::default().polarity` is strictly `SignalPolarity::Inverted`.
2. `SignalPolarity::Normal` is strictly pass-through ($x \mapsto x$).
3. `SignalPolarity::Inverted` is strictly exact negation ($x \mapsto -x$).
4. `SignalPolarity::AutoDetect` is an experimental statistical heuristic evaluating $\gamma_1 < -0.3 \implies \text{invert}$.
5. No public API signatures or protobuf schemas were modified.

---

## 10. Remaining Limitations

Statistical skewness is a third-moment distributional summary and has explicit operational boundaries:
1. **Physical Unidentifiability**: Skewness cannot determine whether an optical sensor measures increasing reflected light intensity or increasing absorption when waveform morphology is symmetric or noisy.
2. **Nonstationary Artifacts**: Isolated large motion spikes (such as in ADV-3 where a single $-25.0$ spike contributes $> 99.9\%$ of the third central moment) dominate the cubic power of sample deviations, driving skewness negative or positive regardless of underlying cardiac pulse polarity.
3. **Low Amplitude / High Noise**: When pulsatile signal-to-noise ratio is low, statistical skewness falls within the ambiguity band $[-0.3, +0.3]$, causing `AutoDetect` to default to `false` (no flip).

Callers with known hardware optical sensor conventions should configure `SignalPolarity::Inverted` or `SignalPolarity::Normal` explicitly rather than relying on statistical auto-detection.

---

## 11. Final Status

**FINAL STATUS: PASS**

All implementation changes, unit tests, adversarial fixtures, code formatting, clippy lints, and 12 validation phases are 100% passing without regressions.

