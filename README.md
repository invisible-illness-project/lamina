# Lamina 🦀 🐍 🎯

**Multi-Language Scientific Biosignal Processing Engine in Rust, Python, and Dart**

**Lamina** is a safe, high-performance, scientific computational engine for biomedical and physiological signal processing (ECG, PPG, EDA, RSP, HRV, Complexity, rPPG, and Multimodal Coupling). Inspired by Python's [NeuroKit2](https://github.com/neuropsychology/NeuroKit) and SciPy, Lamina provides zero-phase digital filtering, domain-specific physiological event extractors, and cross-modality cardiorespiratory coupling analysis with strict numerical stability, $O(N)$ linear complexity, and empirical scientific parity.

---

## ⚡ Quickstart across Rust, Python & Dart

Process physiological signals using identical underlying Rust algorithms with native language bindings:

### 🦀 Rust

Add to `Cargo.toml`:
```toml
[dependencies]
lamina = "0.1"
ndarray = "0.17"
```

```rust
use lamina::ecg::{ecg_clean, ecg_findpeaks};
use lamina::hrv::intervals::peaks_to_intervals;
use lamina::hrv::time::hrv_rmssd;
use ndarray::Array1;

fn main() -> lamina::Result<()> {
    let sampling_rate = 250.0;
    let raw_ecg = Array1::<f64>::zeros(2500); // Raw ECG waveform (10s @ 250Hz)

    // 1. Clean ECG signal (bandpass filter + baseline removal)
    let cleaned_ecg = ecg_clean(&raw_ecg, sampling_rate)?;

    // 2. Detect R-peak sample indices
    let peaks_mask = ecg_findpeaks(&cleaned_ecg, sampling_rate)?;

    // 3. Convert peaks to RR intervals (ms) and compute RMSSD
    let rr_intervals_ms = peaks_to_intervals(&peaks_mask, sampling_rate)?;
    let rmssd = hrv_rmssd(&rr_intervals_ms)?;
    println!("RMSSD: {:.2} ms", rmssd);

    Ok(())
}
```

---

### 🐍 Python

Install via `pip`:
```bash
pip install lamina
```

```python
import numpy as np
import lamina

sampling_rate = 250.0
raw_ecg = np.zeros(2500)  # Raw ECG waveform (10s @ 250Hz)

# 1. Clean ECG signal (bandpass filter + baseline removal)
cleaned_ecg = lamina.ecg.clean(raw_ecg, sampling_rate=sampling_rate)

# 2. Detect R-peak sample indices
r_peaks = lamina.ecg.findpeaks(cleaned_ecg, sampling_rate=sampling_rate)

# 3. Convert peaks to RR intervals (seconds) and compute RMSSD
rr_intervals = lamina.hrv.peaks_to_intervals(r_peaks, sampling_rate=sampling_rate)
rmssd = lamina.hrv.rmssd(rr_intervals)
print(f"RMSSD: {rmssd * 1000.0:.2f} ms")
```

---

### 🎯 Dart & Flutter

Add to `pubspec.yaml`:
```yaml
dependencies:
  lamina_dart: ^0.1.0
```

```dart
import 'dart:typed_data';
import 'package:lamina_dart/lamina.dart';

Future<void> main() async {
  // 1. Initialize native Rust computational engine
  await Lamina.init();

  final samplingRate = 250.0;
  final rawEcg = Float64List(2500); // Raw ECG waveform (10s @ 250Hz)

  // 2. Clean ECG signal (bandpass filter + baseline removal)
  final cleanedEcg = await Lamina.ecg.clean(rawEcg, samplingRate);

  // 3. Detect R-peak sample indices
  final rPeaks = await Lamina.ecg.findPeaks(cleanedEcg, samplingRate);

  // 4. Convert peaks to RR intervals and compute RMSSD
  final rrIntervals = await Lamina.hrv.peaksToIntervals(
    Int64List.fromList(rPeaks),
    samplingRate,
  );
  final rmssd = await Lamina.hrv.rmssd(rrIntervals);
  print('RMSSD: ${(rmssd * 1000.0).toStringAsFixed(2)} ms');
}
```

---

## Ecosystem Packages

| Ecosystem | Package Directory | Target | Documentation |
| :--- | :--- | :--- | :--- |
| **Rust** 🦀 | Core Crate (`src/`) | Native Rust applications, microcontrollers, servers | [API Inventory](./validation/API-INVENTORY.md) |
| **Python** 🐍 | [`lamina_py/`](./lamina_py/) | CPython / PyO3 data science, Jupyter, ML | [`lamina_py/README.md`](./lamina_py/README.md) |
| **Dart** 🎯 | [`lamina_dart/`](./lamina_dart/) | Flutter mobile, web, and desktop apps via FFI | [`lamina_dart/README.md`](./lamina_dart/README.md) |
| **Validation** 🧪 | [`validation/`](./validation/) | Reproducible public dataset validation suite | [`validation/README.md`](./validation/README.md) |

