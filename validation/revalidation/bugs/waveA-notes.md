# Wave A — Bug-Matrix Revalidation: API-Level & Synthetic Bugs

**Repo/HEAD:** `$HOME/work` clone of `/mnt/agents/lamina-reval`, branch `reval/bugs`, base `caa7484` (Lamina `main @ fec2668` post-remediation).
**Mode:** validation-only. No Lamina source (`src/`, `tests/`, …) or existing evaluation code modified. Rust probes live in a scratch crate **outside** the repo (`$HOME/probe_crate`, path-dep on the lamina crate); only probe sources + captured outputs were copied into `validation/revalidation/bugs/evidence/`.
**Toolchain:** rustc/cargo 1.98.1 (rsproxy mirror); bridge `lamina_bridge v0.1.0` built release against post-remediation lamina; Python probes use `validation/validation/bridge.py`.

---

## BUG-001 — `ecg_clean` method dispatch

**Original Finding:** `ecg_clean(signal, fs, method)` silently ignored `method`: byte-identical output for `none`/`neurokit`/`pantompkins`/`biosppy`/bogus strings; no error for unknown methods (`_method` parameter, always 0.5 Hz HP order 5).

**Previous Evidence:** `validation/results/bugs-repro/repro_ecg_001_method_ignored.{py,txt}` (max abs diff 0.0 for all 15 pairs, bogus accepted). Stage-1 source check: dispatch now exists at `src/ecg/clean.rs:17-31`; all four accepted names map to the same pipeline; unsupported → `SignalError::InvalidCutoffFrequency`.

**Current Implementation:** `method.trim().to_lowercase()` normalized; `"" | "neurokit" | "pantompkins" | "biosppy"` → `signal_filter(signal, fs, Some(0.5), None, 5)`; anything else → `Err(InvalidCutoffFrequency("Unsupported ECG cleaning method: {method}"))`. Rustdoc now states the single methodology honestly. Bridge maps historical `"none"` → `""` (SPEC §A.1).

**Validation Performed:** Bridge probe (`bug001_probe.py`, 30 s synthetic ECG @ 360 Hz): outputs for `""`, `none`, `neurokit`, `pantompkins`, `biosppy` — all 10 pairwise max-abs-diffs = 0.0; pipeline demonstrably runs (raw-vs-cleaned diff 0.303). Unsupported strings `bogus`, `NEUROKIT2`, `pan-tompkins`, `hamilton` → `lamina_error` with clear message; `'  neurokit  '` and `'BioSPPy'` accepted (normalization). Original repro script rerun **unported** (bridge schema unchanged): bogus string now ERRORs. Note: the old script's final "CONFIRMED … (bogus string accepted, no error)" line is a stale conclusion string — it keys only on successful outputs; its own output above the line shows the rejection.

**Result:** **RESOLVED.** Both halves of the original expected behavior are met to the extent the approved remediation defined it: unknown methods are rejected with an error; the dispatch is real (though all accepted names share one pipeline). Same-pipeline-for-all-methods is **not** a contract violation: `docs/architecture/signal-contracts.md` §2.2 is silent on cleaning methods, and the rustdoc no longer implies distinct pipelines.

**Remaining Concern:** P3 — `SignalError::InvalidCutoffFrequency` is a semantically wrong variant for a method-string error (Display prefixes "Invalid cutoff frequency:" on the message). Four aliases for one numerically identical pipeline remains an API footgun, albeit now documented.

---

## BUG-002 — `threshold_multiplier` range validation

**Original Finding:** Any `threshold_multiplier >= 1.0` failed with "Input signal contains non-finite values" on provably finite input (range check mapped to `NonFiniteInput`).

**Previous Evidence:** `repro_ecg_002_threshold_multiplier.{py,txt}` (sweep; 1.0/1.1/1.5/2.0/3.0 all wrong-message failures). Stage-1: new check at `src/ecg/peaks.rs:116-120` returns `InvalidCutoffFrequency("Threshold multiplier must be strictly between 0.0 and 1.0")`.

**Current Implementation:** `EcgPeakDetectionConfig::validate()` rejects `tm` non-finite, ≤ 0, or ≥ 1 with the message above; called from `ecg_findpeaks_config` after input guards.

**Validation Performed:** Bridge sweep (`bug002_probe.py`) on verified-finite synthetic ECG: tm = 0.0, 1.0, 1.5, −0.5 → all rejected with the clear new message; 0.9999, 1e-12, 0.25, 0.5 → accepted (peak counts 29/87/79/70). NaN via bridge: rejected one layer up as `bad_request` (strict JSON cannot carry NaN); the Rust-level NaN/+inf path was therefore verified directly in the `bug019_nan` probe: `with_threshold_multiplier(NaN)` and `(INFINITY)` → same clear range error. Original repro rerun unported: boundary behavior identical (0.9999 ok, 1.0 fails) but with the corrected message; all other config fields unaffected at tm=0.25.

