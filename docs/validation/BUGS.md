# Lamina Validation — Potential / Confirmed Bug Log

This file records candidate defects and unexpected behaviors discovered while
validating the **current** Lamina implementation against public datasets.

**Scope: validation findings only. No Lamina source modification is performed
in this task.** Another engineering team owns triage and fixes. Each entry
must contain enough evidence to reproduce the behavior.

Consolidated 2026-09-07 by the bug-triage reviewer from the four adapter-group
candidate lists (`validation/results/{ecg,ppg,autonomic,rppg}/bug-candidates.md`)
and the framework sharp-edge list (`validation/API-INVENTORY.md`). Every
`confirmed` entry below was independently re-reproduced by the reviewer on
synthetic inputs (no dataset downloads) using the `lamina_bridge` JSON bridge
or a read-only Rust probe; scripts and captured outputs are in
`validation/results/bugs-repro/`. Claims that could not be re-reproduced on
synthetic inputs were kept at `suspected`/`ambiguous` and say so explicitly.

## Status taxonomy

| Status | Meaning |
| ------ | ------- |
| `confirmed` | Reproduced against the current implementation with a minimal input; behavior contradicts the documented/API-contract expectation. |
| `suspected` | Evidence points to an implementation defect but reproduction is incomplete or confounded. |
| `dataset-issue` | Root cause traced to the dataset/adapter (parsing, units, channel selection, annotation semantics), not Lamina. |
| `ambiguous` | Unexpected result; evidence insufficient to classify as defect or dataset issue. |
| `limitation` | Expected/by-design limitation of the current implementation (documented here for visibility, not a defect). |

Severity: `Informational` / `Low` / `Medium` / `High` / `Critical`.

## Entry format

```markdown
## BUG-XXX — Short Description

### Status
Suspected / Confirmed / Dataset-issue / Ambiguous / Limitation

### Component
Lamina component/API involved.

### Dataset
Dataset and version (or synthetic fixture).

### Reproduction
Exact command or minimal reproduction procedure.

### Input
Signal characteristics, sampling rate, relevant recording/segment.

### Expected Behavior
What should reasonably occur.

### Actual Behavior
What Lamina produced.

### Evidence
Metrics, output, plots, logs, or references.

### Severity
Informational / Low / Medium / High / Critical

### Suggested Investigation
Optional technical hypothesis about the cause.

### Scope
Validation finding only. No source modification performed.
```

## Cross-reference to original group IDs

| This log | Original ID | Source file | Status | Severity |
| -------- | ----------- | ----------- | ------ | -------- |
| BUG-001 | BUG-ECG-001 | results/ecg | confirmed | Low |
| BUG-002 | BUG-ECG-002 | results/ecg | confirmed | Medium |
| BUG-003 | BUG-A03 | results/autonomic | confirmed | Medium |
| BUG-004 | API-INVENTORY sharp edge #2 | framework review | confirmed | Medium |
| BUG-005 | BUG-R01 | results/rppg | confirmed | Medium |
| BUG-006 | API-INVENTORY sharp edge #4 | framework review | confirmed | Low |
| BUG-007 | API-INVENTORY sharp edge #3 | framework review | confirmed | Low |
| BUG-008 | API-INVENTORY sharp edge #5 | framework review | confirmed | Low |
| BUG-009 | API-INVENTORY sharp edge #6 | framework review | confirmed (test infra) | Low |
| BUG-010 | API-INVENTORY sharp edge #8 | framework review | confirmed (docs) | Informational |
| BUG-011 | BUG-ECG-003 | results/ecg | suspected | Medium |
| BUG-012 | BUG-PPG-001 | results/ppg | suspected | Medium |
| BUG-013 | BUG-ECG-005 | results/ecg | ambiguous | Low |
| BUG-014 | BUG-A02 | results/autonomic | ambiguous | Low |
| BUG-015 | BUG-A04 | results/autonomic | ambiguous | Low |
| BUG-016 | BUG-PPG-002 | results/ppg | limitation | Medium |
| BUG-017 | BUG-PPG-003 | results/ppg | limitation | Low |
| BUG-018 | BUG-A01 | results/autonomic | limitation | Informational |
| BUG-019 | API-INVENTORY sharp edge #9 | framework review | suspected (latent) | Low |
| BUG-020 | BUG-PPG-004 | results/ppg | dataset-issue | Informational |
| (closed) | BUG-R02 | results/rppg | fixed-elsewhere | — |
| (closed) | BUG-ECG-006 | results/ecg | dataset-issue (run scope) | — |
| (out of scope) | API-INVENTORY sharp edge #7 | framework review | not verified / stale reference | — |

## Entries

## BUG-001 — `ecg_clean` silently ignores its `method` argument

### Status
Confirmed (independently re-reproduced 2026-09-07 on a synthetic signal).

### Component
Lamina `ecg_clean(signal, fs, method)` via bridge op `ecg-clean`
(config field `method`).

### Dataset
Synthetic fixture (30 s ECG-like signal @ 360 Hz); originally observed on
mit-bih-arrhythmia 1.0.0 record 100.

### Reproduction
```
python3 validation/results/bugs-repro/repro_ecg_001_method_ignored.py
```

### Input
Any finite ECG segment.

### Expected Behavior
Different `method` values select different cleaning pipelines (per the API
signature), and unknown method names should be rejected with an error.

### Actual Behavior
Output is byte-identical for `none`, `neurokit`, `pantompkins`, `biosppy`, a
bogus method string, and the no-method default call; no error is raised. The
signature names the parameter `_method: &str` and always applies a 0.5 Hz
high-pass, order 5.

### Evidence
`validation/results/bugs-repro/repro_ecg_001_method_ignored.txt` — max abs
diff 0.0 for all 15 method pairs; cleaning itself does modify the signal
(max abs diff raw vs cleaned 0.10), so the pipeline runs; only dispatch is
missing. Source: `src/ecg/clean.rs` signature per API-INVENTORY §ecg.

