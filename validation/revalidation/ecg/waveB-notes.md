# Wave B — ECG Revalidation Notes (post-remediation `fec2668`, bridge `caa7484`)

## Methodology

- Environment: Rust stable 1.98.1 (rsproxy), Python 3.x with numpy/scipy/pandas/
  wfdb/matplotlib; bridge built release (`lamina_bridge`, lamina 0.1.0).
- Data: prefetched tarballs extracted at adapter roots; counts verified
  (mitdb 48, nstdb 7, wrist 19).
- Runs: `python -m validation run --dataset mit-bih-arrhythmia --seed 42` and
  `--dataset mit-bih-noise-stress --seed 42`, 48/48 and 7/7 records ok.
  Matching: unchanged greedy one-to-one, 150 ms tolerance. Adapter configs
  untouched; bridge ecg ops unchanged (`ecg-clean "none"→""` mapping verified
  to hit the same 0.5 Hz high-pass path as baseline; my direct bridge probe on
  record 228 reproduces the suite output exactly: F1 0.9913, TP 2050/FP 33/FN 3).
- Direct-probe scripts (228 deep-dive, wrist s6, regression probes, fs sweep,
  threshold diagnostics) replicate the adapter call path
  (`bridge.ecg_peaks`, defaults) and reuse `validation.metrics.events.match_events`.

## BUG-011 — record 228 tall-PVC under-detection

- **Original finding:** recall 0.6016 / F1 0.7492 (TP 1235/FP 9/FN 818);
  SPKI inflated by tall PVCs → subsequent normal beats below threshold.
- **Previous evidence:** baseline metrics.csv; REMEDIATION_PLAN Phase 4
  (normal-beat recall bar ≥ 0.950).
- **Current implementation:** SPKI/NPKI clamping in `src/ecg/peaks.rs`
  (`y_eff = y_val.min(spki)` l.247, searchback `sb_val.min(2.5*spki)` l.278,
  `y_val.min(2.5*spki)` l.296).
- **Validation performed:** full suite + direct probe; class-conditional recall
  (N 0.9982, V 1.0000, other 1.0000); neighbor scan 221/222/223/231/232/233
  (226 not in the 48-record MIT-BIH set); threshold diagnostic.
- **Result:** target symptom RESOLVED (F1 0.9913, plan bar met, +815 TP at
  +24 FP cost, no PVC loss). FN remainder: 3 normal beats at 615.8/1218.2/1475.4 s.
- **Remaining concern:** same change severely regresses neighbors 231/232/222
  and 9 other records (see BUG-NEW-B1). The fix mechanism is sound for 228 but
  the resulting operating point is too permissive fleet-wide.

## BUG-012 — wrist s6 blackout

- **Original finding:** ~90 s detection blackout (bins [66,2,0,3,...]),
  F1 0.657 / recall 0.614.
- **Current implementation:** same SPKI clamp + 6-case regression fixtures
  (`tests/ecg_tests.rs::test_ecg_6case_regression_matrix`).
- **Validation performed:** adapter-identical direct probe; baseline binning
  reverse-engineered (`np.histogram`, `linspace(0,280,10)`); longest-gap scan;
  threshold_multiplier sensitivity (diagnostic only).
- **Result:** blackout ELIMINATED (longest gap 0.68 s, recall 0.966) but plan
  pass bar F1 ≥ 0.900 NOT met (F1 0.736, 353 FP, precision 0.595) →
  **PARTIALLY RESOLVED**; residual FP over-detection is BUG-NEW-B1's signature.

## BUG-013 — paced-beat lead sensitivity (documentation claim)

- **Original finding:** mitdb/104 V5 F1 0.683 vs V2 0.982.
- **Validation performed:** record 104 both channels through bridge; synthetic
  paced-spike probe (regression matrix case 5).
- **Result:** V5 F1 0.9744, V2 F1 0.9677 — gap closed, synthetic paced case
  perfect. **RESOLVED** (symptom). Caveats: V2 marginally below plan bar
  (0.9677 < 0.980, baseline 0.982); FP inflation on both leads (V5 FP 2→98)
  consistent with BUG-NEW-B1. Note: 104's V5 improvement is partly the same
  low-threshold regime (at multiplier 0.5, 104 falls back to F1 0.63).

## BUG-NEW-B1 — fleet-wide ECG over-detection regression (P0) — NEW

Cross-references Wave C's BUG-NEW-C1 (T-wave double detection confirmed with
plot evidence on WESAD) and Wave F's autonomic-aging peak-count changes.