**Result:** **RESOLVED.** Error is now a truthful parameter-range message for every out-of-range class tested (zero, ≥1, negative, NaN, +inf).

**Remaining Concern:** P3 — variant reuse: `InvalidCutoffFrequency` for a threshold-multiplier range problem (no dedicated `InvalidParameter` variant exists in `src/error.rs`). Cosmetic; message text is unambiguous.

---

## BUG-004 — explicit `fs` in `eda_findpeaks` / `rsp_findpeaks`

**Original Finding:** Convenience wrappers hardcoded fs=100 Hz: `rsp_findpeaks` kept 1 of 25 breaths at true fs=25; `eda_findpeaks` kept 10 (vs config-correct 5) pulses at true fs=250. Silent wrong gating at any fs≠100.

**Previous Evidence:** `repro_fs100_hardcoded_probe.rs` + `repro_fs100_hardcoded.txt`. Stage-1: signatures changed to `(&Array1<f64>, sampling_rate: f64)` (breaking); fs validated `> 0` and finite → `InvalidSamplingRate` (`src/eda/peaks.rs:232-237`, `src/rsp/peaks.rs:347-352`); hardcoded 100.0 removed.

**Current Implementation:** Both wrappers validate fs then delegate to the `_mask` variants with default configs — identical code path to the `_config(fs, …)` entry points.

**Validation Performed:** (1) The archived probe — **already ported to the new API by the remediation commit** — was copied verbatim into the scratch crate; it compiles unmodified (itself proof the port is type-correct) and its claims verify: EDA fs=250 → `eda_findpeaks` 5 peaks == `_config` 5 peaks; RSP fs=25 → `rsp_findpeaks` 25 peaks == `_config` 25 peaks. (2) New probe (`bug004_fs_probe.rs`): same underlying synthetic signal sampled at fs=25 and fs=100 gives peak *times* consistent within half a sample at the coarser rate (RSP: 1.96/4.96/8.00… vs 1.98/4.98/7.99… s; EDA: 3.00/5.48/… vs 3.00/5.50/… s). (3) fs ∈ {0, −25, NaN, +inf} → `Err(InvalidSamplingRate)` with clear message on both functions. (4) Regression assertions: old failure modes gone (25 not 1; 5 not 10).

**Result:** **RESOLVED.**

**Remaining Concern:** None behavioral. Cosmetic: the archived probe file still carries a stale comment "rsp_findpeaks (hardcoded fs=100)" describing pre-remediation semantics. The fs-less signatures no longer exist, so silent misuse is impossible (compile error) — the strongest possible fix class.

---

## BUG-006 — `sample_entropy` `Ok(+inf)` contract

**Original Finding:** Zero template matches → `Ok(f64::INFINITY)` with no documentation; direct Rust callers get an unannounced inf. (Bridge already surfaced `is_infinite`.)

**Previous Evidence:** `repro_entropy_and_rsp_double_clean.txt` (§a2); `src/complexity/entropy.rs:67-68`. Stage-1: rustdoc added (`entropy.rs:11`); the r>0 validation at `:31-35` is **pre-existing**, not a remediation change.

**Current Implementation:** Rustdoc Scientific Contract now reads "Output: Non-negative Sample Entropy value h ≥ 0.0, **or `f64::INFINITY` if zero template matches occur**"; Errors section documents the r validation. Behavior unchanged.

**Validation Performed:** Bridge probe (`bug006_probe.py`): (a) noise n=300, m=2, r ∈ {1e-3, 1e-4, 1e-6} → `sample_entropy=null, is_infinite=true` (i.e. Rust `Ok(+inf)`); (b) r = 0.0, −0.1 → clear error "Tolerance threshold r (X) must be > 0.0 and finite"; (c) constant signal with bridge default r = 0.2·std = 0 → same clean error (documented edge); (d) perfectly periodic signal and (e) linear ramp → `-0.0` (≡ 0, mathematically `-ln(1)`), so "identical/fully-regular signals → 0" holds; (f) noise with default r → finite 2.282. Original repro reruns unported, consistent. Rustdoc evidence captured in `bug006_rustdoc.txt`.

**Result:** **RESOLVED.** The plan's action was "document the +inf contract; validate r > 0" — the contract is now documented and every behavioral clause was exercised.

**Remaining Concern:** P3 — `InvalidCutoffFrequency` variant reused for the tolerance error (same cross-cutting variant-reuse nit as BUG-001/002). Constant-signal + default-r still errors rather than returning a defined statistic; acceptable and explicit.