### Severity
Low (documented API footgun; the single implemented pipeline performs well:
mean mitdb F1 = 0.988).

### Suggested Investigation
Either implement method dispatch or remove/rename the parameter to avoid
implying configurability; reject unknown method strings with
`SignalError`-style validation.

### Scope
Validation finding only. No source modification performed.

---

## BUG-002 — `ecg-peaks` fails with "Input signal contains non-finite values"
whenever `threshold_multiplier >= 1.0`, on provably finite input

### Status
Confirmed (independently re-reproduced 2026-09-07 on a synthetic signal;
root cause located by source inspection).

### Component
Lamina `ecg_findpeaks_config` / `EcgPeakDetectionConfig.threshold_multiplier`
via bridge op `ecg-peaks`.

### Dataset
Synthetic fixture (30 s ECG-like signal @ 360 Hz, verified finite);
originally observed on mitdb/100 MLII and a synthetic sinusoid.

### Reproduction
```
python3 validation/results/bugs-repro/repro_ecg_002_threshold_multiplier.py
```

### Input
Any finite signal; boundary is exact: 0.9999 works, 1.0 and above fail.

### Expected Behavior
A config-range rejection should use a config-related error (e.g.
`InvalidWindowSize`-style or a dedicated variant/message); a value the API
accepts should not produce an input-data error.

### Actual Behavior
Every `threshold_multiplier >= 1.0` returns `lamina_error` with the message
"Input signal contains non-finite values (NaN or Infinity)", which is
factually wrong about the input. Root cause: `src/ecg/peaks.rs:110-112` maps
the `threshold_multiplier` range check (`tm <= 0.0 || tm >= 1.0`) onto
`SignalError::NonFiniteInput` — a config-range validation reusing the wrong
error variant.

### Evidence
`validation/results/bugs-repro/repro_ecg_002_threshold_multiplier.txt` —
sweep 0.1..0.9999 ok, 1.0/1.0000001/1.1/1.5/2.0/3.0 all fail with the
identical message; all other config fields work normally at tm=0.25.
`src/ecg/peaks.rs:110-112`.

### Severity
Medium — blocks legitimate threshold configurations and misleads users with a
wrong diagnosis; the default (0.25) is unaffected.

### Suggested Investigation
Return a config-validation error variant for out-of-range
`threshold_multiplier`, and document the accepted range `(0, 1)`.

### Scope
Validation finding only. No source modification performed.

---

## BUG-003 — `eda_clean` / `eda_peaks` hardcode a 5 Hz lowpass cutoff: any
fs ≤ 10 Hz fails (blocks all Empatica E4 EDA @ 4 Hz)

### Status
Confirmed (independently re-reproduced 2026-09-07 on a synthetic signal).

### Component
Lamina `eda_clean` (and therefore `eda_peaks`, which cleans internally).
`eda_decompose` does NOT fail at fs=4 Hz.

### Dataset
Synthetic fixture; originally observed on big-ideas 1.0.0 (EDA @4 Hz, 2/2
recordings failed) and wearable-exam-stress 1.0.0 (all EDA.csv @4 Hz).

### Reproduction
```
python3 validation/results/bugs-repro/repro_a03_eda_5hz_lowpass.py
```

### Input
Any EDA signal with sampling_rate ≤ 10 Hz (fs=4, 8, 10 all fail; fs=16 ok).
Empatica E4 wrist EDA at 4 Hz is the most common wearable EDA rate.

### Expected Behavior
A sampling-rate-parameterized cleaning routine should adapt its filter cutoff
to the Nyquist frequency (or document/enforce a minimum fs gracefully), not
unconditionally fail.

### Actual Behavior
`lamina_error`: "Invalid cutoff frequency: Cutoff frequency (5) must be
strictly less than Nyquist frequency (2)" at fs=4 Hz; same failure at fs=8
and fs=10 (Nyquist == 5, strict inequality). `eda_decompose` at fs=4 succeeds,
isolating the failure to `eda_clean`'s filter spec.

### Evidence
`validation/results/bugs-repro/repro_a03_eda_5hz_lowpass.txt` — fs sweep.
Group evidence: `validation/results/autonomic/big-ideas/recordings.csv` (2/2
failed with the cutoff error).

### Severity
Medium (blocks wrist-EDA validation on the most common wearable EDA sampling
rate; the error is explicit, so no silent wrong results).

### Suggested Investigation
Locate the hardcoded 5 Hz cutoff in `eda_clean`; either clamp to
`min(5, 0.8 * fs/2)` or return a documented error stating the minimum
supported fs.

### Scope
Validation finding only. No source modification performed.

---

## BUG-004 — `eda_findpeaks(phasic)` and `rsp_findpeaks(cleaned)` hardcode
fs = 100 Hz: silently wrong peak gating at any other sampling rate

### Status
Confirmed (independently re-reproduced 2026-09-07 with a read-only Rust probe
against the compiled crate; behavior also visible in source).

### Component
Lamina `eda::eda_findpeaks` (`src/eda/peaks.rs:234-236` calls
`eda_findpeaks_mask(phasic, 100.0, ...)`) and `rsp::rsp_findpeaks`
(`src/rsp/peaks.rs:336-338` calls `rsp_findpeaks_mask(cleaned, 100.0, ...)`).

### Dataset
Synthetic fixtures.

### Reproduction
Rust probe (source archived at
`validation/results/bugs-repro/repro_fs100_hardcoded_probe.rs`; crate lives
outside the repo with a path dependency, builds without modifying Lamina):
```
cargo run --release --manifest-path <probe>/Cargo.toml
```
Captured output: `validation/results/bugs-repro/repro_fs100_hardcoded.txt`.