- **Evidence (mitdb, 48/48):** mean per-record F1 0.9820 → 0.9016
  (−0.080); pooled F1 0.9856 → 0.9021; pooled FP 698 → 17,884 (25×);
  pooled FN 2,492 → 5,242; mean HR MAE 0.93 → 8.61 bpm.
- **Material regressions (delta_f1 < −0.01):** 123 (−0.867), 108 (−0.671),
  117 (−0.620), 100 (−0.591), 113 (−0.333), 232 (−0.350), 111 (−0.295),
  231 (−0.294), 115 (−0.182), 222 (−0.052), 220 (−0.044), 103 (−0.036),
  207 (−0.024), 106 (−0.023), 219 (−0.020).
- **Material improvements:** 228 (+0.242, intended), 104 (+0.291, but FP 2→98).
- **Doubling signature (n_detected ≈ 2× n_ref):** 117 (2.22×), 123 (2.08×),
  113 (1.99×), 232 (1.86×), 111 (1.84×), 100 (1.70×) — matches Wave C's
  confirmed R+T double-detection mechanism.
- **In-vitro reproduction:** regression-matrix case 4 (narrow biphasic QRS)
  double-detects both phases (det/truth = 2.0×, precision 0.50) at fs 250;
  fs-sweep shows rate dependence (128 Hz 2.05×, 250 Hz 1.54×, 360/500 Hz
  clean on the pure synthetic — real 360 Hz records still fail, i.e. real
  T-waves, not just biphasic R, cross the lowered threshold).
- **Diagnostic (not a fix):** threshold_multiplier 0.5 recovers every regressed
  record probed (100 → 1.000, 117 → 1.000, 232 → 0.998, 231 → 0.999,
  123 → 0.989, 108 → 0.922) while 228 stays fixed (0.9966) and wrist s6 reaches
  F1 0.919 with no blackout. The default multiplier is 0.25 (verified: 0.25
  run reproduces default output bit-for-bit on s6).
- **Upstream test gap:** the shipped 6-case regression matrix uses count-only
  assertions (`count >= 7` etc.) with no FP/precision bound — case 4 passes
  upstream while double-detecting 2×; the fixtures cannot catch this class of
  regression.

## Six-case regression matrix (task §7)

- Upstream: `cargo test --release --test ecg_tests` → **9/9 pass** (verbatim:
  "test result: ok. 9 passed; 0 failed"), including
  `test_ecg_6case_regression_matrix ... ok`.
- Independent bridge probes (same constructions, ground-truth matched @150 ms):
  cases 1/2/3/5 perfect (P=R=F1=1.0, 10 s and 60 s extended); case 4
  double-detects (F1 0.667, precision 0.5, det 2× truth) — upstream assert
  blind to it; case 6 passes with 2 FPs from the noise burst.
  Full table: `regression_matrix.csv`.

## Sampling-rate sweep (task §8)

- Monophasic QRS: P=R=F1=1.0 at 128/250/360/500 Hz; timing MAE ≈ 39–40 ms
  constant (equals the 40 ms ground-truth-vs-pulse-center convention offset;
  no hidden rate assumption in timing).
- Biphasic QRS: rate-dependent double-detection (128 Hz F1 0.64, 250 Hz 0.79,
  360/500 Hz 1.00). See `ecg_sampling.csv`.

## NSTDB (task §6 noise robustness)

SNR gradient shape preserved but degraded at mid SNR: 0 dB 0.831→0.820,
6 dB 0.884→0.842, 12 dB 0.959→0.888, 18 dB 0.999→0.979 (**below** plan bar
0.990), 24 dB 1.000→1.000; 119e00 0.787→0.771, 119e24 1.000→0.996. FP inflation
at every non-24 dB level (e.g. 12 dB FP 170→493). Plan's 0 dB bar (≥0.830)
also marginally missed (0.820). `nstdb_results.csv`.

## New issues filed

- **BUG-NEW-B1 (P0):** fleet-wide ECG over-detection / threshold-floor
  regression (details above). Blocks treating the Phase-4 remediation as
  shippable as-is.
- **Upstream-fixture gap (P2, framework):** count-only assertions in
  `test_ecg_6case_regression_matrix` cannot catch FP regressions; recommend
  adding precision/FP bounds (observation only; no code changed).

## Limitations of this revalidation

- Baseline class-conditional recall for 228 was never recorded (baseline was
  symbol-agnostic); comparison uses the plan's cited 0.602 normal recall.
- Record 226 does not exist in the MIT-BIH 48-record set; neighbor scan covers
  221/222/223/231/232/233.
- Wrist baseline timing MAE was not recorded; only current (34.8 ms) available.
