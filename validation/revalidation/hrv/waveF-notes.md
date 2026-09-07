# Wave F — HRV / Ectopy Architecture Revalidation (Lamina @ fec2668, bridge caa7484)

Revalidation branch `reval/hrv`. All probes drive the compiled Rust bridge
(`validation/lamina_bridge`, op `hrv-correct`); Lamina source untouched.
Scripts: `policy_matrix.py`, `cubic_vs_linear.py`, `aging_revalidation.py`
(this directory). Data: autonomic-aging 1.0.0 prefetched to `$HOME/data`
(`LAMINA_DATA_DIR=$HOME/data`).

## Original Finding (BUG-018)
Pre-remediation, the `hrv` bridge op computed RMSSD/mean-NN over **all**
detected RR intervals with no ectopy/artifact filtering. On autonomic-aging
this confounded the population trend: RMSSD medians young/middle/old =
52.3/24.9/142.8 ms (physiology expects monotone decrease); record 0554
(sustained ventricular bigeminy) showed RMSSD 315.6 ms.

## Previous Evidence
- `reval-baseline/autonomic_metrics_extra.csv`: per-record RMSSD/mean-NN via
  bridge `hrv` op on Lamina ECG1 peaks; record 0554 rmssd_ms = 315.59.
- `docs/validation/REMEDIATION_PLAN.md` §4.4: five-stage decoupled pipeline
  design (BeatQuality → IntervalQuality → CorrectionPolicy → NN → hrv).
- `docs/architecture/signal-contracts.md` §2.7: operational rule — callers
  must pass intervals through `clean_rr_intervals`.

## Current Implementation (post-remediation)
- `src/hrv/quality.rs`: `BeatQuality` (dead enum — no public producer/
  consumer), `IntervalQuality`, `CorrectionPolicy` (None / RejectInvalid /
  InterpolateLinear / InterpolateCubic / PercentThreshold),
  `classify_intervals` (hardcoded 300–2000 ms artifact bounds + 20%
  self-inclusive rolling-median ectopy test), `clean_rr_intervals`.
- Bridge op `hrv-correct` (SPEC.md §A.2) chains
  peaks_to_intervals → classify_intervals → clean_rr_intervals →
  hrv_rmssd/hrv_mean_nn, returning per-interval `interval_quality` labels.
- `IntervalCleaningConfig` from the plan does **not** exist;
  `clean_rr_intervals` takes only `(rr, policy)`; artifact bounds are not
  configurable (SPEC §A.3).
- `CorrectionPolicy::InterpolateCubic` shares its match arm with
  `InterpolateLinear` (`src/hrv/quality.rs:157`) — one linear body.

## Validation Performed
1. **Policy semantics matrix** (`policy_matrix.csv`, 41 rows): 7 crafted RR
   series (clean 800±10 ms; single ectopic 500+1100 ms; continuous bigeminy
   440/830 ms; 150 ms artifact spike; 3500 ms gap; 3×250 ms burst; sinus
   arrhythmia ±80 ms) × 5 policies, plus 299/300/2000/2001 ms boundary
   probes, a percent-band probe, and a NaN probe. NN outputs cross-checked
   against independent NumPy reimplementations of each policy's documented
   semantics; RMSSD cross-checked against an independent implementation.
2. **Cubic scrutiny** (`cubic_vs_linear.csv`): sinus-arrhythmia series with
   a 3-interval artifact gap and a 2-interval ectopy gap in high-curvature
   regions (true cubic spline differs from linear by 8.6–17.3 ms at the
   filled samples); both policies run and compared sample-by-sample.
3. **Classification correctness**: per-interval labels checked against
   expectation for every crafted interval; artifact bound probes at
   299/300/2000/2001 ms in homogeneous neighborhoods (isolating the artifact
   rule from the ectopy rule).