---

## BUG-008 — `FeatureQuality.total_feature_count` (28 vs 30)

**Original Finding:** Denominator hardcoded to 30 while only 28 `usable_count += 1` increments exist → ratio capped at 0.933 even for a complete feature vector.

**Previous Evidence:** grep/source count. Stage-1: value corrected 30→28 at `src/features/quality.rs:138` but **still a hardcoded literal** (plan claimed "dynamically calculated") — rated PARTIAL at source level.

**Current Implementation:** `let total_features = 28;` followed by exactly 28 flat `is_some()` increments (7 cardiac + 9 EDA + 6 respiration + 6 coupling Option fields), assigned at `:237`.

**Validation Performed:** Behavioral, not just counting: scratch probe (`bug008_features_probe.rs`) constructs fully populated `CardiacFeatures`/`EdaFeatures`/`RespirationFeatures`/`CouplingFeatures` and calls `evaluate_feature_quality` → `usable_feature_count = 28`, `total_feature_count = 28`, ratio = 1.000000, no issues. Dropping one field (pnn50=None) → 27/28 = 0.9643, i.e. numerator tracks population exactly. Independent field census of the four structs = 28, matching both constants.

**Result:** **RESOLVED** behaviorally — the reported denominator is now correct and 100% usable is reachable.