### Input
(a) Phasic EDA at true fs=250 Hz, 10 SCR pulses 0.6 s apart (150 samples;
default `min_distance_sec=1.0`). (b) Cleaned respiration at true fs=25 Hz,
25 breaths 2.0 s apart (50 samples; default `min_breath_interval_sec=1.2`).

### Expected Behavior
Convenience wrappers either take fs as a parameter, or their second-based
gating behaves consistently with the `_config(fs, ...)` variants.

### Actual Behavior
- `eda_findpeaks` assumes 100 Hz → min distance 100 samples instead of 250 →
  keeps all 10 pulses where `eda_findpeaks_config(fs=250)` keeps 5
  (over-detection / wrong gating).
- `rsp_findpeaks` assumes 100 Hz → min breath interval 120 samples instead of
  30 → keeps 1 of 25 breaths where `rsp_findpeaks_config(fs=25)` keeps all 25
  (catastrophic under-detection at typical 25-50 Hz respiration rates).

The rustdoc does say "assuming default 100 Hz sampling rate", so the behavior
is documented at the API level; the defect is that the most discoverable
entry point silently produces wrong results at any other fs, with no error.

### Evidence
`validation/results/bugs-repro/repro_fs100_hardcoded.txt` (10 vs 5; 1 vs 25);
source citations above.

### Severity
Medium (silent wrong results for the RSP convenience function at common
respiration sampling rates; no error raised).

### Suggested Investigation
Deprecate the fs-less wrappers, or add `fs` parameters, or make the wrappers
route through the `_config` variants with an explicit fs.

### Scope
Validation finding only. No source modification performed.

---

## BUG-005 — Inconsistent output sign convention across rppg algorithms
silently degrades `ppg-peaks` HR for `green`/`pos`

### Status
Confirmed (sign-convention inconsistency independently re-reproduced
2026-09-07 on a synthetic RGB oscillation probe; magnitude of downstream harm
on realistic data supported by the rPPG group's SCAMPS evidence).

### Component
`rppg` — `GreenChannel`/`Pos` `extract_window` vs `Chrom::extract_window`
(bridge op `rppg-algorithm`), in composition with `ppg_findpeaks_config`
(Elgendi 2013, morphology-sensitive).

### Dataset
Synthetic fixture (30 fps, 20 s, GT HR 75 bpm; camera intensity modeled
anti-phase with blood volume, per the physics of reflectance PPG) and scamps
example set (10 synthetic videos with ground-truth `d_ppg`).

### Reproduction
```
python3 validation/results/bugs-repro/repro_r01_rppg_sign.py
```

### Input
Per-frame skin RGB means, uniform 30 fps.

### Expected Behavior
All three algorithms emit waveforms in a consistent phase convention (or the
difference is documented), so that feeding any of them into `ppg-peaks` — the
documented downstream path in API-INVENTORY §rppg — yields comparable HR
accuracy.

### Actual Behavior
- Synthetic probe: `green` r = -0.985 and `pos` r = -0.983 vs GT blood-volume
  pulse (intensity phase); `chrom` r = +0.979 (blood-volume phase). The
  underlying traces are all excellent (|r| > 0.96); only the sign convention
  differs between algorithms. Same picture with an asymmetric
  (fast-rise/slow-decay) BVP morphology: -0.983 / -0.964 / +0.939.
- SCAMPS (group evidence): `green` raw r = -0.921 mean, `pos` -0.823,
  `chrom` +0.36 (positive on 7/10). Fed as-is into `ppg-peaks`, `green` peak
  F1 collapses to 0.245 and `pos` HR MAE is worst (16.2 bpm). Sign-flip probe
  (3 videos): HR MAE improves for green/pos when flipped (e.g. pos
  27.50/12.17/11.67 → 7.73/1.42/0.95) and gets worse for chrom
  (2.26/10.58/1.19 → 10.75/21.40/24.75), confirming a phase-convention
  difference rather than a generic detector asymmetry.
- On the reviewer's clean synthetic sinusoid the HR effect is small (Elgendi
  tolerates a symmetric waveform either way); the collapse requires realistic
  morphology/noise, as in the SCAMPS evidence.

### Evidence
`validation/results/bugs-repro/repro_r01_rppg_sign.txt`;
`validation/results/rppg/metrics.csv`, `metrics_extra.csv` (raw and
sign-corrected waveform correlations, sign-flip probe numbers).

### Severity
Medium (silent accuracy degradation on the documented downstream path; no
error is raised).

### Suggested Investigation
Decide a crate-wide phase convention (e.g. blood-volume/BVP phase) and
normalize `GreenChannel`/`Pos` outputs to it, or document per-algorithm sign
behavior and/or make `ppg-peaks` sign-robust. Physically, camera intensity
decreases as blood volume increases, so an intensity-phase output is not
"wrong" — the defect is the undocumented inconsistency between algorithms and
the sign-sensitive composition with Elgendi peak detection.

### Scope
Validation finding only. No source modification performed.

---

## BUG-006 — `sample_entropy` returns `Ok(+inf)` on zero template matches

### Status
Confirmed (independently re-reproduced 2026-09-07 via the bridge; also
visible in source).

### Component
Lamina `complexity::sample_entropy` (`src/complexity/entropy.rs:67-68`
returns `Ok(f64::INFINITY)` when `count_m == 0 || count_m1 == 0`).

### Dataset
Synthetic fixture (iid Gaussian noise, n=300, m=2, explicit small r).

### Reproduction
```
python3 validation/results/bugs-repro/repro_entropy_and_rsp_double_clean.py
```

### Input
Random noise with r = 1e-3, 1e-4, 1e-6 (finite, positive, legitimate
tolerances that happen to yield zero matches).