4. **Dataset revalidation**: `python -m validation run --dataset
   autonomic-aging --seed 0` (12/12 ok, matches baseline record set) plus
   per-record recompute: same ECG1 peaks → `hrv` op (raw, baseline
   methodology) and `hrv-correct policy=reject_invalid` (NEW corrected
   methodology — did not exist at baseline; task §5 methodology change).
5. **Edge probes**: empty input → empty outputs + null metrics (per SPEC);
   single-valid interpolation → constant fill; all-invalid reject →
   `lamina_error` (EmptySignal); bogus policy / missing or out-of-range
   percent param (0.0, 1.0, −0.1) → `bad_request`; `classify_threshold`
   override verified (500 ms ectopic flagged at default 0.20, retained at
   0.50).

## Result
- **Policy semantics: PASS for all 5 policies.** `none` = exact passthrough
  (≤1 ulp JSON round-trip); `reject_invalid` removes exactly the non-
  `normal_nn` intervals; `interpolate_linear`/`interpolate_cubic` preserve
  length, keep valid samples bit-exact, linearly fill invalid runs with edge
  clamping; `percent_threshold(p)` applies the stated band (isolated 700 ms
  interval in an 800 ms series: rejected at p=0.05, retained at p=0.20).
- **Classification: PASS at boundaries and for isolated events.** 299→
  artifact, 300→valid, 2000→valid, 2001→artifact (strict `<300`/`>2000`
  rule). Ectopic couplet (500/1100) correctly `ectopic_rr`; 150/3500/250 ms
  correctly `artifact_rr`; sinus arrhythmia fully `normal_nn` and survives
  every policy intact (0 removed at p=0.20).
- **BUG-NEW-01 CONFIRMED (P2)**: `interpolate_cubic` is bit-identical to
  `interpolate_linear` (max diff 0.0 ms; RMSSD equal to 15 digits) on a
  series where a genuine cubic spline differs by up to 17.32 ms. Source:
  shared match arm `src/hrv/quality.rs:157`. Documented, not fixed.