---

## Key Features & Modules

### 1. `lamina::signal` — Generic DSP & Math Core
- **Digital Butterworth SOS Filtering (`FilterSpec` & `SosFilter`)**: Lowpass, Highpass, Bandpass, and Notch IIR filter design via analog prototypes, frequency pre-warping, bilinear transform, and nearest pole-zero biquad SOS sectioning. Tested for exact frequency response invariants ($|H(f_c)| = 1/\sqrt{2} \approx -3.01\text{ dB}$).
- **Zero-Phase Digital Filtering (`signal_filtfilt`)**: SciPy-equivalent `sosfiltfilt` zero-phase forward-backward filtering utilizing Direct Form II Transposed (DF2T) biquads and odd-reflection boundary padding ($3 \times \text{order}$). Matches SciPy end-to-end with Max Abs Error $L_\infty < 1.0 \times 10^{-10}$.
- **Generic Peak Detection (`signal_findpeaks_config`)**: Configurable peak detection wrapping `find_peaks` with height, distance, prominence, width, and threshold constraints, verified against SciPy.
- **Fast Moving Average (`signal_smooth_moving_average`)**: Optimized prefix-sum sliding window accumulator operating in strict $O(N)$ time.

### 2. `lamina::ecg` — Electrocardiography Processing
- **Pan-Tompkins QRS Detection (`ecg_findpeaks_config`, `ecg_clean`)**: 
  - Canonical bandpass filtering ($5\text{--}15\text{ Hz}$).
  - 5-point slope derivative and non-linear power squaring.
  - $150\text{ ms}$ moving window integration.
  - Adaptive dual-thresholding ($SPKI$, $NPKI$), $200\text{ ms}$ refractory period enforcement, and searchback for missed beats ($RR > 1.66 \cdot RR_{\text{avg}}$).
  - Fine alignment to exact R-peak max amplitude within $\pm 150\text{ ms}$.

### 3. `lamina::ppg` — Photoplethysmography Processing
- **Elgendi Systolic Peak Detection (`ppg_findpeaks_config`, `ppg_clean`)**:
  - $0.5\text{--}8.0\text{ Hz}$ zero-phase Butterworth bandpass filter.
  - Non-linear clipping & squaring ($S[n] = \max(0, x[n])^2$).
  - Dual moving averages: Short MA ($W_{\text{peak}} \approx 111\text{ ms}$) & Long MA ($W_{\text{beat}} \approx 667\text{ ms}$).
  - Adaptive block thresholding ($THRESHOLD = MA_{\text{beat}} + 0.02 \cdot \bar{S}$) and $300\text{ ms}$ pulse refractory period.

### 4. `lamina::eda` — Electrodermal Activity Processing
- **Tonic / Phasic Decomposition (`eda_decompose`, `eda_clean`)**:
  - Zero-phase low-pass filtering ($5.0\text{ Hz}$ for noise cleaning, $0.05\text{ Hz}$ for Tonic SCL extraction).
  - Exact mathematical reconstruction: $\text{tonic}[n] + \text{phasic}[n] = \text{cleaned}[n]$.
- **SCR Event Detection (`eda_findpeaks_events`)**:
  - Characterizes Skin Conductance Response (SCR) events: `onset_index`, `peak_index`, `amplitude` ($\mu\text{S}$), and `rise_time_sec`.

### 5. `lamina::rsp` — Respiration Processing
- **Respiratory Band Filtering (`rsp_clean`, `rsp_clean_config`)**:
  - 3rd-order zero-phase Butterworth bandpass filtering ($0.05\text{--}0.50\text{ Hz}$, $3\text{--}30\text{ BPM}$).
- **Breath Cycle Construction (`RespirationCycle`, `rsp_cycles`)**:
  - Pairs inspiratory peaks ($i_k$) with intervening expiratory troughs ($e_k$) under strict ordering $i_k < e_k < i_{k+1}$.
  - Validates breath duration ($1.50\text{--}15.0\text{ s}$, $4\text{--}40\text{ BPM}$), minimum amplitude, and flatline noise floor ($A_{\text{p2p}} \ge 10^{-12}$).
- **Continuous Rate Array (`rsp_rate`, `rsp_rate_config`)**:
  - Piecewise constant interpolation with boundary extrapolation returning an $N$-length instantaneous rate series (BPM).

### 6. `lamina::hrv` — Heart Rate Variability
- **Time-Domain HRV (`hrv_rmssd`, `hrv_mean_nn`, `peaks_to_intervals`)**:
  - Converts peak detection masks into inter-beat interval series ($\text{ms}$).
  - Computes RMSSD (Root Mean Square of Successive Differences) and Mean NN interval.