### Expected Behavior
Either a documented sentinel with clear semantics, or an error; `Ok(+inf)`
as a normal return value pushes an infinite float into downstream arithmetic
with no signal that the statistic is undefined for the input.

### Actual Behavior
The Rust function returns `Ok(f64::INFINITY)`; the bridge surfaces it as
`{"is_infinite": true, "sample_entropy": null}`. Related edges observed in
the same probe: the bridge convenience default `r = 0.2*std` collapses to 0
for a constant signal and errors ("Tolerance threshold r (0) must be > 0.0"),
and a linear ramp yields `sample_entropy = -0.0`.

### Evidence
`validation/results/bugs-repro/repro_entropy_and_rsp_double_clean.txt`
(section a2); `src/complexity/entropy.rs:67-68`.

### Severity
Low (bridge consumers see an explicit `is_infinite` flag; direct Rust callers
get an unannounced inf).

### Suggested Investigation
Document the +inf return in the function contract, or return a dedicated
error/NaN with documentation.

### Scope
Validation finding only. No source modification performed.

---

## BUG-007 — `rsp_cycles_config` re-cleans its input internally (double
filtering when the caller pre-cleans)

### Status
Confirmed (behavioral; independently re-reproduced 2026-09-07 via the bridge;
also visible in source).

### Component
Lamina `rsp::rsp_cycles_config` (`src/rsp/peaks.rs:173` unconditionally calls
`rsp_clean_config` on the input).

### Dataset
Synthetic fixture (0.25 Hz respiration + drift + noise @ 25 Hz).

### Reproduction
```
python3 validation/results/bugs-repro/repro_entropy_and_rsp_double_clean.py
```

### Input
Raw and pre-cleaned versions of the same 180 s respiration signal.

### Expected Behavior
Either the API documents that `rsp_cycles_config` always cleans internally
(and callers must not pre-clean), or it accepts pre-cleaned input without
re-filtering.

### Actual Behavior
`rsp_cycles(rsp_clean(x))` differs from `rsp_cycles(x)` (cycle-level shifts,
e.g. first inspiration index 25 vs 24 in the probe), showing the internal
cleaning pass is applied again to already-cleaned input. The effect is small
for in-band signals (44 cycles, median rate 15.00 brpm in both arms) because
the default bandpass is close to idempotent in-band, but band-edge
attenuation compounds and results are not identical.

### Evidence
`validation/results/bugs-repro/repro_entropy_and_rsp_double_clean.txt`
(section b); `src/rsp/peaks.rs:168-173`.

### Severity
Low (behavioral; outputs differ but the practical impact on in-band synthetic
data is negligible).

### Suggested Investigation
Document the re-cleaning on `rsp_cycles_config`/`rsp_cycles`, or add a
`precleaned: bool`-style escape hatch.

### Scope
Validation finding only. No source modification performed.

---

## BUG-008 — `FeatureQuality.total_feature_count` hardcoded to 30, but only
28 features are countable

### Status
Confirmed (by source inspection; counting is unconditional, no loops).

### Component
Lamina `features::quality` — `src/features/quality.rs:138`
(`let total_features = 30;`) vs 28 flat `usable_count += 1` increments in the
same function.

### Dataset
N/A (code-level).

### Reproduction
```
grep -n "total_features = 30" src/features/quality.rs
grep -c "usable_count += 1" src/features/quality.rs   # -> 28
```

### Input
Any fully-populated feature vector.

### Expected Behavior
`usable_feature_count / total_feature_count` should be able to reach 1.0 when
every computed feature is present.

### Actual Behavior
With 28 countable increments and a hardcoded denominator of 30, the
usable-feature ratio is capped at 28/30 ≈ 0.933 even for a complete feature
vector, and silently drifts further if the feature set changes without
updating the constant.

### Evidence
`src/features/quality.rs:138-237` (28 increments between the `let mut
usable_count = 0;` and the struct construction at line 237).

### Severity
Low (systematically understates a quality ratio; no crash).

### Suggested Investigation
Compute the denominator from the actual feature list, or add a unit test
asserting `total_feature_count` equals the number of counted features.

### Scope
Validation finding only. No source modification performed.

---

## BUG-009 — `filter_parity_tests.rs` / `peaks_parity_tests.rs` panic on a
fresh checkout (golden JSONs not checked in)

### Status
Confirmed (test infrastructure; functionally re-run 2026-09-07).

### Component
`tests/filter_parity_tests.rs:317,371` and `tests/peaks_parity_tests.rs:26`
open `tests/golden_filter.json` / `tests/golden_peaks.json` with `.expect()`;
only `tests/golden_ecg.json` is checked in.

### Dataset
N/A.

### Reproduction
```
cargo test --test filter_parity_tests --test peaks_parity_tests   # fresh checkout
```

### Input
N/A (test infrastructure). Trigger is a fresh checkout missing
`tests/golden_filter.json` / `tests/golden_peaks.json`; no signal input.

### Expected Behavior
Parity tests either skip gracefully with instructions to run the
`generate_*_reference.py` scripts, or the goldens are checked in.

### Actual Behavior
3 tests panic with "Failed to open golden filter/peaks JSON: No such file or
directory" (filter group C, filter group D, peaks parity); the other tests
pass. Pre-existing condition, noted in API-INVENTORY baseline test status.

### Evidence
`validation/results/bugs-repro/repro_parity_tests_missing_goldens.txt`
(captured cargo output, 2026-09-07).

### Severity
Low (test infra friction; `cargo test` is red on a fresh clone).

### Suggested Investigation
Generate the goldens in a build/setup step, gate the parity tests behind a
feature/`ignore` attribute with documentation, or check the goldens in.

### Scope
Validation finding only. No source modification performed.

---

## BUG-010 — `multimodal_quality` rustdoc names the wrong error variant

### Status
Confirmed (documentation/code mismatch; by source inspection).

