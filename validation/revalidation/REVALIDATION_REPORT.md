# Lamina Post-Remediation Revalidation Report

**Revalidated commit:** `fec2668409b871094eeee35538b72dec776224f8` (Lamina `main`, post-remediation)
**Validation repo/bridge:** `lamina-reval` bridge commit `caa7484`
**Validation date:** 2026-09-07 (waves A–H + consolidation)
**Mode:** validation-only. **No fixes were implemented** — no Lamina source, tests, benches, bindings, or pre-existing validation files were modified.

## Executive Summary

The remediation resolves **15 of 20** original findings — including the
4 Hz EDA sampling-rate floor (BUG-003), `ecg_clean` method dispatch (BUG-001),
the fs-explicit `eda_findpeaks`/`rsp_findpeaks` APIs (BUG-004), the
checked-in golden fixtures (BUG-009), and the rsp `precleaned` flag
(BUG-007) — **but introduces a fleet-wide P0 ECG over-detection regression**
(BUG-NEW-03): on MIT-BIH the pooled false-positive count rose **698 → 17,884
(25×)** and mean per-record beat F1 fell **0.982 → 0.902**, with **15 records**
showing per-record F1 deltas < −0.01 (worst: record 123, F1 0.999 → 0.132). The same SPKI-clamp
mechanism double-detects T-waves/secondary humps on WESAD chest ECG (S3
baseline HR 55 → 110 bpm), BIDMC lead II, wrist ECG, and autonomic-aging
(record 0342 raw RMSSD 64.5 → 634.4 ms). **This regression must block
shipping the remediation as-is.**

Additional new issues: **P1** sustained-bigeminy evasion of the new HRV
interval classifier (BUG-NEW-02 — the exact pattern the architecture was built
for) and **P1** protobuf transport gaps (BUG-NEW-14); **P2**
`InterpolateCubic` ≡ `InterpolateLinear` (BUG-NEW-01) and rPPG `AutoDetect`
heuristic backwards (BUG-NEW-10) — the rPPG polarity contract machinery was
delivered and is mechanically exact, but the auto mode resolves to the wrong
phase in 29/30 SCAMPS cases.

Bright spots that must not be lost in the regression finding: the BUG-011 fix
**works on its target** (mitdb/228 normal-beat recall 0.6016 → 0.9982 at a
cost of only +24 FP, PVC recall 1.000), the wrist s6 detection blackout is
**eliminated** (longest gap 0.68 s), both previously-failed EDA datasets now
complete end-to-end with plausible SCR output, all 462 API-robustness cases
ran with **zero panics/timeouts/crashes**, and all cross-language protobuf
round-trips are byte-identical.

**Overall verdict: NOT fully validated — remediation partially successful with
one severe (P0) regression.** 15/20 findings fully resolved, 3 partially
resolved, 1 not resolved, 1 not reproduced;
15 new issues (1×P0, 2×P1, 6×P2, 6×P3); 53 regression rows, all tracing to
the single P0 mechanism.

## Environment

From the stage-1 baseline verification (pristine tree, `$HOME` copy; /mnt is
FUSE/noexec):

| Item | Value |
|---|---|
| Lamina commit | `fec2668409b871094eeee35538b72dec776224f8` (verified byte-identical to GitHub upstream for all source/test/doc files) |
| rustc / cargo | `rustc 1.98.1 (48a229cea 2026-09-01)` / `cargo 1.98.1`, rsproxy.cn sparse mirror |
| OS | `Linux 5.10.134-18.0.12.lifsea8.x86_64 x86_64` |
| `cargo fmt -- --check` | **PASS** (empty diff) |
| `cargo clippy --all-targets --all-features -- -D warnings` | **FAIL** — 2 test-target lints: unused import `BvpWaveform` (`tests/rppg_tests.rs:781`), `neg_multiply` (`tests/ecg_tests.rs:376`); library target clean (BUG-NEW-12) |
| `cargo test --all-features` | **PASS — 139 passed / 0 failed** across 14 integration binaries (root crate only; `lamina_py`/`lamina_dart` are non-default workspace members, not exercised) |
| Bridge hermetic tests | **71/71 pass** (validation/tests suite: 71 test functions across bridge/cli/manifest/metrics/registry/revalidation-ops/runner/schema, run by the waves at bridge commit `caa7484`; not re-executed during consolidation — no toolchain in this container); cross-language repo suites 4/4 Rust, 3/3 Python, 3/3 Dart (Wave H) |
| Dependencies | No root `Cargo.lock` tracked (165 packages resolved at build); `validation/lamina_bridge/Cargo.lock` tracked; key pins: ndarray 0.17.2, realfft 3.5.0, rustfft 6.4.1, statrs 0.18.0, biquad 0.4.2, find_peaks 0.1.5 |
| Bridge | `lamina_bridge` v0.1.0, release build against post-remediation Lamina; 17 ops exercised by Wave G |
| Cross-language env (Wave H) | prost 0.13 / protoc 25.6; Python 3.12.12 + protobuf 6.32.1; Dart SDK 3.13.3 + protobuf 6.0.0 |

## Methodology

Full details in [`methodology.md`](methodology.md). Summary:

- **Preserved from the original validation**: event matching (greedy
  one-to-one; 150 ms ECG/PPG, 0.5 s respiration, 1.0 s SCR onsets); seeds
  (42 for ecg/ppg/bidmc/wrist, 0 for autonomic/rppg); adapters and runner
  configs unchanged; bridge ops extended additively with the historical
  `"none"` → `""` method-token mapping verified equivalent.
- **Documented changes (task §5)**: new bridge ops for new APIs
  (`eda-clean-config`, `rppg-polarity`, `hrv-correct`); class-conditional
  recall added for record 228 as NEW analysis alongside the old all-beat
  methodology; HRV corrected path (`hrv-correct reject_invalid`) reported as a
  new column next to the raw-RR baseline methodology; SCAMPS polarity matrix
  as a new analysis; threshold-multiplier sensitivity runs used as
  diagnostics only (never as a proposed fix).
- **Counting conventions** (for `summary.json`): `bugs_resolved` counts
  verdict==RESOLVED including BUG-011 resolved-on-target; BUG-020
  (NOT REPRODUCED) is not counted as
  resolved; BUG-015 IS counted as resolved (question conclusively answered:
  reference-side defect, no Lamina-side change — analogous to BUG-014). All `regressions.csv` rows trace to BUG-NEW-03 (P0).
- **Accuracy rule applied throughout**: where a wave's prose notes and its
  CSV disagree, the CSV is authoritative; unverifiable items are marked
  INCONCLUSIVE rather than guessed.

## Dataset Accessibility

**9 datasets revalidated** (all with rerun evidence post-remediation):

| Dataset | Records | Status post-remediation |
|---|---|---|
| mit-bih-arrhythmia | 48/48 ok | revalidated (seed 42) — see P0 regression |
| mit-bih-noise-stress | 7/7 ok | revalidated — mid-SNR degradation |
| bidmc | 12/12 ok | revalidated (seed 42) — resp metrics byte-identical |
| wrist-ppg-exercise | 19/19 ok | revalidated (seed 42) — PPG byte-identical; ECG channel mixed |
| wesad | 15/15 ok | revalidated (seed 0) — 34 metric rows changed (ECG detector) |
| autonomic-aging | 12/12 ok | revalidated (seed 0) — 4/12 peak counts changed |
| wearable-exam-stress | 30/30 ok | **15 EDA recordings FIXED** (were BUG-003 failures); BVP bit-identical |
| big-ideas | 2/2 ok | **FIXED** (dataset previously failed on BUG-003) |
| scamps | 30/30 ok | revalidated (seed 0) — 510/510 metric rows bitwise identical |

**9 datasets remain inaccessible — unchanged from the previous validation**
(reasons probed and recorded 2026-09-06 in `docs/validation/datasets.md`):

| Dataset | Reason (unchanged) |
|---|---|
| pulsedb | official hosts unreachable; mirrors blocked |
| mimic-iii-waveform | PhysioNet credentialed access (CITI + DUA); 403 verified |
| mimic-iii-ext-ppg | same credentialed barrier |
| pure | e-mail application to TU Ilmenau required |
| ubfc-rppg | official page unreachable; Kaggle mirror requires auth |
| cohface | Zenodo restricted (EULA, academic signatory); host IP-blocked |
| ubfc-phys | smallest subject archive 91.7 GB, beyond download timebox |
| mmpd | signed release agreement from faculty address required |
| ibvp | EULA signed by academic supervisor required |

## BUG-001 — `ecg_clean` silently ignores its `method` argument