**Remaining Concern:** P3 — the denominator remains a hand-maintained literal (the plan's "dynamically calculated" was not implemented), and no test pins it to the actual field count (`tests/features_tests.rs:575` asserts only `usable > 0`). Future feature additions can silently re-skew the ratio.

---

## BUG-009 — parity tests / checked-in goldens

**Original Finding:** `filter_parity_tests.rs` / `peaks_parity_tests.rs` panicked on fresh checkout ("Failed to open golden filter/peaks JSON") because goldens weren't checked in.

**Previous Evidence:** `repro_parity_tests_missing_goldens.txt` (3 panics). Stage-1: `tests/golden_filter.json` (3.9 MB) + `tests/golden_peaks.json` now tracked; consumed at `filter_parity_tests.rs:317,371`, `peaks_parity_tests.rs:26`.

**Current Implementation:** Goldens checked into `tests/` alongside pre-existing `golden_ecg.json`.

**Validation Performed:** On the pristine clone (`git status --porcelain -- src/ tests/` empty): `cargo test --test filter_parity_tests --test peaks_parity_tests` → **7/7 + 3/3 pass**, including the three previously panicking tests (group C SOS parity, group D end-to-end parity, scipy peak parity). Captured in `bug009_output.txt`.

**Result:** **RESOLVED.**

**Remaining Concern:** None. (General hygiene note, outside this bug: the workspace `cargo test` at root exercises only the root crate; stage-1 found two clippy `-D warnings` failures in test targets — unrelated to BUG-009.)

---

## BUG-010 — `multimodal_quality` rustdoc error variant

**Original Finding:** Rustdoc named `SignalError::InvalidSamplingRate`; code returns `SignalError::EmptySignal` when all modalities are `None`.

**Previous Evidence:** Source inspection. Stage-1: rustdoc corrected.

**Current Implementation:** `src/multimodal/quality.rs:138` — "Returns [`SignalError::EmptySignal`] if all modality arguments are `None`."; matching return at `:161`.

**Validation Performed:** Source-confirm only (per task scope): rustdoc text and code path re-inspected at fec2668; they now agree. Captured in `bug010_output.txt`.

**Result:** **RESOLVED.**

**Remaining Concern:** None.

---

## BUG-018 — HRV interval quality/correction architecture (API-path scope only)

**Original Finding:** `hrv` op computes RMSSD over all detected intervals; no ectopy/artifact filtering; elderly-stratum RMSSD inflated by bigeminy (record 0554: 315.6 ms).

**Previous Evidence:** `validation/results/autonomic/metrics_extra.csv`. Stage-1: full `src/hrv/quality.rs` shipped (`BeatQuality`, `IntervalQuality`, `CorrectionPolicy{None,RejectInvalid,InterpolateLinear,InterpolateCubic,PercentThreshold}`, `classify_intervals`, `clean_rr_intervals`); deviations: no `IntervalCleaningConfig`, `clean_rr_intervals` takes `&CorrectionPolicy` directly.

**Current Implementation:** Bridge op `hrv-correct` (SPEC §A.2): peaks-envelope or raw `rr_intervals_ms` → `classify_intervals` (rolling-median, default 0.20, hardcoded 300–2000 ms artifact bounds) → `clean_rr_intervals(policy)` → `hrv_rmssd`/`hrv_mean_nn`.

**Validation Performed:** Smoke of the API path (`bug018_probe.py`) with a bigeminy-like RR train (20×800 ms + 10×[440,830] ms + 20×800 ms): all five policies execute; `interval_quality` kinds emitted per input interval; `reject_invalid`/`percent_threshold(0.2)` drop 2 intervals (n_nn 58/60), interpolation policies keep 60; raw `hrv` op RMSSD 228.2 ms vs corrected 203–207 ms; peaks-envelope input path works (59 intervals in); empty input → empty outputs + null metrics (not an error), per SPEC. Cubic vs linear compared directly: **byte-identical** outputs.

**Result:** **RESOLVED** within the delegated scope (correction policies exist and the full classify→correct→NN→HRV path is behaviorally exercisable). Deep HRV validation against annotation ground truth is Wave F's scope.

**Remaining Concern:** P2 — `CorrectionPolicy::InterpolateCubic` is behaviorally identical to `InterpolateLinear` (shared match arm per SPEC §A.3; confirmed byte-identical here): the variant name promises cubic spline interpolation it does not perform. Also flagged for Wave F: the rolling-median 20 % classifier flags only 2/20 intervals of *sustained* bigeminy (the alternating pattern *is* the local norm), so corrected RMSSD stays inflated (203 ms vs ~0 for the regular runs) — correction architecture exists but classifier efficacy on sustained arrhythmia is unproven. `BeatQuality` enum is exported but dead (no public producer/consumer; SPEC §A.3).

---

## BUG-019 — `partial_cmp().unwrap()` → `total_cmp`

**Original Finding:** Latent panic-capable sort on NaN in the integrated ECG signal (`src/ecg/peaks.rs:218` old); inputs pre-validated finite, so trigger was hypothetical.

**Previous Evidence:** Source citation only; no runtime occurrence. Stage-1: `total_cmp` confirmed at `src/ecg/peaks.rs:225`.

**Current Implementation:** `candidate_heights.sort_by(|a, b| a.total_cmp(b))`; additionally the filter stage validates its own output (`src/signal/filter.rs:505-506, 541-542` → `NonFiniteInput`), so overflow in the pipeline surfaces as an error before reaching the sort.

**Validation Performed:** Scratch probe (`bug019_nan_probe.rs`) using `catch_unwind` around `ecg_findpeaks_config` with signals engineered to produce non-finite intermediates from *finite* raw input: amplitudes 1e150/1e155/1e160/1e170/1e200/1e300 (squaring overflows to +inf from 1e155 up), alternating ±1e160 spikes, plus raw NaN/+inf controls. Result: **zero panics** in all 9 cases; overflow cases return clean `Err(NonFiniteInput)` from the filter-stage guard; 1e150 (squared = 1e300, still finite) legitimately returns 29 peaks. (Same probe doubles as the Rust-level NaN/+inf `threshold_multiplier` check for BUG-002.)

**Result:** **RESOLVED.** Even a hypothetical residual NaN can no longer panic (`total_cmp` is total over all f64 bit patterns), and the observed overflow path errors explicitly rather than propagating silent NaN.

**Remaining Concern:** P3 nit — the error message "Input signal contains non-finite values (NaN or Infinity)" is emitted even when the *raw input* is finite and only pipeline intermediates overflowed (same wrong-diagnosis flavor as original BUG-002, much milder: amplitude 1e155 ECG is not a realistic input).

---

## Cross-cutting observations (new issues)

| # | Issue | Severity | Evidence |
|---|-------|----------|----------|
| N1 | `SignalError::InvalidCutoffFrequency` reused as a generic parameter-validation variant (method strings BUG-001, threshold multiplier BUG-002, tolerance BUG-006). Display prefixes "Invalid cutoff frequency:" on unrelated messages. No dedicated `InvalidParameter` variant exists. | P3 | bug001/002/006 outputs; `src/ecg/clean.rs:29`, `src/ecg/peaks.rs:117`, `src/complexity/entropy.rs:32` |
| N2 | `CorrectionPolicy::InterpolateCubic` ≡ `InterpolateLinear` behaviorally (byte-identical NN output); already documented in SPEC §A.3, confirmed behaviorally here. | P2 | bug018_output.txt ("linear nn == cubic nn: True") |
| N3 | Rolling-median interval classifier misses sustained bigeminy (2/20 flagged); corrected RMSSD still inflated. Efficacy question for Wave F's deep HRV validation, not an API defect. | P2 (open question) | bug018_output.txt |
| N4 | Original repro script `repro_ecg_001_method_ignored.py` prints a stale hardcoded "CONFIRMED" conclusion line even though its own output now shows the bogus method rejected. Validation-asset cosmetic issue; left unmodified per protocol. | P3 (test-asset nit) | bug001_original_rerun.txt |