### Component
`src/multimodal/quality.rs:138` documents "Returns
[`SignalError::InvalidSamplingRate`] if input bounds are invalid", but
line 161 returns `SignalError::EmptySignal` when all four modality arguments
are `None`.

### Dataset
N/A.

### Reproduction
Source inspection (citations above).

### Input
N/A (documentation/code mismatch). The relevant call is
`multimodal_quality(None, None, None, None, ...)`, i.e. all four modality
arguments `None`.

### Expected Behavior
Docs should name the error actually returned (`EmptySignal` on all-None
input).

### Actual Behavior
Docs name `InvalidSamplingRate`, which this function never returns.

### Evidence
`src/multimodal/quality.rs:135-161`.

### Severity
Informational.

### Suggested Investigation
Fix the rustdoc.

### Scope
Validation finding only. No source modification performed.

---

## BUG-011 — Default `ecg-peaks` under-detects low-amplitude normal beats in
the presence of tall PVCs (mitdb/228 recall 0.60)

### Status
Suspected (dataset evidence is strong, but the reviewer's synthetic
reproduction attempts did NOT reproduce the failure — see Evidence).

### Component
Lamina `ecg_findpeaks` adaptive threshold (default
`EcgPeakDetectionConfig`: lowcut 5 / highcut 15 Hz, integration window
0.150 s, refractory 0.200 s, searchback on, threshold_multiplier 0.25).

### Dataset
mit-bih-arrhythmia 1.0.0, record 228 (MLII @ 360 Hz; multiform PVCs,
first-degree AV block per header comments).

### Reproduction
```bash
python -m validation run --dataset mit-bih-arrhythmia --recordings 228 \
    --seed 42 --results-dir validation/results/ecg
```
Synthetic probe (non-reproducing):
```
python3 validation/results/bugs-repro/repro_ecg_003_amplitude_disparity.py
```

### Input
Record 228 MLII: normal beats with R amplitude ~0.7 mV interleaved with PVCs
at ~2.2 mV.

### Expected Behavior
Recall broadly comparable to other mitdb records (dataset median per-record
recall = 0.999); published Pan-Tompkins-class detectors achieve > 0.95 recall
on this record.

### Actual Behavior
recall 0.602, precision 0.993, F1 0.749. Missed beats cluster where small
normal beats follow tall PVCs: in a 20 s window at minute 10, 26 annotated
beats but only 6 detections — all 6 on the tall PVCs. With
`threshold_multiplier: 0.1`, recall rises to 0.999 (F1 0.990), so the peaks
are detectable by the same pipeline at a lower threshold.

### Evidence
`validation/results/ecg/mit-bih-arrhythmia/metrics.csv` (record 228 row);
plot `validation/results/ecg/mit-bih-arrhythmia/record228_minute10.png`;
818 of 2053 beats missed, concentrated in minutes 9–16 and 27–28.
Reviewer non-reproduction: on idealized synthetic signals with the same 3:1
amplitude disparity (both interleaved every-4th-beat PVCs and a 15 s run of
consecutive PVCs), default `ecg_peaks` achieves recall = 1.000
(`validation/results/bugs-repro/repro_ecg_003_amplitude_disparity.txt`). The
failure therefore appears to depend on real-record properties not captured by
the probe (multiform PVC morphology, ectopic timing/RR irregularity, noise);
the dataset evidence stands but the minimal triggering condition is unknown.

### Severity
Medium — affects recordings with large QRS amplitude disparity (e.g.
multiform ectopy); results remain excellent on typical records.

### Suggested Investigation
Check the learning/adaptation of the signal/noise level estimates in
`ecg/peaks.rs` (SPKI/NPKI initialized from the 75th/25th percentiles of
candidate heights, `src/ecg/peaks.rs:213-222`): after runs of large-amplitude
beats the signal-level estimate may dominate the threshold. Characterize
which real-record property (morphology variability, RR irregularity, noise)
is required to trigger the failure; the idealized amplitude disparity alone
is not sufficient.

### Scope
Validation finding only. No source modification performed.

---

## BUG-012 — `ecg-peaks` adaptive threshold under-detects on clean
narrow-biphasic QRS morphology with a ~90 s blackout and no searchback
recovery (wrist s6_low_resistance_bike)

### Status
Suspected (dataset evidence is detailed, but the reviewer's synthetic
narrow-biphasic probe did NOT reproduce the failure — see Evidence).

### Component
`lamina::ecg::ecg_findpeaks_config` (Pan-Tompkins threshold/searchback
logic), bridge op `ecg-peaks` with default config.

### Dataset
Wrist PPG During Exercise v1.0.0, record `s6_low_resistance_bike`
(chest_ecg @ 256 Hz), PhysioNet `pn_dir='wrist'`.

### Reproduction
```
python -m validation run --dataset wrist-ppg-exercise --seed 42 \
    --results-dir validation/results/ppg
```
Synthetic probe (non-reproducing): see
`validation/results/bugs-repro/repro_ppg_001_biphasic_probe.txt`.

### Input
280 s chest-strap ECG @ 256 Hz. QRS = narrow biphasic spike (~+180/-260 ADC
units), beat every ~0.52 s (~115 bpm), visually clean; rolling 10 s
peak-to-peak amplitude steady (497-740 units). `.atr` annotations: 536 beats.

### Expected Behavior
A Pan-Tompkins-style detector should not miss >90% of beats for 30+ s on a
clean, steady-amplitude ECG, and searchback should recover the threshold
within seconds.

### Actual Behavior
Per-30-s detection counts `[66, 2, 0, 3, 54, 71, 67, 105, 97]` vs annotation
counts `[52, 57, 58, 59, 61, 62, 61, 62, 64]` — a ~90 s blackout (30-120 s)
followed by ~1.6x over-detection (240-280 s). Window 30-60 s in isolation
yields 3/54 beats (default) vs 68 with `threshold_multiplier=0.1`;
`integration_window_sec=0.08` yields 6. The band-passed front-end output is
fine (scipy finds all lobes); the failure is in thresholding/decision.
Annotations were verified to align (ECG F1 0.88-1.00 on 14 of 19 records).