### 7. `lamina::complexity` — Non-Linear Dynamics
- **Sample Entropy (`sample_entropy`)**:
  - Calculates Sample Entropy ($\text{SampEn}(m, r)$) measuring signal regularity and physiological complexity.

### 8. `lamina::multimodal` — Multimodal Physiological Feature & Coupling Layer
- **Physical Time Synchronization (`sync`)**: `sample_to_time` maps 0-indexed sample $n$ to physical timestamp $t = \text{offset} + n / F_s$ across heterogeneous sampling frequencies and hardware start offsets.
- **Respiratory Phase Mapping (`phase`)**: `respiratory_phase_at_time` maps timestamps into continuous normalized respiratory phase $\phi \in [0, 2\pi)$ ($[0, \pi)$ for inspiration, $[\pi, 2\pi)$ for expiration).
- **Respiratory Sinus Arrhythmia (`rsa`)**: `cardiac_respiratory_phase` and `rsa`/`rsa_config` compute beat-level respiratory phase and within-cycle peak heart rate modulation ($\Delta \text{BPM}$ and $\Delta \text{RR}_{\text{sec}}$).
- **Cardiorespiratory Phase Coupling (`coupling`)**: `cardiorespiratory_phase_coupling` computes circular concentration (resultant vector length $R \in [0.0, 1.0]$) and circular mean phase $\bar{\phi}$.
- **ECG-to-PPG Pulse Delay (`ecg_ppg`)**: `ecg_ppg_timing` performs deterministic two-pointer 1-to-1 matching pairing ECG R-peaks to following PPG pulse waves within $[0.10, 0.60]\text{ s}$.
- **EDA Associations (`eda_assoc`)**: `eda_cardiorespiratory_association` links SCR events to cardiac timestamps and respiratory phase.
- **Signal Quality Assessment (`quality`)**: `multimodal_quality`, `evaluate_ecg_quality`, and `evaluate_rsp_quality` transparent rule-based quality evaluation.

### 9. `lamina::features` — Windowed Multimodal Physiological Feature Extraction Layer
- **Window Generation (`generate_windows`, `FeatureWindow`)**: Generates sliding fixed-duration windows over physical timestamps $[t_{\text{start}}, t_{\text{end}})$ with configurable duration ($60.0\text{ s}$), step ($30.0\text{ s}$), and coverage. Includes `time_range_to_sample_range` for half-open $[t_{\text{start}}, t_{\text{end}})$ index conversion.
- **Cardiac Features (`cardiac_features`)**: Mean/median HR ($\text{BPM}$), SDNN ($\text{ms}$), RMSSD ($\text{ms}$ via `lamina::hrv`), pNN50 ($\%$), mean/std RR intervals ($\text{ms}$), and beat count. Implements Terminating R-peak boundary convention for crossing intervals and explicit population standard deviation.
- **EDA Features (`eda_features`)**: Mean/median/std Tonic SCL ($\mu\text{S}$), mean/std Phasic SCR ($\mu\text{S}$), SCR event count, normalized SCR rate ($\text{events/min}$), and mean/median SCR amplitude ($\mu\text{S}$) & rise time ($\text{seconds}$). Enforces $N_{\text{tonic}} == N_{\text{phasic}}$ signal dimension equality.
- **Respiration Features (`respiration_features`)**: Mean/median/std respiratory rate ($\text{BPM}$), mean cycle duration ($\text{seconds}$), cycle count, and amplitude statistics ($\text{peak-to-trough}$).
- **Coupling Features (`coupling_features`)**: Precomputes recording coupling streams (`pulse_delays`, `cr_phases`, `eda_assocs` in `PrecomputedCoupling`) and aggregates them per window. RSA modulation ($\Delta \text{BPM}$, $\Delta \text{RR}_{\text{sec}}$) is evaluated as a window-local coupling metric over window-bounded beats and cycles. Also provides cardiorespiratory phase concentration ($R \in [0, 1]$), mean phase ($\bar{\phi}$), mean/std ECG-PPG pulse delay ($\text{seconds}$), and SCR cardiorespiratory association counts.
- **Unified Feature Pipeline (`extract_features`, `MultimodalInput`, `MultimodalFeatureVector`)**: Accepts fallible builder inputs (`with_ecg`, `with_eda`, `with_rsp`, `with_ppg`) bundling `TimedEvents` and `TimedSignal` containers. Enforces non-decreasing chronological ordering (`SignalError::UnsortedEvents`). Monotonic range lookup (`EventCursor`) operates in $O(N + W)$ total time across the recording, eliminating per-window full scans ($O(N)$), while per-window feature statistics scale as $O(K)$ / $O(K \log K)$ over window-bounded events $K$. Supports missing modalities via `Option::None` without artificial zero-filling. Includes `extract_features_naive` reference oracle for exact parity testing.