- **Original Finding:** `method` parameter ignored (`_method`); byte-identical output for `none`/`neurokit`/`pantompkins`/`biosppy`/bogus strings; no error for unknown methods.
- **Previous Evidence:** `validation/results/bugs-repro/repro_ecg_001_method_ignored.{py,txt}` (max abs diff 0.0 for all 15 pairs; bogus accepted).
- **Current Implementation:** `src/ecg/clean.rs:17-31` dispatches on trim+lowercased method; `""`/`none`(bridge)/`neurokit`/`pantompkins`/`biosppy` → the single 0.5 Hz HP order-5 pipeline; anything else → `SignalError::InvalidCutoffFrequency("Unsupported ECG cleaning method: …")`. Rustdoc now states the single methodology honestly.
- **Validation Performed (Wave A):** bridge probe on 30 s synthetic ECG @360 Hz — all 10 pairwise max-abs-diffs = 0.0 among accepted names; `bogus`, `NEUROKIT2`, `pan-tompkins`, `hamilton` rejected with clear `lamina_error`; normalization (`'  neurokit  '`, `'BioSPPy'`) accepted; original repro rerun unported now errors on bogus.
- **Result:** **RESOLVED.** Unknown methods rejected; dispatch real (all accepted names share one pipeline, which does not violate the written contract — signal-contracts.md §2.2 is silent on cleaning methods).
- **Remaining Concern (P3):** `InvalidCutoffFrequency` is the wrong variant for a method-string error; four aliases for one pipeline remain an API footgun (now documented).

## BUG-002 — `ecg-peaks` threshold_multiplier out-of-range → misleading NonFiniteInput

- **Original Finding:** Any `threshold_multiplier ≥ 1.0` failed with "Input signal contains non-finite values" on provably finite input.
- **Previous Evidence:** `repro_ecg_002_threshold_multiplier.{py,txt}` (1.0/1.1/1.5/2.0/3.0 all wrong-message failures).
- **Current Implementation:** `EcgPeakDetectionConfig::validate()` (`src/ecg/peaks.rs:116-120`) rejects tm non-finite, ≤0, ≥1 with "Threshold multiplier must be strictly between 0.0 and 1.0".
- **Validation Performed (Wave A):** bridge sweep tm ∈ {0.0, 1.0, 1.5, −0.5} → all rejected with the clear message; 0.9999/1e-12/0.25/0.5 accepted (peak counts 29/87/79/70); Rust-level NaN/+inf probe → same clear error; original repro rerun unported.
- **Result:** **RESOLVED.**
- **Remaining Concern (P3):** variant reuse (`InvalidCutoffFrequency`); the *same* wrong-variant pattern persists in 6 other validators — see BUG-NEW-04.

## BUG-003 — 4 Hz EDA sampling-rate floor (`eda_clean` hardcoded 5 Hz lowpass)

- **Original Finding:** any fs ≤ 10 Hz failed `InvalidCutoffFrequency` (5 Hz cutoff vs 2 Hz Nyquist @ fs=4); blocked all 17 Empatica E4 4 Hz EDA recordings (15 wearable-exam-stress + 2 big-ideas).
- **Previous Evidence:** per-recording runner failures; synthetic repro (fs 4/8/10 fail, 16 ok).
- **Current Implementation:** `EdaCleaningConfig` (`src/eda/clean.rs:7-61`) with `lowpass_cutoff_hz=Some(5.0)`, `pass_through_if_nyquist_violated=true` default; bypass logic `:101-110`; bridge op `eda-clean-config` exposes the tri-state cutoff, order, and flag.
- **Validation Performed (Wave C):** full adapter reruns of all three datasets; plain `eda-clean` repro sweep (fs 4/8/10 pass-through rms_change ~1e-17; fs=16 filters, rms 3.1e-4); `eda-clean-config` path matrix at fs=4 and fs=64 (`eda_config_paths.csv`); deep-validation of the full raw→clean→decompose→peaks chain on 4 recordings (`eda_deepval_report.json`, `plots/*_chain.png`); sampling sweep 4–64 Hz (`eda_sweep.csv`).
- **Result:** **RESOLVED** (Wave C verdict: "fixed"). All 17 previously-failed recordings now complete; output at 4 Hz is scientifically plausible: zero NaNs, reconstruction invariant |tonic+phasic−clean| ≤ 4.4e-16, SCR rates 0.36–5.73/min (median 2.26) on wearable-exam-stress and 6.33/10.88/min on big-ideas, sane event structure (min ICI 0.5 s, median amplitude 0.03–0.08 μS). Above-Nyquist cutoff silently passes through by default; explicit `pass_through_if_nyquist_violated:false` errors per SPEC A.2; `lowpass_cutoff_hz:null` disables filtering.
- **Remaining Concern:** none blocking. 4 Hz EDA is **not low-pass filtered at all** by default (contract-permitted bypass); high-frequency noise is retained unless callers pass an explicit cutoff. See also BUG-NEW-11 (boundary transients).

## BUG-004 — `eda_findpeaks`/`rsp_findpeaks` hardcode fs=100 Hz

- **Original Finding:** convenience wrappers hardcoded fs=100: `rsp_findpeaks` kept 1 of 25 breaths at true fs=25; `eda_findpeaks` kept 10 (vs config-correct 5) pulses at true fs=250.
- **Previous Evidence:** `repro_fs100_hardcoded_probe.rs/.txt`.
- **Current Implementation:** breaking signature change — both take `sampling_rate: f64`, validate fs>0 & finite → `InvalidSamplingRate`, then delegate to the `_config` path (`src/eda/peaks.rs:232-237`, `src/rsp/peaks.rs:347-352`).
- **Validation Performed (Wave A):** archived probe (already ported by the remediation commit) compiles unmodified and its claims verify (25==25, 5==5 vs config paths); new timing-consistency probe (fs=25 vs 100 peak times agree within half a sample); fs ∈ {0, −25, NaN, +inf} → clear `InvalidSamplingRate`.
- **Result:** **RESOLVED.** The fs-less signatures no longer exist — silent misuse is a compile error, the strongest possible fix class.
- **Remaining Concern:** none behavioral. (Cosmetic: archived probe keeps a stale "hardcoded fs=100" comment.)

## BUG-005 — Inconsistent rPPG output sign convention across algorithms

- **Original Finding:** green/pos produce intensity-phase waveforms, chrom BVP-phase; silent degradation of `ppg-peaks` (baseline: green raw r=−0.921, pos −0.823, chrom +0.36 vs d_ppg; green peak F1 0.245).
- **Previous Evidence:** baseline scamps metrics; sign-flip probe improving green/pos HR MAE (pos 27.50/12.17/11.67 → 7.73/1.42/0.95).
- **Current Implementation:** `SignalPolarity {Normal, Inverted, AutoDetect}` (`src/rppg/config.rs:78`), `BvpWaveform` + `to_bvp_waveform(polarity)` (`src/rppg/signal.rs:264,234,302`), pulse-phase contract `docs/architecture/signal-contracts.md` §2.4; new bridge op `rppg-polarity`.
- **Validation Performed (Wave E):** unchanged-adapter rerun (30/30 ok — **all 510 metric rows bitwise identical to baseline**); polarity matrix 10 videos × 3 algorithms × 3 modes (`polarity_matrix.csv`); exact sign-flip check (30/30 pairs `inverted == −normal`, max |sum| = 0; `signflip_check.csv`); AutoDetect audit against d_ppg ground truth; morphology plots.
- **Result:** **PARTIALLY RESOLVED (P2).** Contract + exact flip machinery delivered and verified; convention-correct polarity massively fixes alignment (green F1 0.245→0.918, pos F1 0.540→0.920, pos HR MAE 16.19→7.90). But raw outputs are unchanged by default (`polarity=normal` is pass-through, so the default BVP violates the §2.4 contract for green/pos) and `AutoDetect` is backwards (BUG-NEW-10, 1/30 agreement). A caller following the documented default path still gets wrongly-phased waveforms for 2/3 algorithms.
- **Remaining Concern:** no default phase normalization; green HR MAE is insensitive to polarity (11.39→12.44 despite F1 → 0.918), so HR-only benchmarks can mask phase bugs (report peak F1/correlation alongside).

## BUG-006 — `sample_entropy` returns `Ok(+inf)` on zero template matches

- **Original Finding:** zero template matches → `Ok(f64::INFINITY)`, undocumented.
- **Previous Evidence:** `repro_entropy_and_rsp_double_clean.txt` §a2.
- **Current Implementation:** rustdoc Scientific Contract now documents "or `f64::INFINITY` if zero template matches occur" (`src/complexity/entropy.rs:11`); the r>0 validation (`:31-35`) is pre-existing.
- **Validation Performed (Wave A):** bridge probes — noise with tiny r → `sample_entropy=null, is_infinite=true`; r ≤ 0 → clear "Tolerance threshold r (X) must be > 0.0 and finite"; constant signal with default r=0.2·std=0 errors cleanly; periodic/ramp → −0.0; noise with default r → finite 2.282; original repro rerun unported.
- **Result:** **RESOLVED** (documentation action + every behavioral clause exercised).
- **Remaining Concern (P3):** `InvalidCutoffFrequency` variant reused for the tolerance error.

## BUG-007 — `rsp_cycles_config` re-cleans its input (double filtering)