### Evidence
Group metrics in `validation/results/ppg/metrics.csv` /
`metrics_extra.csv` and the per-window analysis quoted above. Reviewer
non-reproduction: an idealized synthetic narrow-biphasic train (+180/-260,
~16 ms width, 115 bpm @ 256 Hz) is detected perfectly with default config
(F1 1.000). The blackout therefore depends on real-record properties beyond
biphasic shape alone.

### Severity
Medium (the reporting group assessed Medium-High for continuous-monitoring
use: silent, large HR underestimation followed by over-estimation).

### Suggested Investigation
Instrument threshold/searchback state over the 30-120 s window of
s6_low_resistance_bike; identify which signal property (residual motion
modulation, amplitude drift within the window, T-wave content) inflates the
threshold estimate, given that the idealized biphasic morphology alone does
not trigger it. Cross-check BUG-016 compounding on motion-heavy records.

### Scope
Validation finding only. No source modification performed.

---

## BUG-013 — Paced-beat under-detection on mitdb/104 is strongly
channel-dependent (V5 F1 0.68 vs V2 F1 0.98)

### Status
Ambiguous

### Component
Lamina `ecg_findpeaks` default pipeline on paced/fusion-beat morphology.

### Dataset
mit-bih-arrhythmia 1.0.0, record 104 (no MLII; channels V5, V2; paced rhythm
with many pacemaker fusion beats per header comments).

### Reproduction
`bridge.ecg_peaks` on each channel of record 104, matched against
`wfdb.rdann('104', 'atr')` at 150 ms tolerance.

### Input
Record 104, both channels @ 360 Hz.

### Expected Behavior
Reasonable beat recall on either ECG lead.

### Actual Behavior
V5: precision 0.998, recall 0.520, F1 0.683 (2229 reference beats, 1160
detected). V2: precision 0.970, recall 0.995, F1 0.982. The adapter's
documented fallback (MLII absent → first channel) evaluates V5, so the
reported dataset metric is the V5 number. Unclear how much of the gap is a
Lamina morphology weakness vs a property of the V5 lead for this patient.

### Evidence
Per-channel metric comparison (ECG group, 2026-09-06);
`validation/results/ecg/bug-candidates.md` BUG-ECG-005.

### Severity
Low

### Suggested Investigation
Inspect V5 segments around missed paced beats; compare published detectors on
the same lead. Consider adapter-side selection of the better-quality lead
when MLII is absent.

### Scope
Validation finding only. No source modification performed.

---

## BUG-014 — WESAD S3 chest-EDA: baseline window shows 147 SCRs/4 min
(implausibly high)

### Status
Ambiguous (insufficient evidence to classify as Lamina defect vs dataset
characteristic)

### Component
Bridge op `eda-peaks` (`eda_clean` -> `eda_decompose` ->
`eda_findpeaks_events`) at fs=700 Hz.

### Dataset
WESAD 1.0, subject S3, baseline condition window.

### Reproduction
```
python -m validation run --dataset wesad --recordings S3_baseline \
    --results-dir <dir>
```

### Input
S3 chest EDA @700 Hz, 4-min middle window of the longest baseline segment.

### Expected Behavior
Baseline (resting) condition should show a low SCR rate (typically < ~5/min),
lower than the stress condition.

### Actual Behavior
S3 baseline: 147 SCRs in 4 min (36.8/min) vs S3 stress: 39 in 4 min
(9.8/min). For the other 4 subjects baseline SCR rate is 0-10.5/min and
stress is higher (group medians: baseline 0.5/min, stress 8.5/min — expected
direction holds). S3's baseline chest EDA may contain motion/squeeze
artifacts, or `eda_findpeaks_events` may over-segment slow drifts at 700 Hz
(above the ~4 Hz wrist-EDA design point).

### Evidence
`validation/results/autonomic/metrics_extra.csv` rows
`wesad,S3_baseline,condition_scr_rate_per_min,36.75` vs
`wesad,S3_stress,...,9.75`.

### Severity
Low

### Suggested Investigation
Inspect S3 chest-EDA raw trace in the baseline window; compare
`eda_findpeaks_events` on the native 700 Hz window vs a downsampled (e.g.
10 Hz) version to test for a rate-dependent threshold/rise-time interaction.

### Scope
Validation finding only. No source modification performed.

---

## BUG-015 — Large Lamina-vs-E4 wrist-BVP heart-rate gap (MAE 26-39 bpm) even
on signal-active segments

### Status
Ambiguous (evidence insufficient to classify as Lamina defect vs E4 reference
error vs dataset quality)

### Component
Lamina `ppg_clean` -> `ppg_findpeaks_config` (bridge op `ppg-peaks`) at
fs=64 Hz, vs Empatica E4 proprietary HR.csv reference.

### Dataset
wearable-exam-stress 1.0.0, S1..S5 x {midterm_1, midterm_2, Final}.

### Reproduction
```
python -m validation run --dataset wearable-exam-stress --results-dir <dir>
python validation/results/autonomic/extra_analysis.py exam
```

### Input
E4 BVP @64 Hz, sessions 1.5-7 h; per-second-std>1 active fraction 0.17-0.35
for midterm_1 vs 0.78-1.00 for midterm_2/Final.

### Expected Behavior
Two HR estimates from the same wrist device should roughly agree when the BVP
signal is valid.