- **BUG-NEW-02 CONFIRMED (P1)** — focused investigation of the sustained-
  bigeminy classifier failure (cross-confirmed by Wave A, who saw 2/20
  flagged on their bigeminy series):
  - **Which intervals get flagged**: in a 60-interval alternating 440/830 ms
    (±3 ms jitter) series, exactly **1/60** (index 58, an edge artifact of
    the asymmetric end-of-series window) is `ectopic_rr`; the other 59 are
    `normal_nn`.
  - **Why the classifier adapts to bigeminy as "normal"**: the rolling
    window `i-2..i+2` (5 samples) INCLUDES the interval itself, so in a
    strictly alternating pattern it always contains 3 same-parity and 2
    opposite-parity values; the (upper) median is therefore a same-parity
    value and the deviation test measures *jitter*, not ectopy — observed
    deviations 0.07–0.30% vs the 20% threshold.
  - **No `percent_threshold` band rescues it** (sweep on bigeminy ±3/±10 ms
    jitter vs clean sinus ±10 ms):

      | p | bigeminy jit3 | bigeminy jit10 | clean jit10 |
      |---|---|---|---|
      | 0.005 | 16 | 32 | 25 |
      | 0.01 | 5 | 25 | 9 |
      | 0.02 | 1 | 11 | 0 |
      | ≥0.05 | 1 | 1 | 0 |

      The classifier responds to jitter amplitude, not alternating
      structure; the bands that catch some bigeminy intervals (p≲0.02) also
      flag clean sinus jitter, and even then catch only a minority (11/60).
  - **Isolated couplets ARE caught**: three 500+1100 ms couplets embedded in
    a clean series → all 6 intervals correctly `ectopic_rr`. The failure is
    specific to *sustained periodic* ectopy.
  - **Real-data consequence (record 0554)**: after `reject_invalid`, 199/599
    retained NN intervals are still <550 ms; period-3 ectopy runs
    ([416, 840, 656] ms-type triplets) keep the median-matching phase
    labeled `normal_nn`. Corrected RMSSD 102.2 ms remains ~5× the middle-
    stratum median partly for this reason.
  - Severity raised to **P1**: the classify→correct architecture fails on
    the exact pattern (sustained bigeminy) that motivated it (BUG-018 /
    REMEDIATION_PLAN §4.4 success criterion "appropriately identifies and
    filters ectopic intervals on ... bigeminy recordings").
- **Aging trend**: corrected medians young/middle/old = **54.2/18.2/89.5**
  ms vs raw current 56.3/24.9/174.4 and baseline 52.3/24.9/142.8. Correction
  halves the old-stratum inflation (0554: 315.6 baseline → 224.1 raw current
  → **102.2 corrected**) but the trend is **still not monotone-decreasing**.
- **BUG-018 verdict: PARTIALLY RESOLVED.** A documented, user-visible
  quality/correction path now exists and works for isolated ectopy/artifacts;
  the default path (`hrv` op, and `hrv-correct` default policy `none`) is
  still uncorrected; sustained bigeminy/trigeminy partially evades
  classification, so the age trend remains non-monotone even after
  correction.

## Remaining Concerns / cross-wave notes
- **Detector confound (ECG wave territory)**: current-run peak counts differ
  from baseline on 4/12 records (0342 ECG1 478→955, 0637 442→1081,
  0554 933→998, 0276 581→583). Consequently raw RMSSD changed on those
  records independent of HRV architecture (0342: 64.5→634.4 ms raw — peaks
  roughly doubled; 0637: 153.9→355.7 raw with 379/1080 intervals flagged
  artifact, consistent with over-detection). The other 8 records reproduce
  baseline RMSSD bit-exactly, confirming the `hrv` op path itself is
  unchanged. Aging-trend interpretation must be read against the ECG wave's
  detector-change verdicts.
- 0637 corrected RMSSD (185.7 ms on only 409 NN intervals) is dominated by
  detector behavior, not physiology — treat with caution.
- `IntervalQuality::Missing` is unreachable through the bridge (JSON cannot
  carry NaN/null into `Vec<f64>`; peaks-derived intervals are always finite)
  — BUG-OBS-01 (P3).
- No public `sdnn` and no `BeatQuality` producer/consumer (SPEC §A.3) — the
  plan's five-stage pipeline is only realized from IntervalQuality downward.
- All-invalid input under `reject_invalid`/interpolation raises
  `SignalError::EmptySignal` (exit 1) rather than graceful null metrics;
  contract-compliant but worth documenting for consumers (BUG-OBS-01).
- Methodology note (task §5): baseline RMSSD used bridge op `hrv` on raw
  peaks; the corrected column uses the NEW `hrv-correct reject_invalid`
  architecture on the same detected peaks. Old vs new are labeled per-column
  in `aging_results.csv`.

## New issues (severity)
- P2 BUG-NEW-01 InterpolateCubic ≡ InterpolateLinear (see bug_verdicts.csv;
  independently byte-confirmed by Wave A).
- P1 BUG-NEW-02 self-inclusive-median classifier blind spot for sustained
  periodic ectopy; no percent_threshold band separates bigeminy from clean
  sinus jitter.
- P3 BUG-OBS-01 Missing-class unreachable via bridge; all-invalid reject
  surfaces bare EmptySignal error.

## Cross-wave confirmation
- Wave A (bug-matrix) independently found InterpolateCubic ≡ InterpolateLinear
  byte-identical and the sustained-bigeminy under-flagging (2/20 on their
  series vs 1/60 here — both edge-window artifacts), with corrected RMSSD
  still inflated (207 vs 228 ms raw), matching the 0554 result
  (102.2 vs 224.1 ms raw).
- Default path confirmed uncorrected: `op_hrv`
  (validation/lamina_bridge/src/main.rs:435) maps peaks → peaks_to_intervals
  → hrv_rmssd directly, no classify/clean step; 8/12 aging records with
  unchanged peak sets reproduce baseline RMSSD bit-exactly, so the `hrv` op
  itself is numerically unchanged.