- **Original Finding:** `rsp_cycles_config` unconditionally re-cleaned input, double-filtering pre-cleaned signals.
- **Previous Evidence:** source inspection (pre-remediation `src/rsp/peaks.rs`).
- **Current Implementation:** `RspProcessingConfig.precleaned: Option<bool>` (`src/rsp/peaks.rs:22`, default `Some(false)`, bypass at `:178-186`); bridge exposes optional `precleaned`.
- **Validation Performed (Wave D):** contract probes (`rsp_contract.csv`): absent ≡ false (identical outputs); `precleaned=true` verifiably skips the bandpass (92 vs 44 cycles on a 2 Hz-artifact signal); identity test `rsp_cycles(rsp_clean(raw), precleaned=true)` produces **exactly** the cycles of `rsp_cycles(raw, precleaned=false)`; double-cleaning still shifts amplitudes (2.0089→2.0332) if the caller misuses the API.
- **Result:** **RESOLVED.**
- **Remaining Concern:** none for the contract; callers who pre-clean but forget the flag still double-filter (~1% amplitude shift) — documented, informational.

## BUG-008 — `FeatureQuality.total_feature_count` hardcoded to 30 (only 28 fields)

- **Original Finding:** denominator 30 vs 28 actual increments → ratio capped at 0.933 even for complete feature vectors.
- **Previous Evidence:** source grep/count; stage-1 rated the fix PARTIAL (value corrected 30→28 but still a literal).
- **Current Implementation:** `let total_features = 28;` (`src/features/quality.rs:138`), exactly 28 `is_some()` increments (7 cardiac + 9 EDA + 6 respiration + 6 coupling).
- **Validation Performed (Wave A):** behavioral Rust probe — fully populated structs → 28/28, ratio 1.000000; one field dropped → 27/28 = 0.9643; independent field census = 28.
- **Result:** **RESOLVED** behaviorally — correct denominator, 100% usable reachable.
- **Remaining Concern (P3):** still a hand-maintained literal (the plan's "dynamically calculated" was not implemented); no test pins it to the field count — future feature additions can silently re-skew the ratio (doc-drift item).

## BUG-009 — parity tests panic on fresh checkout (goldens not checked in)

- **Original Finding:** `filter_parity_tests.rs`/`peaks_parity_tests.rs` panicked opening missing golden JSONs.
- **Previous Evidence:** `repro_parity_tests_missing_goldens.txt` (3 panics).
- **Current Implementation:** `tests/golden_filter.json` (3.9 MB), `golden_peaks.json`, `golden_{eda,ppg,rsp}.json` checked in; consumed at `filter_parity_tests.rs:317,371`, `peaks_parity_tests.rs:26`.
- **Validation Performed (Wave A):** on a pristine clone, `cargo test --test filter_parity_tests --test peaks_parity_tests` → **7/7 + 3/3 pass** incl. the three former panics.
- **Result:** **RESOLVED.**
- **Remaining Concern:** none. (Hygiene note: root `cargo test` exercises only the root crate.)

## BUG-010 — `multimodal_quality` rustdoc names the wrong error variant

- **Original Finding:** rustdoc named `InvalidSamplingRate`; code returns `EmptySignal` when all modalities are `None`.
- **Previous Evidence:** source inspection.
- **Current Implementation:** rustdoc corrected (`src/multimodal/quality.rs:138`; return at `:161`).
- **Validation Performed (Wave A):** source-confirm per task scope — doc and code now agree.
- **Result:** **RESOLVED.**
- **Remaining Concern:** none.

## BUG-011 — mitdb/228: normal beats under-detected after tall PVCs (SPKI inflation)

- **Original Finding:** record 228 recall 0.6016 / F1 0.7492 (TP 1235/FP 9/FN 818, n_ref 2053); SPKI inflated by tall PVCs → subsequent normal beats fall below threshold.
- **Previous Evidence:** baseline metrics.csv; REMEDIATION_PLAN Phase 4 (normal-beat recall bar ≥ 0.950).
- **Current Implementation:** SPKI/NPKI clamping in `src/ecg/peaks.rs` — `y_eff = y_val.min(spki)` (l.247), searchback `sb_val.min(2.5*spki)` (l.278), `y_val.min(2.5*spki)` (l.296); 3:1 PVC disparity fixture in `tests/ecg_tests.rs:323-339`.
- **Validation Performed (Wave B):** full 48-record suite rerun (seed 42); direct bridge probe on 228 reproducing suite output exactly; NEW class-conditional recall analysis against `.atr` symbols (`228_class_recall.csv`); neighbor scan 221/222/223/231/232/233; threshold diagnostic.
- **Result:** **RESOLVED on the target record.** All-beat F1 0.7492 → **0.9913**; recall 0.6016 → 0.9985; TP/FP/FN 1235/9/818 → **2050/33/3** (+815 TP at only +24 FP); HR MAE 14.87 → 1.41 bpm. Class-conditional recall: N **0.9982** (1685/1688, plan bar ≥0.950 met), V **1.0000** (362/362 — no PVC cost), other 3/3. Remaining 3 FNs are N beats at 615.8/1218.2/1475.4 s.
- **Remaining Concern:** the same change severely regresses neighbors 231/232/222 and 12 other records (BUG-NEW-03, P0). The 228 mechanism fix is sound; the resulting default operating point is too permissive fleet-wide.

## BUG-012 — wrist s6_low_resistance_bike ~90 s detection blackout

- **Original Finding:** ~90 s blackout (per-30 s bins [66, 2, 0, 3, …] vs ~57 annotations/bin), F1 0.657 / recall 0.614, FN 207.
- **Previous Evidence:** baseline wrist metrics + binning.
- **Current Implementation:** same SPKI clamp + 6-case regression fixtures (`test_ecg_6case_regression_matrix`).
- **Validation Performed (Wave B):** adapter-identical direct probe (chest_ecg @256 Hz, 536 annotated beats, 280 s); baseline binning reverse-engineered exactly (`np.histogram`, `linspace(0,280,10)`); longest-gap scan; threshold_multiplier sensitivity (diagnostic).
- **Result:** **PARTIALLY RESOLVED (P1).** Blackout **eliminated** — longest gap anywhere 0.68 s, recall 0.614 → 0.966, bins 1–3 now 67/91/92 detections. But the plan's pass bar **F1 ≥ 0.900 is not met**: F1 0.736, precision 0.595, 353 FP; bins 2–8 over-detect 1.5–1.85× (T-wave/secondary-hump double detection, BUG-NEW-03 signature). Diagnostic: multiplier 0.5 → F1 0.919 with no blackout (longest gap 1.29 s).
- **Remaining Concern:** residual FP over-detection at the default operating point; elevated timing MAE (34.8 ms) consistent with matches locking onto features offset from the annotated R peak.

## BUG-013 — paced-beat lead sensitivity (mitdb/104: V5 F1 0.683 vs V2 0.982)

- **Original Finding:** strong lead sensitivity on paced record 104.
- **Previous Evidence:** baseline per-channel metrics.
- **Current Implementation:** documentation (`docs/validation/BUGS.md:774-812`) + the ECG detector changes above.
- **Validation Performed (Wave B):** record 104 both channels through the bridge; synthetic paced-spike probe (regression matrix case 5).
- **Result:** **RESOLVED (symptom) (P2).** V5 F1 0.6834 → 0.9744; V2 0.9820 → 0.9677 — the channel gap is closed and the synthetic paced case is perfect (F1 1.000).
- **Remaining Concern:** V2 is now marginally below the plan bar (0.9677 < 0.980); both leads carry FP inflation (V5 FP 2 → 98) consistent with BUG-NEW-03; at multiplier 0.5, 104-V5 falls back to F1 0.63, i.e. its improvement partly rides the same low-threshold regime.

## BUG-014 — WESAD S3 chest EDA @700 Hz: 147 SCRs/4 min baseline-window anomaly

- **Original Finding:** S3 baseline 147 SCRs/4 min (36.8/min) vs S3 stress 9.8/min; ambiguous rate-dependent over-segmentation vs dataset characteristic.
- **Previous Evidence:** baseline WESAD run.
- **Current Implementation:** unchanged detector; fs-explicit API (BUG-004); multi-rate tests exist for `eda_clean`/`eda_decompose` (fs ∈ {4, 16, 100, 700}).
- **Validation Performed (Wave C):** identical adapter windows, all 5 subjects × {baseline, stress}; `eda-peaks` at native 700 Hz vs harness-side anti-aliased decimation to 10 Hz and 4 Hz (`bug014_scr_counts.json`, plot).
- **Result:** **RESOLVED — not a Lamina defect** (Wave C verdict: "resolved-not-a-lamina-defect"). Counts scale sanely: S3_baseline 147 (700 Hz) → 130 (10 Hz, −12%) → 49 (4 Hz, smoothing merges small bumps). The raw trace shows periodic ~1 Hz conductance oscillations (~0.02 μS pk-pk, respiration-coupled chest-strap crosstalk) that legitimately exceed the 0.01 μS amplitude threshold. Group SCR medians unchanged (stress−baseline delta +8.0/min, identical to baseline).
- **Remaining Concern (P3):** `eda_findpeaks_events` has no periodicity/respiration-crosstalk gating — regular ~1 Hz modulation on chest EDA is reported as SCRs. Dataset-facing caveat, not a rate-handling bug.

## BUG-015 — Large Lamina-vs-E4 wrist-BVP heart-rate gap (wearable-exam-stress)

- **Original Finding:** Lamina-vs-E4 HR.csv MAE 26–47 bpm; ambiguous Lamina defect vs E4 reference error.
- **Previous Evidence:** baseline wearable-exam-stress hr_* metrics; IBI cross-check.
- **Current Implementation:** ppg path untouched by remediation.
- **Validation Performed (Wave C):** full adapter rerun; recomputed Lamina BVP IBI vs E4 IBI.csv aligned MAE for S2_Final and S5_midterm_2.
- **Result:** **RESOLVED** — question conclusively answered: the defect is reference-side, with **no Lamina-side change** (P3). (Wave C verdict, verbatim: "unresolved-unchanged (reference-side, as previously suspected)".). Every BVP session's peaks and hr_* metrics are **bit-identical to baseline**; HR MAE persists at 26.38–47.13 bpm (median 35.16). The counter-evidence reproduces exactly: Lamina BVP inter-beat intervals agree with E4's own confident IBI output within 28.9 ms (S2_Final, n=8361) and 45.4 ms (S5_midterm_2, n=4056); E4 HR.csv contains implausible seated-exam HR (150–170 bpm).
- **Remaining Concern (P3):** E4 HR.csv remains unreliable as a reference; no Lamina-side defect found. (Documentation-only remediation per plan; BUGS.md:883-920.)

## BUG-016 — wrist PPG over-counting under motion artifact

- **Original Finding:** foot-strike spikes accepted as beats; HR MAE stratified by activity (walk 15.9 / run 32.9 / bike-low 14.7 / bike-high 20.3; s1_walk bias +33.0 bpm).
- **Previous Evidence:** baseline wrist-ppg-exercise metrics.
- **Current Implementation:** no detector change (by design); limitation documented in `docs/validation/public-dataset-validation.md` ("Motion remains the hard case for wrist PPG") with the activity-stratified numbers.
- **Validation Performed (Wave D):** full 19-record rerun — all PPG/HR metrics **byte-identical** to baseline (HR MAE overall 20.75; motion 23.61 vs stationary 16.81; run worst 32.90; s1_walk bias +32.96).
- **Result:** **RESOLVED (documentation action).** Behavior matches the documented limitation exactly; signal-contracts.md §2.3 makes no motion-robustness promise, so no contract violation.
- **Remaining Concern (P3, doc drift):** the PPG contract section itself (signal-contracts.md §2.3) still lacks an explicit "single-channel detector, no motion-artifact rejection" caveat — it lives only in the validation report (Wave D NEWD-02, folded here).

## BUG-017 — rsp-cycles over-detection at ~6 brpm (bidmc05 biphasic)

- **Original Finding:** bidmc05 F1 0.658 (P 0.490 / R 1.000), 98 detected vs 48 annotated at ~6 brpm; baseline digest attributed it to the 12 s `max_breath_interval_sec` ceiling.
- **Previous Evidence:** baseline BIDMC metrics; digest structural-rejection story.
- **Current Implementation:** default `max_breath_interval_sec` raised 12 → 20 (`src/rsp/peaks.rs:32`, behaviorally confirmed); the promised **secondary peak prominence gating is absent** — no prominence parameter exists; gates are duration [1.2, 20] s and `min_amplitude` 0.05.
- **Validation Performed (Wave D):** full BIDMC rerun (12/12, all rsp metrics byte-identical) + direct probes of bidmc05 RESP under cap 12 / 20 / absent.
- **Result:** **NOT RESOLVED (P3).** All three configs yield 98 cycles, F1 0.6575 — the ceiling change is a **no-op** on this record (detected durations span only 2.56–7.18 s; zero candidate intervals in (12, 20] s). The true mechanism is biphasic peak doubling (mean 2.06 detections per annotated breath, alternating 5.55/4.16 s intervals; minimum surviving cycle amplitude 0.052). The digest's structural-rejection story is disproven (baseline recall was already 1.000).
- **Remaining Concern (P3, doc drift):** prominence gating — the remediation that would address the doubling — was claimed in REMEDIATION_PLAN Phase 2 but not implemented (Wave D NEWD-02, folded here). Low severity per plan (limitation).

## BUG-018 — `hrv` op computes RMSSD over all detected RR intervals (no ectopy/artifact filtering)

- **Original Finding:** no interval quality/correction stage; autonomic-aging RMSSD medians confounded (young/middle/old = 52.3/24.9/142.8 ms; bigeminy record 0554 RMSSD 315.6 ms).
- **Previous Evidence:** `reval-baseline/autonomic_metrics_extra.csv`; REMEDIATION_PLAN §4.4 five-stage design.
- **Current Implementation:** `src/hrv/quality.rs`: `BeatQuality`, `IntervalQuality`, `CorrectionPolicy {None, RejectInvalid, InterpolateLinear, InterpolateCubic, PercentThreshold(f64)}`, `classify_intervals` (hardcoded 300–2000 ms artifact bounds + 20% self-inclusive rolling-median ectopy test), `clean_rr_intervals(rr, &CorrectionPolicy)`; bridge op `hrv-correct`. Deviations from plan: no `IntervalCleaningConfig`; artifact bounds not configurable.
- **Validation Performed (Wave F + Wave A API smoke):** 41-row policy semantics matrix with independent NumPy cross-checks; boundary probes (299/300/2000/2001 ms); cubic-vs-linear falsification; full aging rerun + per-record recompute (raw vs corrected); edge probes (empty input, all-invalid reject, bogus policy, percent-param bounds, classify_threshold override).
- **Result:** **PARTIALLY RESOLVED (P2).** A documented, user-visible classify→correct→NN→HRV path now exists and is semantically correct for all 5 policies (passthrough ≤1 ulp; reject removes exactly non-normal; interpolations bit-exact on valid samples; percent band honored; boundaries exact; isolated ectopic couplets caught 6/6). Record 0554: RMSSD 315.6 (baseline) → 224.1 (raw current) → **102.2 corrected**; corrected strata medians 54.2/18.2/89.5 ms. **But:** the default path is still uncorrected (`hrv` op unchanged; `hrv-correct` default policy `none`); sustained periodic ectopy evades the classifier (BUG-NEW-02, P1); `InterpolateCubic` ≡ linear (BUG-NEW-01, P2); the age trend remains non-monotone even after correction.
- **Remaining Concern:** detector confound — current peak counts differ from baseline on 4/12 aging records (0342: 478→955, 0637: 442→1081, 0554: 933→998, 0276: 581→583), so raw-path RMSSD changes on those records are detector-driven (BUG-NEW-03), not architecture-driven. `BeatQuality` is dead code and `IntervalQuality::Missing` unreachable via the bridge (BUG-NEW-13); no public `sdnn`.

## BUG-019 — `partial_cmp().unwrap()` panic-capable on NaN in `src/ecg/peaks.rs`

- **Original Finding:** latent panic-capable sort (old line 218); trigger hypothetical (inputs pre-validated finite).
- **Previous Evidence:** source citation only.
- **Current Implementation:** `total_cmp` at `src/ecg/peaks.rs:225`; the filter stage additionally validates its own output (`src/signal/filter.rs:505-506, 541-542`).
- **Validation Performed (Wave A):** `catch_unwind` probe with 9 adversarial cases (amplitudes 1e150–1e300, alternating ±1e160 spikes, raw NaN/inf controls) — **zero panics**; overflow → clean `Err(NonFiniteInput)`; 1e150 (still finite when squared) legitimately returns 29 peaks.
- **Result:** **RESOLVED.**
- **Remaining Concern (P3 nit):** the "Input signal contains non-finite values" message fires even when only pipeline intermediates overflowed (same wrong-diagnosis flavor as original BUG-002, much milder).

## BUG-020 — bidmc03 ECG lead II near-flatline (153 ECG vs 610 PPG beats)

- **Original Finding:** ecg-peaks found 153 beats vs 610 PPG beats; attributed to a near-flatline lead II (dataset issue); remediation: document + verify `evaluate_ecg_quality` flags low quality.
- **Previous Evidence:** baseline BIDMC metrics; HR consistency MAE 51.6 bpm.
- **Current Implementation:** `evaluate_ecg_quality` (`src/multimodal/quality.rs:47`) — rule-based, **not exposed through the bridge**; documentation in `docs/validation/BUGS.md:1130-1153`.
- **Validation Performed (Wave D):** BIDMC rerun; direct bridge ecg-peaks/ppg-peaks on bidmc03; scratch Rust harness (outside the repo) calling `evaluate_ecg_quality`.
- **Result:** **NOT REPRODUCED (P3).** The anomaly no longer occurs: ecg-peaks now finds **628 beats vs 610 PPG** (mean 78.5 vs 76.2 bpm); PPG-vs-ECG HR consistency MAE improved 51.6 → 5.1 bpm. `evaluate_ecg_quality` on current peaks: score 1.0, valid, no issues; fed a baseline-like 153-beat detection: score 0.6 with `UnplausibleHeartRate` but still `valid=true` (threshold ≥ 0.5). The lead is low-voltage (p2p 1.26 mV, std 53 µV) but QRS is clearly detectable — the original "near-flatline dataset-issue" attribution was likely wrong: the 153-beat under-detection looks like a detector defect that disappeared with the ECG remediation.
- **Remaining Concern (P3, doc drift):** docs (`public-dataset-validation.md`, `BUGS.md`, `datasets.md`) still describe the stale 153-vs-610 anomaly as a dataset issue; the quality API is unreachable via the bridge; `valid=true` at score 0.6 means the gate would not have rejected even the baseline broken detection.

## Dataset Results

Full machine-readable tables: `dataset_results.csv` (60 aggregate rows) and
`record_results.csv` (1626 per-record rows). Headlines below.

### ECG — MIT-BIH arrhythmia (48/48 ok, seed 42, 150 ms tolerance)

| metric | baseline | current | delta |
|---|---|---|---|
| mean per-record beat F1 | 0.9820 | 0.9016 | **−0.080** |
| pooled beat F1 | 0.9856 | 0.9021 | −0.084 |
| pooled FP | 698 | **17,884** | **+17,186 (25×)** |
| pooled FN | 2,492 | 5,242 | +2,750 |
| mean HR MAE (bpm) | 0.93 | 8.61 | +7.69 |
| mean timing MAE (ms) | 4.57 | 9.21 | +4.64 |

Worst current records (baseline → current F1): **123** 0.999→0.132 (FP 0→2842),
**108** 0.885→0.215, **117** 1.000→0.380 (FP 0→2472), **100** 1.000→0.409,
**232** 0.999→0.649 (detections 1.86× reference), **113** 0.9997→0.666,
**111** 0.999→0.705, **231** 0.999→0.705, **115** 1.000→0.818.
Improvements: **228** 0.7492→**0.9913** (BUG-011 target, intended), **104** 0.6834→0.9744 (BUG-013; FP 2→98).

Record 228 detail (deep-dive, `ecg/record228_deepdive.md`): TP/FP/FN
1235/9/818 → 2050/33/3; class-conditional recall N 0.9982 / V 1.0000 / other
1.0000; HR MAE 14.87 → 1.41 bpm; timing MAE 2.40 → 2.17 ms.

### ECG — MIT-BIH noise-stress (7/7 ok; base record 118 SNR gradient)

| SNR | baseline F1 | current F1 | delta | FP base→cur |
|---|---|---|---|---|
| 0 dB | 0.831 | 0.820 | −0.011 | 750→863 |
| 6 dB | 0.884 | 0.842 | −0.042 | 530→728 |
| 12 dB | 0.959 | 0.888 | −0.071 | 170→493 |
| 18 dB | 0.999 | 0.979 | −0.021 (**below plan bar 0.990**) | 3→81 |
| 24 dB | 1.000 | 1.000 | 0.000 | 1→1 |

(119e00 0.787→0.771; 119e24 1.000→0.995.) Gradient shape preserved; mid-SNR
degraded by FP inflation at every non-24 dB level; the plan's 0 dB bar (≥0.830)
is also marginally missed (0.820).

### ECG — wrist s6 blackout record (chest ECG @256 Hz, 536 beats)

F1 0.657 → 0.736; recall 0.614 → **0.966**; precision 0.708 → 0.595 (353 FP);
longest detection gap ~90 s → **0.68 s**. Plan bar F1 ≥ 0.900 **not met**.
Per-bin counts in `ecg/wrist_s6_bins.csv` / `ecg/wrist_s6_blackout.md`.
Diagnostic (not a fix): `threshold_multiplier` 0.5 → F1 **0.919**, no blackout.

### EDA / autonomic (seed 0, SCR tolerance 1.0 s)

| dataset | baseline | current |
|---|---|---|
| wearable-exam-stress | 15 BVP ok, **15 EDA FAILED** (BUG-003) | **30/30 ok** (+15 fixed, 0 regressions) |
| big-ideas | **failed** (2/2 EDA, BUG-003) | **2/2 ok** |
| wesad | 15/15 ok | 15/15 ok (34 metric rows changed — all chest-ECG peak counts + derived hr_* consistency; every EDA/RSP/BVP metric bit-identical) |

Metric-level (`eda/dataset_results_eda.csv`, 358 rows): 288 unchanged
(bit-identical), 17 new (previously-errored EDA recordings), 17 improvement
(recording_status failed→ok), 36 changed (WESAD chest-ECG peak counts on 5
recordings + derived cross-device hr_* metrics + 2 known-groups rows).

SCR plausibility at 4 Hz: wearable-exam-stress 0.36–5.73/min (median 2.26),
big-ideas 6.33/min (015) and 10.88/min (003); no NaNs; reconstruction
invariant ≤ 4.4e-16; min ICI 0.5 s; median amplitude 0.03–0.08 μS.
`eda-clean-config` paths (`eda_config_paths.csv`): above-Nyquist cutoff →
silent pass-through by default (filter_applied=false, output == input),
explicit `pass_through_if_nyquist_violated:false` → `InvalidCutoffFrequency`
per SPEC A.2; `lowpass_cutoff_hz:null` → no filtering; cutoff 1.0 Hz filters
at both fs=4 and fs=64. Sampling sweep (4–64 Hz): recall 1.0 at 4–8 Hz, 0.875
at ≥16 Hz (one 0.25 μS bump below threshold after 5 Hz smoothing), timing
error ≤ 0.145 s — no hidden 100 Hz assumptions, no crashes/NaN.

WESAD known-groups (15 recordings, 4-min windows, identical formulas):

| metric (stress − baseline) | baseline | current | direction |
|---|---|---|---|
| HR | +16.75 bpm | **+12.0 bpm** | preserved (attenuated by NEW-C1/BUG-NEW-03: S3_baseline HR 55.25→110.25 bpm) |
| SCR count | +8.0 /min | +8.0 /min | preserved (bit-identical) |
| RR | +4.638 brpm | +4.638 brpm | preserved (bit-identical) |

### Respiration — BIDMC (12/12 ok, seed 42, 0.5 s tolerance)

Mean cycle F1 **0.9437 → 0.9437 (Δ 0.0000)** — every rsp metric (P/R/F1/TP/FP/FN/timing) byte-identical on all 12 records:

| record | resp F1 (base = cur) | | record | resp F1 (base = cur) |
|---|---|---|---|---|
| bidmc01 | 0.988 | | bidmc07 | 0.984 |
| bidmc02 | 0.967 | | bidmc08 | 0.976 |
| bidmc03 | 0.975 | | bidmc09 | 0.987 |
| bidmc04 | 0.926 | | bidmc10 | 0.966 |
| bidmc05 | **0.658** (BUG-017, unchanged) | | bidmc11 | 0.945 |
| bidmc06 | 0.981 | | bidmc12 | 0.973 |

Contract checks (`rsp_contract.csv`, all PASS): sampling_rate respected at
25/50/100 Hz (59 cycles at each rate, cross-rate timing deviation ≤ 0.04 s);
`max_breath_interval_sec` enforced (15 s intervals rejected at cap 12, accepted
at cap 20; absent key ≡ 20 — new default confirmed behaviorally); `precleaned`
honored with exact identity test. RSP sampling sweep (`rsp_sampling.csv`):
P 1.0 / R 0.983 / F1 0.9916 at all rates (single structural FN: trailing peak
without successor); timing MAE 0.0088 s clean.
ECG-channel counts changed on 4 records (Wave B/BUG-NEW-03 territory):
bidmc01 730→782, bidmc03 153→628 (BUG-020 anomaly gone), bidmc04 738→739,
bidmc06 656→812.

### PPG — wrist-ppg-exercise (19/19 ok)

All PPG/HR metrics **byte-identical** to baseline: HR MAE overall **20.75 bpm**;
walk 15.88 / run 32.90 / bike-low 14.73 / bike-high 20.27; motion 23.61 vs
stationary 16.81; s1_walk bias +32.96 bpm (BUG-016 documented limitation,
persists exactly). The wrist **ECG** channel moved (BUG-NEW-03 signature):
s5_low_resistance_bike F1 0.716→0.997, s6_low_resistance_bike 0.657→0.736
(BUG-012), s2_walk 0.895→**0.688**, s6_walk 0.880→0.714, s8_walk 0.793→0.673,
s1_walk 0.667→0.579.

### rPPG — SCAMPS (30/30 ok, seed 0)

Unchanged-adapter rerun: **510/510 metric rows bitwise identical to baseline**
(HR MAE: green 11.39, chrom 11.82, pos 16.19 bpm; peak F1 green 0.245 /
chrom 0.768 / pos 0.540 — BUG-005's harm still fully visible on the raw path).
Polarity matrix (new analysis, 10-video means; `rppg/polarity_matrix.csv`):

| algorithm | mode | corr vs d_ppg | peak F1 | HR MAE |
|---|---|---|---|---|
| green | normal (default) | −0.921 | 0.245 | 11.39 |
| green | inverted (contract-correct) | +0.921 | **0.918** | 12.44 |
| green | auto | −0.921 (wrong) | 0.245 | 11.39 |
| chrom | normal (contract-correct) | +0.362 | 0.768 | 11.82 |
| chrom | inverted | −0.362 | 0.803 | 13.93 |
| chrom | auto | −0.432 (wrongly flipped) | 0.723 | 14.16 |
| pos | normal (default) | −0.823 | 0.540 | 16.19 |
| pos | inverted (contract-correct) | +0.823 | **0.920** | **7.90** |
| pos | auto | −0.823 (wrong) | 0.540 | 16.19 |

Exact sign flip verified (30/30 pairs, max |a+b| = 0). AutoDetect agrees with
the correlation-maximizing convention in only **1/30** cases (BUG-NEW-10).

### HRV — autonomic-aging (12/12 ok, seed 0)

Policy matrix (`hrv/policy_matrix.csv`, 41 rows): all 5 correction policies
semantically correct against independent reimplementations; artifact bounds
exact (299→artifact, 300→valid, 2000→valid, 2001→artifact); isolated ectopy
caught; sustained bigeminy **not** caught (BUG-NEW-02).

RMSSD strata medians (young/middle/old, ms):

| path | young | middle | old | monotone? |
|---|---|---|---|---|
| baseline (raw) | 52.3 | 24.9 | 142.8 | no |
| current raw (baseline methodology) | 56.3 | 24.9 | 174.4 | no |
| current corrected `reject_invalid` (NEW methodology) | 54.2 | **18.2** | **89.5** | no |

Correction halves the old-stratum inflation (0554: 315.6 → 224.1 raw → 102.2
corrected) but the trend remains non-monotone. Records 0342/0637 raw-path
changes are detector-driven (BUG-NEW-03), not architecture-driven: 8/12 records
reproduce baseline RMSSD bit-exactly (the `hrv` op itself is numerically
unchanged).

## Regression Analysis

Every material regression is in `regressions.csv` (53 rows). **All 53 rows
trace to a single mechanism — BUG-NEW-03** (the SPKI-clamp threshold-floor
change in `src/ecg/peaks.rs:247-248,278,296`): confidence high = 43 rows,
medium = 10. Regressions were investigated as carefully as improvements; the
record-228 cost analysis below shows the *intended* improvement is real on its
target, and the diagnostic shows the fleet damage is an operating-point
problem, not a failure of the 228 mechanism fix.

### The intended improvement is real on its target (record 228 cost analysis)

Detections on 228 rose 1244 → 2083 (+839). Of these, **+815 are true
positives** (recovered beats) and only **+24 are false positives**; PVC recall
is 1.000 (362/362); the 33 FPs are mostly isolated (one 6-FP cluster in a
noise-burst-like segment at t≈1499.6–1506.8 s). On record 228 in isolation the
trade is overwhelmingly favorable. The damage is on the neighbors and the
fleet:

| neighbor | baseline F1 | current F1 | delta | baseline FP/FN | current FP/FN |
|---|---|---|---|---|---|
| 221 | 0.9992 | 0.9994 | +0.0002 | 0/4 | 0/3 (unchanged) |
| 222 | 0.9917 | 0.9395 | −0.052 | 11/30 | 251/61 **regressed** |
| 223 | 0.9981 | 0.9983 | +0.0002 | 0/10 | 0/9 (unchanged) |
| 231 | 0.9994 | 0.7053 | −0.294 | 1/1 | 819/269 **severe** |
| 232 | 0.9992 | 0.6491 | −0.350 | 2/1 | 1664/125 **severe (det 3319 ≈ 1.86× n_ref; TP actually fell 1779→1655)** |
| 233 | 0.9998 | 0.9998 | 0.0000 | 0/1 | 0/1 (unchanged) |

### Key diagnostic (documented as diagnostic, NOT a proposed fix)

Re-running with `threshold_multiplier = 0.5` (default 0.25, verified bit-for-bit)
through the bridge **recovers every regressed record probed** — 100 → F1 1.000,
117 → 1.000, 232 → 0.998 (FP 1), 231 → 0.999, 123 → 0.989, 108 → 0.922 —
**while 228 stays fixed** (F1 0.9966) and wrist s6 reaches F1 0.919 with no
blackout. Interpretation: the SPKI clamp (`y_eff = y_val.min(spki)`) keeps the
adaptive threshold near the noise floor on real ECG, so T-waves and secondary
humps exceed THRESHOLD I and pass the 200 ms refractory. The fleet-wide
regression is an **operating-point problem**, not a failure of the 228
mechanism fix.

### Regression inventory (record structure: Dataset / Record / Metric / Baseline / Current / Delta / Likely mechanism / Confidence)

**Fleet ECG — mitdb per-record beat F1** (mechanism: T-wave/threshold-floor
over-detection from SPKI clamp; confidence high; full precision/recall/FP/FN in
`record_results.csv`):

| record | baseline | current | delta | | record | baseline | current | delta |
|---|---|---|---|---|---|---|---|---|
| 123 | 0.999 | 0.132 | −0.867 | | 231 | 0.999 | 0.705 | −0.294 |
| 108 | 0.885 | 0.215 | −0.671 | | 115 | 1.000 | 0.818 | −0.182 |
| 117 | 1.000 | 0.380 | −0.620 | | 222 | 0.992 | 0.940 | −0.052 |
| 100 | 1.000 | 0.409 | −0.591 | | 220 | 1.000 | 0.956 | −0.044 |
| 232 | 0.999 | 0.649 | −0.350 | | 103 | 0.999 | 0.963 | −0.036 |
| 113 | 1.000 | 0.666 | −0.333 | | 207 | 0.919 | 0.895 | −0.024 |
| 111 | 0.999 | 0.705 | −0.295 | | 106 | 0.999 | 0.976 | −0.023 |
| | | | | | 219 | 0.998 | 0.978 | −0.020 |

Doubling signature (n_detected ≈ 2× n_ref): 117 (2.22×), 123 (2.08×),
113 (1.99×), 232 (1.86×), 111 (1.84×), 100 (1.70×) — matching Wave C's
plot-confirmed R+T double detection on WESAD. Aggregate rows: mean F1
0.982→0.902; pooled FP 698→17,884; pooled FN 2,492→5,242; mean HR MAE
0.93→8.61 bpm. Also 104-V2 0.982→0.9677 (below plan bar 0.980).

**nstdb mid-SNR degradations** (same mechanism; FP inflation at every non-24 dB
level): 118e00 0.831→0.820; 118e06 0.884→0.842; 118e12 0.959→0.888;
118e18 0.999→0.979 (below plan bar 0.990); 119e00 0.787→0.771. Confidence high.

**Wrist ECG channel** (chest ECG @256 Hz; same mechanism; from the Wave D
results-vs-baseline join): s2_walk 0.895→**0.688** (FP 101→462), s6_walk
0.880→0.714, s8_walk 0.793→0.673, s1_walk 0.667→0.579,
s2_low_resistance_bike 0.982→0.905, s1_high_resistance_bike 0.998→0.950,
s2_high_resistance_bike 0.972→0.944, s6_run 0.670→0.641, s8_run 0.695→0.683,
s3_walk 0.968→0.957; s6_low_resistance_bike precision 0.708→0.595 (recall
improved 0.614→0.966 — blackout fix works; FP cost is the regression).

**BIDMC ECG / cross-channel consistency** (same mechanism on ICU lead II
@125 Hz): bidmc06 HR-consistency MAE 0.54→**30.24 bpm** (ECG peaks 656→812 =
101.5/min vs PPG 81.8/min); bidmc01 0.52→8.00 bpm (730→782); bidmc04
1.69→2.03 bpm (mild, confidence medium). bidmc03 153→628 is an *improvement*
(BUG-020), not a regression.

**WESAD chest ECG @700 Hz**: S3_baseline peaks 221→**441** (HR 55.25→110.25
bpm at rest; plot shows R+T double detection, 97% short-then-long RR
alternation), S3_amusement 217→430, S2_baseline 287→298, S2_stress 307→309,
S4_amusement 250→256; derived known-groups HR delta attenuated +16.75→+12.0 bpm
(direction preserved; stress medians unchanged at 79.0).

**Autonomic-aging** (detector-driven, not HRV-architecture-driven): 0342 peaks
478→955 (raw RMSSD 64.5→**634.4** ms), 0637 peaks 442→1081 (raw RMSSD
153.9→355.7 ms; 379/1080 intervals flagged artifact; corrected 185.7 ms on 409
NN is dominated by detector behavior — treat with caution), 0554 peaks 933→998
(mild; raw RMSSD 315.6→224.1).

**In-vitro mechanism reproduction** (`ecg/regression_matrix.csv` case 4):
narrow biphasic QRS double-detects both phases (det/truth = 2.00×, precision
0.50, F1 0.667) at 250 Hz; fs-sweep shows rate dependence (128 Hz 2.05×, 250 Hz
1.54×, 360/500 Hz clean on the pure synthetic — real 360 Hz records still fail,
i.e. real T-waves, not just biphasic R, cross the lowered threshold).
**Upstream test gap:** the shipped 6-case regression matrix uses count-only
assertions (`count >= 7`) with no FP/precision bound, so case 4 passes upstream
while double-detecting 2× — the fixtures cannot catch this class of regression
(remaining concern under BUG-NEW-03).

No other metric regressed beyond tolerance anywhere in the wave CSVs: all BIDMC
rsp metrics, all wrist PPG/HR metrics, all wearable-exam-stress BVP metrics,
all WESAD EDA/RSP/BVP metrics, all 510 SCAMPS rows, and 8/12 aging records are
byte/bit-identical to baseline (each wave's own status column).

### Cross-wave consistency notes (discrepancies found during consolidation)

1. **mitdb pooled-count artifact (Wave B):** `ecg/mitdb_records.csv` row
   `ALL_pooled` reports fractional counts (FP 698.25→17883.98, FN
   2491.85→5241.98) because the pooling script summed the `ALL` mean-row into
   the counts. True per-record sums are FP 684→17,519 and FN 2,441→5,135. The
   headline 698→17,884 / 2,492→5,242 (waveB-notes, and used above) follow the
   CSV; the 25.6× FP ratio is identical either way, and pooled F1
   (0.9856→0.9021) is exact. Flagged for correction; conclusions unaffected.
2. **Wave C notes vs CSV row counts:** waveC-notes describe
   `dataset_results_eda.csv` as "302 rows + 8 known-groups" with 251 unchanged /
   34 changed / 17 new; the committed CSV has **358 rows**: 288 unchanged, 36
   changed, 17 new, 17 improvement (recording_status rows). The CSV is
   authoritative (the notes appear to predate a CSV regeneration that split
   out the recording_status improvement rows and folded in the 8 known-groups
   rows); all substantive claims (0 lost metrics, all changes = WESAD chest-ECG
   + derived hr_*) check out against the CSV.
3. **Rounding:** baseline digest cites WESAD RR known-groups delta +4.64 brpm;
   CSV value is 4.638 (used above). Digest cites mitdb timing MAE 4.6 ms; CSV
   4.57 ms baseline → 9.21 ms current.
4. **BUG-011 baseline recall:** plan cites 0.602 normal-beat recall; baseline
   was symbol-agnostic (0.6016 all-beat). Class-conditional baseline was never
   recorded — the comparison uses the plan's cited number (Wave B limitation).
5. **"12 records" vs 15:** waveB-notes and `ecg/bug_verdicts.csv` say "12/48
   mitdb records materially regressed", but the notes' own list and the
   underlying `mitdb_records.csv` give **15** records at the stated
   delta_f1 < −0.01 threshold. This report uses 15 (CSV-derived).
6. **BUG-NEW-02 counts across waves:** Wave A saw 2/20 flagged on their
   bigeminy series, Wave F 1/60 on theirs — different crafted series, same
   mechanism (both flags are edge-window artifacts). Consistent.
7. **BUG-013 plan bar:** V2 F1 0.9677 is marginally below the plan's 0.980 bar
   (baseline 0.982) while the V5 symptom is resolved — recorded as
   RESOLVED-symptom with the caveat, per Wave B.

## Remaining Bugs

Verdict scale used above: RESOLVED / PARTIALLY RESOLVED / NOT RESOLVED / NOT
REPRODUCED / INCONCLUSIVE / DATASET INACCESSIBLE. Remaining original findings
(5): **BUG-005** (P2 — polarity contract delivered, default/auto still wrong),
**BUG-012** (P1 — blackout gone, F1 bar missed), **BUG-017** (P3 — prominence
gating claimed but absent), **BUG-018** (P2 — architecture exists; default
uncorrected; bigeminy evasion), **BUG-020** (P3 — anomaly gone; docs stale).
(BUG-015 moved to RESOLVED: question conclusively answered as reference-side,
no Lamina-side change.) New issues, normalized
IDs (wave-local IDs mapped in `bugs_confirmed.csv`):

### BUG-NEW-03 — Fleet-wide ECG over-detection from SPKI clamp — **P0**

- **Evidence:** `ecg/mitdb_records.csv` (15/48 records with delta F1 < −0.01; pooled FP 698→17,884; doubling signature on 100/111/113/117/123/232), `ecg/nstdb_results.csv`, `ecg/regression_matrix.csv` case 4, `ecg/wrist_s6_bins.csv`; cross-wave: WESAD S3 plot (`eda/plots/s3_baseline_ecg_peaks.png`), BIDMC `rsp_ppg/results/bidmc`, aging `hrv/aging_results.csv` notes; consolidated in `regressions.csv` (53 rows).
- **Reproduction:** `python -m validation run --dataset mit-bih-arrhythmia --seed 42` → record 123 F1 0.132 (baseline 0.999); or bridge `ecg-peaks` defaults on record 100/117/123 → detections ≈ 1.7–2.2× reference. In-vitro: regression-matrix case 4 construction at fs 128/250.
- **Affected Component:** `src/ecg/peaks.rs` (SPKI clamp l.247-248, searchback/running clamps l.278,296) — Pan-Tompkins adaptive threshold operating point.
- **Recommended Investigation:** redesign the operating point post-clamp; the threshold_multiplier=0.5 diagnostic (recovers all regressed records while 228 stays fixed at 0.9966, and wrist s6 reaches 0.919 with no blackout) localizes the problem to the default threshold level, not the clamp mechanism itself. Also add precision/FP bounds to the upstream 6-case regression fixtures (count-only asserts are blind to this class). **Blocks ship.**

### BUG-NEW-02 — Sustained bigeminy evades the new HRV interval classifier — **P1**

- **Evidence:** `hrv/policy_matrix.csv` (strictly alternating 440/830 ms bigeminy → 59/60 `normal_nn`; the single flag is an edge-window artifact); band sweep (no `percent_threshold` separates bigeminy from clean sinus jitter); record 0554 retains 199/599 NN intervals <550 ms after `reject_invalid`; cross-confirmed by Wave A (2/20 flagged on their series).
- **Reproduction:** bridge `hrv-correct` on a 60-interval alternating 440/830 ms (±3 ms jitter) series → 1/60 ectopic.
- **Affected Component:** `src/hrv/quality.rs` `classify_intervals` — rolling window `i-2..i+2` **includes the interval itself**, so in a strictly alternating pattern the median equals the interval's own parity class and the deviation test measures jitter (0.07–0.30%), not ectopy.
- **Recommended Investigation:** exclude self from the rolling-median window or add a global-distribution/bimodality check. Isolated couplets ARE caught (6/6); the failure is specific to sustained periodic ectopy — the exact pattern REMEDIATION_PLAN §4.4 was built for.

### BUG-NEW-14 — Protobuf transport gaps (3 items) — **P1**

- **Evidence:** `cross_language_results.csv`, `cross_language_report.md`.
- **Items:** (1) `CorrectionPolicy::PercentThreshold(f64)` parameter cannot be transported (proto `CorrectionPolicyKind` is a bare enum; the f64 silently disappears); (2) `RppgSignal.algorithm: RppgAlgorithmId` has no proto field/enum; (3) per-sample `timestamps_sec` and per-segment `quality` on `BvpWaveform`/`RppgSegment` have no proto representation (non-uniform rPPG timing does not round-trip).
- **Affected Component:** `sensor_messages` schema (`sensor-messages.proto`) vs `src/hrv/quality.rs:44`, `src/rppg/signal.rs:264-297`.
- **Recommended Investigation:** add a param field to the policy message, an rPPG algorithm enum, and optional repeated timestamps/quality fields. All other round-trip aspects are clean (6/6 legs byte-identical, f64 bit-exact incl. NaN/subnormals, 217/217 field tags consistent).

### P2 new issues

| ID | Issue | Evidence | Reproduction | Affected component / recommended investigation |
|---|---|---|---|---|
|BUG-NEW-01|`CorrectionPolicy::InterpolateCubic` executes the linear code path (bit-identical to linear; true cubic differs by up to 17.32 ms on the probe series)|`hrv/cubic_vs_linear.csv`; `bugs/evidence/bug018_output.txt`| bridge `hrv-correct` with `correction_policy=interpolate_cubic` vs `interpolate_linear` on any ectopy-containing RR series → outputs bit-identical |`src/hrv/quality.rs:157` shared match arm. Implement a real cubic spline or rename/document; rustdoc + plan §4.4 currently overpromise.|
|BUG-NEW-04|Wrong-variant error pattern (`NonFiniteInput` for finite-but-out-of-range config) persists in **6 validators**, incl. the new hrv code (`percent_threshold` ∉ (0,1) at `src/hrv/quality.rs:138`)|`api/api_results.csv` (45 error_correct=no rows: 21 P2 + 24 P3); Wave G F1| bridge `hrv-correct` with `percent_threshold=1.5` (finite, out of range) → `NonFiniteInput`; same pattern at the other 5 listed validators |`src/eda/peaks.rs:71,74`; `src/ppg/peaks.rs:117`; `src/rsp/peaks.rs:118`; `src/rppg/config.rs:52,134,140`; `src/signal/peaks.rs:73`; `src/hrv/quality.rs:138`. Add a dedicated `InvalidParameter` variant to `SignalError`.|
|BUG-NEW-05|`InvalidWindowSize(0)` mislabels non-window parameters and echoes a value never supplied (e.g. `min_breath_interval_sec=10 > max=1` → "Invalid window size: 0")|Wave G F2; `api/api_results.csv`| bridge `rsp-cycles` with `min_breath_interval_sec=10, max_breath_interval_sec=1` → "Invalid window size: 0" |`src/rsp/peaks.rs:108-115`, `src/eda/peaks.rs:77-87` (P2); ecg/ppg/rppg window-ish params (P3). Echo the actual parameter name/value.|
|BUG-NEW-06|`sample_entropy` accepts m=0 (mathematically undefined) and returns a plausible finite 2.1487|Wave G F3; `api/api_results.csv`| bridge `sample-entropy` with `m=0` → returns finite 2.1487 instead of an error |`src/complexity/entropy.rs` — validate m ≥ 1 per its own rustdoc.|
|BUG-NEW-10|`SignalPolarity::AutoDetect` skewness heuristic backwards: flips contract-phase signals, keeps intensity-phase ones; 1/30 agreement with ground truth; chrom harmed if trusted (F1 0.768→0.723, HR MAE 11.82→14.16)|`rppg/polarity_matrix.csv`, `rppg/waveE-notes.md`, morphology plots| bridge `rppg-polarity` with `polarity=auto_detect` on SCAMPS contract-phase waveforms → returns flipped phase in 29/30 cases |`src/rppg/signal.rs:558` `compute_should_flip`: flip when skew < −0.3 (or use Elgendi peak-prominence asymmetry); add realistic-morphology regression coverage.|
|BUG-NEW-15|Proto dead enums (`IntervalQualityKind`/`CorrectionPolicyKind` referenced by no message field; Lamina `BeatQuality` has no proto counterpart) + Dart diverts unknown enum values to `unknownFields` (typed getter returns default 0) while Rust/Python expose the raw value|`cross_language_results.csv`; Wave H findings 4–6| inspect `sensor-messages.proto` (no message field references either enum); decode bytes carrying an unknown enum value in Dart → typed getter returns default 0, raw value diverted to `unknownFields` |`sensor-messages.proto`; Dart generated bindings. Wire the enums into messages; document the Dart asymmetry.|

### P3 new issues

| ID | Issue | Evidence | Reproduction | Affected component |
|---|---|---|---|---|
|BUG-NEW-07|Silent edge-value acceptance: `ppg-peaks` alpha=0/1.5 accepted; `hrv-correct` classify_threshold ≤0 or >1 accepted (threshold 0/−0.1 classify everything ectopic on a clean RR series)|Wave G F4; `api/api_results.csv`| bridge `ppg-peaks` with `alpha=0` or `1.5` → accepted silently; bridge `hrv-correct` with `classify_threshold=0` → every interval of a clean RR series classified ectopic | `src/ppg/peaks.rs` alpha validation; `src/hrv/quality.rs` threshold validation |
| BUG-NEW-08 | fs=1e9: silent degenerate filter output — `ppg-clean`/`eda-clean`/`filter` lowpass return ≈0 (max\|out−in\|=0.998 on a 5 Hz sine); biquad coefficients degenerate at normalized cutoff ~1e-8; no upper fs sanity check | Wave G F5; `api/api_results.csv` | bridge `ppg-clean` / `eda-clean` / `filter` (lowpass) at `fs=1e9` on a 5 Hz sine → output ≈ 0 (max\|out−in\| = 0.998) | biquad coefficient computation (no upper-fs sanity check) |
|BUG-NEW-09|`NonFiniteInput` contract unreachable via the JSON bridge (NaN/±inf/null elements fail serde parsing → `bad_request` "invalid input JSON" without naming the field). Safe but poor diagnosis; harness/protocol-level note|Wave G F6| send any bridge op JSON containing NaN/±inf/null signal elements → `bad_request` "invalid input JSON", field not named | bridge serde parsing layer (harness/protocol level) |
|BUG-NEW-11|`eda-decompose` boundary transient creates spurious edge SCRs (1–2 FP at boundaries at every sampled rate; first wes event at onset_index=0)|`eda/eda_sweep.csv`, `eda/plots/eda_sweep.png`| bridge `eda-decompose` on any EDA session at any sampled rate → first SCR event at `onset_index=0`; 1–2 boundary FPs per rate in `eda_sweep.csv` | `src/eda/decompose.rs` boundary handling |
|BUG-NEW-12|`cargo clippy --all-targets --all-features -- -D warnings` fails on the pristine tree (unused import `tests/rppg_tests.rs:781`; `neg_multiply` `tests/ecg_tests.rs:376`); library target clean; fmt and 139/139 tests pass|stage-1 baseline verification| `cargo clippy --all-targets --all-features -- -D warnings` on pristine `fec2668` → 2 test-target errors | `tests/rppg_tests.rs:781`, `tests/ecg_tests.rs:376` |
|BUG-NEW-13|Dead `BeatQuality` enum (no public producer/consumer); `IntervalQuality::Missing` unreachable via the bridge; all-invalid `reject_invalid` surfaces bare `EmptySignal` instead of null metrics; no public `sdnn`|`hrv/bug_verdicts.csv` (BUG-OBS-01); Wave A| grep public API for a `BeatQuality` producer/consumer → none; bridge `hrv-correct reject_invalid` on an all-invalid RR series → bare `EmptySignal` | `src/hrv/quality.rs` public surface |

### Doc-drift / remaining-concern items folded under parent bugs (P3)

- **BUG-017**: REMEDIATION_PLAN Phase 2 claims prominence gating — absent from `src/rsp/peaks.rs`.
- **BUG-016**: motion-artifact caveat missing from signal-contracts.md §2.3 (present only in the validation report).
- **BUG-020**: `public-dataset-validation.md`/`BUGS.md`/`datasets.md` still describe the stale 153-vs-610 bidmc03 anomaly as a dataset issue.
- **BUG-008**: `total_feature_count` still a hardcoded literal 28 vs the plan's "dynamically calculated" claim; no pinning test.
- **BUG-013/011**: upstream 6-case ECG regression fixtures use count-only assertions (blind to FP regressions) — P2 framework gap, folded under BUG-NEW-03.
- **BUG-005**: default `polarity=normal` pass-through means the default BVP violates the §2.4 contract for green/pos.
- Wave G contract-doc notes (P3): signal-contracts.md §2.1 says `Fs ≥ 2·fcutoff` acceptable but every validator enforces *strictly less than* Nyquist; case sensitivity inconsistent (`ecg-clean` lowercases, rppg/filter kinds are case-sensitive); `hrv` peak-envelope silently dedups/reorders peaks; `hrv-correct` missing-fields blame ordering.
- Wave H P3 tail: absent proto polarity ≠ Lamina default `Normal`; proto `polarity` fields have no Lamina stored-state counterpart; ARCHITECTURE_REPORT drift (oneof list omits `bvp_waveform=22`; Python "float" mislabel); prost variant-name prefix cosmetics.
- Wave A N4 (test-asset nit): `repro_ecg_001_method_ignored.py` prints a stale hardcoded "CONFIRMED" conclusion line although its own output shows the bogus method now rejected.

## Recommended Remediation Priorities

1. **P0 — BUG-NEW-03 (blocks ship):** redesign the SPKI-clamp operating point in `src/ecg/peaks.rs`. The Wave B multiplier-0.5 diagnostic (recovers 100/117/123/108/231/232/222 while 228 stays fixed at 0.9966 and wrist s6 reaches F1 0.919 with no blackout) localizes the defect to the default threshold level, not the 228 clamp mechanism. Add FP/precision bounds to the upstream regression fixtures. Re-run mitdb/nstdb/wrist/BIDMC/WESAD/aging after any change.
2. **P1 — BUG-NEW-02:** fix the self-inclusive rolling-median window (exclude self, or add a global-distribution check) so sustained bigeminy is classified; re-run the policy matrix and aging corrected path.
   **P1 — BUG-NEW-14:** extend the proto schema for the `PercentThreshold` parameter, `RppgAlgorithmId`, and per-sample timestamps/segment quality.
3. **P2:** BUG-NEW-01 (real cubic spline or honest rename), BUG-NEW-04 (dedicated `InvalidParameter` variant), BUG-NEW-05 (echo actual parameter/value), BUG-NEW-06 (validate m ≥ 1), BUG-NEW-10 (invert the AutoDetect skewness rule; add realistic-morphology coverage), BUG-NEW-15 (wire dead proto enums; document Dart asymmetry). Also BUG-005 follow-through: make the documented polarity convention achievable by default once AutoDetect is fixed.
4. **P3:** BUG-NEW-07/08/09/11/12/13, the BUG-017 prominence gating (or correct the plan/docs), and the doc-drift items listed above (bidmc03 stale docs, signal-contracts §2.3 motion caveat, BUG-008 dynamic-denominator claim, Nyquist-inequality wording).

**Statement of scope:** this revalidation implemented **NO fixes**. All work was
validation-only; every artifact above was produced by running the existing and
extended validation harness against the unmodified post-remediation crate.