### Actual Behavior
Raw aligned HR MAE vs E4 HR.csv: 26.4-47.1 bpm (median 35.2); on active
segments still 26.4-38.7 bpm. Counter-evidence for a pure Lamina defect:
(1) E4 HR.csv contains implausible values (150-170 bpm during seated exams);
(2) where E4 emits confident IBI values, Lamina BVP inter-beat intervals
agree with E4 IBI.csv within 28.9-45.4 ms; (3) WESAD cross-device check
(Lamina chest-ECG HR vs Lamina wrist-BVP HR) shows MAE 5.8-22.4 bpm.

### Evidence
`validation/results/autonomic/metrics.csv` (`hr_mae`) and
`metrics_extra.csv` (`hr_mae_active_segments_bpm`, `ibi_aligned_mae_ms`,
`bvp_active_fraction`).

### Severity
Low

### Suggested Investigation
Benchmark `ppg-peaks` on a dataset with beat-level ground truth before
attributing the gap to Lamina; quantify E4 HR.csv error against ECG-derived
HR in a dataset that has both.

### Scope
Validation finding only. No source modification performed.

---

## BUG-016 — `ecg-peaks` / `ppg-peaks` over-count beats under periodic motion
artifact (foot-strike spikes), inflating HR

### Status
Limitation (expected/by-design; recorded for visibility)

### Component
`lamina::ecg::ecg_findpeaks_config`, `lamina::ppg::ppg_findpeaks_config`
(no motion-artifact rejection).

### Dataset
Wrist PPG During Exercise v1.0.0; clearest on `s1_walk` (chest_ecg and
wrist_ppg @ 256 Hz).

### Reproduction
```
python -m validation run --dataset wrist-ppg-exercise --seed 42 \
    --results-dir validation/results/ppg
```

### Input
Exercise recordings with periodic foot-strike spikes (~0.55 s spacing)
interleaved with true beats (~1.05 s spacing).

### Expected Behavior
No defect claimed: Lamina implements classical detectors without
accelerometer-guided artifact rejection, and this dataset is explicitly
designed to stress exactly that; errors are expected to grow with motion
intensity.

### Actual Behavior
`s1_walk`: ecg-peaks F1 = 0.667 (973 detected vs 563 annotated), windowed
HR MAE 63.6 bpm; ppg-peaks HR bias +33.0 bpm. HR-from-PPG MAE stratified by
activity: walk 15.9 bpm (n=6), run 32.9 bpm (n=5), low-resistance bike
14.7 bpm (n=5), high-resistance bike 20.3 bpm (n=3) — errors grow with
motion intensity as expected. Note: per-record ECG F1 < 0.75 on 5 of 19
records may partially compound BUG-012 (threshold pathology); the entries
are not fully independent.

### Evidence
`validation/results/ppg/metrics.csv`, `metrics_extra.csv`.

### Severity
Medium (for exercise-HR use)

### Suggested Investigation
None required for correctness; motion-artifact rejection would be a feature.

### Scope
Validation finding only. No source modification performed.

---

## BUG-017 — `rsp-cycles` over-detects breaths at very low respiratory rates
with biphasic inspiratory morphology; default `max_breath_interval_sec=12`
rejects true cycles

### Status
Limitation (borderline ambiguous; see notes)

### Component
`lamina::rsp::rsp_cycles_config` (peak-picking cycle detector; no periodicity
screening).

### Dataset
BIDMC PPG and Respiration Dataset v1.0.0, record `bidmc05` (RESP @ 125 Hz).

### Reproduction
```
python -m validation run --dataset bidmc --seed 42 \
    --results-dir validation/results/ppg
```

### Input
ICU respiration at ~6 brpm with annotated intervals 9.7-14.3 s and a small
pre-inspiratory bump ~4-5 s before each true inspiratory peak.

### Expected Behavior
Mostly an expected limitation of a simple peak-based cycle detector at ~6
brpm with biphasic morphology; ideally only true inspiratory peaks are
counted and true cycles with long intervals are not structurally rejected.

### Actual Behavior
`bidmc05`: F1 = 0.658 (precision 0.490, recall 1.000); Lamina counts both
the pre-inspiratory bump and the true peak (98 cycles vs 48 annotated).
Config relaxation (`max_breath_interval_sec=20`, `min_amplitude=0.15`)
still yields 95 detections (F1 0.671) — not fixable via exposed config.
Separately, annotated intervals up to 14.3 s exceed the default
`max_breath_interval_sec=12`, so some true cycles are structurally rejected
by the default ceiling (masked here by dense false detections). All other
11 BIDMC records: F1 0.93-0.99 (mean 0.944).

### Evidence
`validation/results/ppg/metrics.csv`; group plots.

### Severity
Low

### Suggested Investigation
Document the default 12 s/breath ceiling for low-RR populations; consider
periodicity screening as a feature.

### Scope
Validation finding only. No source modification performed.

---

## BUG-018 — `hrv` op computes RMSSD over all detected intervals; no
ectopy/artifact filtering (confounds age trend)

### Status
Limitation (expected/by-design; documented for visibility)

### Component
Lamina `hrv_rmssd` / `hrv_mean_nn` via bridge op `hrv` (upstream:
`ecg_findpeaks_config` at fs=1000 Hz).

### Dataset
Autonomic Aging 1.0.0, records 0276, 0554, 0637 (age stratum 70+).

### Reproduction
```
python validation/results/autonomic/extra_analysis.py aging
```

### Input
Record 0554 ECG1, first 10 min @1000 Hz: sustained ventricular bigeminy;
Lamina `ecg-peaks` correctly marks every QRS (verified visually); RR sequence
alternates ~0.44 s / ~0.83 s.

### Expected Behavior
Published physiology expects RMSSD to decrease with age.

### Actual Behavior
Lamina RMSSD medians: young 52.3 ms, middle 24.9 ms, old 142.8 ms — the old
stratum is inflated by genuine bigeminy/ectopy (record 0554 RMSSD 315.6 ms).
This is a population-trend check confounded by arrhythmia in the sample, not
a detection error; the HRV op documents no ectopy filtering.