### 10. `lamina::autonomic` — Interpretable Multimodal Physiological State Estimation Layer
- **Baseline Fitting & Normalization (`AutonomicBaseline`, `BaselineFeatureStats`)**: Fits location/scale statistics over baseline feature vectors using explicit **population standard deviation** ($\sigma = \sqrt{\frac{1}{N}\sum (x_i - \mu)^2}$). Enforces zero-variance policy via validated numerical floor threshold `min_scale` ($10^{-6}$). Applies $z$-scoring (Standard or Robust Median/MAD) and hyperbolic tangent scaling into $[-1.0, 1.0]$.
- **Structured Evidence States (`AutonomicState`, `CardiacState`, `ElectrodermalState`, `RespiratoryState`, `CouplingState`)**: Represents normalized physiological evidence without reinterpreting raw metrics as direct sympathetic/parasympathetic outflow (Carter et al., 2026). RespHRV coupling evidence strictly requires direct respiratory context (Buron & Menuet, 2026; Gevonden et al., 2025; 2025 RespHRV Expert Recommendation) and is marked unavailable (`None`) when absent. `RespiratoryState::regularity_index` captures baseline-relative rate regularity derived from inverse `rate_std_bpm`.
- **Composite Evidence Indices & Recovery**: Computes Physiological Activation Evidence Index (`activation_score`), Cardiorespiratory Regulation & Coupling Index (`regulation_score`), and engineered cardiac recovery evidence (`recovery_evidence`) via `RecoveryConfig` ($w_{\text{var}} \ge 0$, $w_{\text{hr}} \ge 0$, $w_{\text{var}} + w_{\text{hr}} > 0$).
- **Multi-Tiered Evidence Confidence (`StateConfidence`)**: Quantifies completeness and quality in $[0.0, 1.0]$ using `ConfidenceWeights` without artificial zero-filling for missing modalities.

### 11. `lamina::rppg` — Remote Photoplethysmography Optical Pulse Substrate
- **Video & ROI Abstractions (`VideoFrame`, `VideoStream`, `Roi`, `StaticRoi`, `TrackedRoiSeries`)**: Enforces explicit 8-bit row-major RGB frame buffer layout, monotonic physical timestamps, and bounding box validation.
- **Physical Timestamp Windowing & Zero-Copy Indexing (`timestamp_range`)**: Slices optical signals into physical-time sliding windows (`RppgWindowConfig`) using $O(\log N)$ binary search range lookup, avoiding unnecessary vector allocations.
- **Window-Local Preprocessing & Classical Algorithms (`GreenChannel`, `CHROM`, `POS`)**: Applies window-local channel mean normalization and linear detrending before executing classical CHROM (de Haan & Jeanne, 2013) or POS (Wang et al., 2017) pulse projections.
- **First-Class Quality Assessment (`RppgQualitySummary`, `RppgSegmentQuality`)**: Evaluates multi-tiered segment quality (ROI sufficiency, motion displacement, illumination stability, spectral periodicity) and computes recording-wide valid duration coverage fraction ($\text{valid\_fraction} = \frac{\text{valid\_duration}}{\text{total\_duration}}$). Rejects unusable segments below `min_quality` without artificial zero-filling.
- **Gap-Aware Downstream PPG Integration (`RppgSignal`, `RppgSegment`)**: Output signals provide `.valid_segments(max_gap_sec)` to extract contiguous non-NaN `RppgSegment` slices paired with `.to_ndarray()` and `.resample_uniform(target_fs, max_gap_sec)` for direct integration into Lamina's existing `lamina::ppg` pulse processing pipeline (`ppg_clean`, `ppg_findpeaks`). See [`docs/rppg.md`](./docs/rppg.md).

> *Disclaimer: Current rPPG support provides a research signal-processing substrate. It is not clinically validated and should not be interpreted as a validated medical vital-sign measurement.*

---

## Testing & Benchmarks

Run the complete validation test suite (unit tests, NeuroKit2 golden reference datasets, multi-rate suites $32\text{--}1000\text{ Hz}$, invariants):
```bash
cargo test
```

Run formatting and linter checks:
```bash
cargo fmt --check
cargo clippy -- -D warnings
```

Run Criterion performance benchmarks:
```bash
cargo bench -- --test
```

---

## Non-Clinical Disclaimer

Lamina is a general-purpose scientific signal-processing library designed for research, data analysis, and physiological computing. **It is not a medical device, nor has it been cleared or approved by regulatory authorities (FDA, CE) for clinical diagnosis, treatment, or monitoring.**