### Evidence
`validation/results/autonomic/metrics_extra.csv` rows
`autonomic-aging,0554,rmssd_ms,315.59`.

### Severity
Informational

### Suggested Investigation
Consumers of `hrv` on elderly/clinical recordings should pre-filter ectopic
intervals; a documented normal-to-normal-interval option could be a feature
request, not a bug fix.

### Scope
Validation finding only. No source modification performed.

---

## BUG-019 — `ecg/peaks.rs` `partial_cmp().unwrap()` is panic-capable on NaN
in integrated signal (latent)

### Status
Suspected (latent; no reproduction — inputs are pre-validated finite and no
NaN-producing path has been observed).

### Component
`src/ecg/peaks.rs:218` —
`candidate_heights.sort_by(|a, b| a.partial_cmp(b).unwrap())`.

### Dataset
N/A (code-level).

### Reproduction
None known. The input signal is validated finite before this point
(`src/ecg/peaks.rs:144`); a panic would require the filter/integration stages
to introduce NaN from finite input (not observed in any validation run).

### Input
`candidate_heights` values of the integrated ECG signal at
`src/ecg/peaks.rs:218`; inputs are pre-validated finite, so triggering input
(NaN in the integrated signal from finite raw input) is hypothetical.

### Expected Behavior
Total-order-aware sort (e.g. `total_cmp`) so no internal numeric edge can
abort the process.

### Actual Behavior
A single NaN in the integrated signal would panic via `unwrap()`. The bridge
contains panics per-op (`catch_unwind`), but direct Rust callers would abort.

### Evidence
Source citation above; no runtime occurrence.

### Severity
Low

### Suggested Investigation
Replace with `total_cmp` (Rust >= 1.62) or filter NaN defensively.

### Scope
Validation finding only. No source modification performed.

---

## BUG-020 — bidmc03 ECG lead II near-flatline: `ecg-peaks` finds 153 beats
vs 610 PPG beats

### Status
Dataset-issue (signal quality); behavioral observation recorded

### Component
`lamina::ecg::ecg_findpeaks_config` under extreme low-amplitude input.

### Dataset
BIDMC `bidmc03`, ECG lead II @ 125 Hz.

### Reproduction
```
python -m validation run --dataset bidmc --seed 42 \
    --results-dir validation/results/ppg
```

### Input
Lead II std = 0.053 mV, p2p = 1.26 mV (vs healthy lead II QRS ~1 mV): the
electrode signal is near-flatline/low-voltage.

### Expected Behavior
Root cause is the recorded signal, not the detector; on a near-flatline
lead a detector ideally reports few/no beats rather than an implausible
rate.

### Actual Behavior
Lamina ecg-peaks finds 153 peaks in 8 min (~19/min, implausible) while
ppg-peaks on the same recording finds 610 (~76/min, plausible). Kept as a
structural anomaly flag; no fix proposed.

### Evidence
`validation/results/ppg/bug-candidates.md` BUG-PPG-004; group metrics.

### Severity
Informational

### Suggested Investigation
None.

### Scope
Validation finding only. No source modification performed.

---

## Not bugs / closed

### BUG-R02 (closed — fixed-elsewhere): setup-env.sh bridge binary filename
The rPPG group reported that `validation/scripts/setup-env.sh` copied
`$HOME/.lamina-bridge-target/release/lamina-bridge` (hyphen) while cargo
names the binary `lamina_bridge` (underscore), so the copy silently no-ops.
Verified 2026-09-07 against master: line 31 now copies
`$HOME/.lamina_bridge-target/release/lamina_bridge`, matching
`[[bin]] name = "lamina_bridge"` in `validation/lamina_bridge/Cargo.toml`,
and a fresh `setup-env.sh` run leaves the binary at
`validation/lamina_bridge/target/release/lamina_bridge`, where
`validation/validation/bridge.py` looks (all repro scripts in
`validation/results/bugs-repro/` ran against it). Fixed in master; no action.

### BUG-ECG-006 (closed — validation-run scope, not a Lamina defect):
nstdb partial coverage
The nstdb evaluation covered 7 of 12 records because physionet.org returned
HTTP 403 from the validation host and the static mirror throttled downloads;
the SNR gradient for base record 118 is fully covered. Downloading
119e06/12/18 in the background (~6 min at observed throttle) would complete
coverage if required. No Lamina implication.

### API-INVENTORY sharp edge #7 (out of scope — stale reference):
`lamina_dart` `simple.rs`
The sharp-edge list cited `lamina_dart/rust/src/api/simple.rs` as likely
uncompilable (`.into_raw_vec()` on a `Result`). On the current tree no
`simple.rs` exists under `lamina_dart/rust/src/api/`, and the only
`into_raw_vec*` usages (`eda.rs`, `rsp.rs`) call
`into_raw_vec_and_offset().0` on `Array1` values after `Ok(...)` wrapping,
which is well-formed. Not verified by compilation (flutter_rust_bridge
toolchain not installed; bindings are outside this suite's scope); the
specific cited defect appears stale.

### Investigated and NOT classified as defects (from group notes)
- BIDMC breath annotations (`.breath`, ann1/ann2) are 0-based sample indices
  at 125 Hz marking inspiratory peaks — verified empirically (median offset
  vs band-passed RESP maxima 0.0 s); inter-annotator agreement F1 0.98-0.99.
- Wrist-dataset short NaN gaps (e.g. 234 samples in s1_walk) are
  wrist-device dropouts; the adapter linearly interpolates them and records
  counts in metadata. The bridge correctly rejects non-finite JSON input
  (good fail-fast behavior).
- ECG @125 Hz (BIDMC) with the default 5-15 Hz detection bandpass: no
  anomaly beyond BUG-020.
