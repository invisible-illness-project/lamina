# Wave C — EDA / Autonomic Dataset Revalidation Notes

Post-remediation Lamina @ fec2668, revalidation bridge @ caa7484 (HEAD).
Validation-only: no Lamina source, tests, benches, bindings, or existing
validation methodology/code modified. All adapter runs use the original
commands, seed 0, tolerances unchanged (SCR onsets 1.0 s).

## Methodology

- Repo cloned to `$HOME/work` (FUSE/noexec on /mnt), branch `reval/eda`.
- Bridge built with `cargo build --release` (toolchain 1.98.1 via rsproxy).
- Data: prefetched copies under `$HOME/data` (`LAMINA_DATA_DIR=$HOME/data`;
  WESAD reassembled from parts, sha256
  `881cb03f…f507` verified; wearable-exam-stress and big-ideas extracted as
  documented).
- Reruns: `python -m validation run --dataset {wesad,wearable-exam-stress,big-ideas}
  --seed 0 --results-dir $HOME/reval-results/<dataset>` (identical to baseline
  invocation).
- Baseline: `/mnt/agents/output/reval-baseline/` (pre-remediation @ f3a195a).
- Deep-validation probes used the same bridge ops (`eda-clean`,
  `eda-decompose`, `eda-peaks`, `eda-clean-config`, `ppg-peaks`) directly;
  downsampling for BUG-014 was done harness-side with anti-aliased decimation
  (`scipy.signal.decimate`, FIR), adapter untouched.

## 1. Adapter rerun results (§4 / §16)

| dataset | baseline | current | delta |
|---|---|---|---|
| wearable-exam-stress | 15/30 ok, 15 EDA FAILED (BUG-003) | **30/30 ok** | +15 fixed, 0 regressions |
| big-ideas | 2/2 FAILED (dataset status failed) | **2/2 ok** | +2 fixed |
| wesad | 15/15 ok | **15/15 ok** | 0 status changes |

Metric-level comparison (`dataset_results_eda.csv`, 302 rows + 8 known-groups
rows):
- **251 unchanged** (bit-identical values), **17 new** (the 15 + 2 EDA
  recordings that previously errored), **34 changed**, 0 lost metrics.
- All 34 changed rows are WESAD chest-ECG peak counts (5 recordings) and the
  derived cross-device `hr_*` consistency metrics (6 recordings). All
  wearable-exam-stress BVP and all WESAD EDA/RSP/BVP metrics are bit-identical
  to baseline. See NEW-C1 below.

## 2. BUG-003 — 4 Hz EDA fs-floor — **FIXED**

- **Original finding:** `eda_clean` hardcoded a 5 Hz lowpass cutoff; any
  fs ≤ 10 Hz failed with `InvalidCutoffFrequency`; blocked all Empatica E4
  4 Hz EDA (15 wearable-exam-stress + 2 big-ideas recordings).
- **Previous evidence:** per-recording runner failures; synthetic repro
  (fs ∈ {4,8,10} fail; fs=16 ok).
- **Current implementation:** `EdaCleaningConfig` defaults
  `lowpass_cutoff_hz=Some(5.0)`, `pass_through_if_nyquist_violated=true`;
  `eda_clean_config` bypasses the low-pass when cutoff ≥ Nyquist
  (src/eda/clean.rs:100-107). `eda-clean-config` bridge op exposes the
  tri-state cutoff, filter order, and pass-through flag.
- **Validation performed:**
  - Adapter reruns: all 17 previously-failed recordings now succeed
    (`dataset_results_eda.csv` status=new).
  - Plain `eda-clean` repro sweep: fs=4/8/10 now ok (pass-through,
    rms_change ~1e-17); fs=16 applies the 5 Hz filter (rms_change 3.1e-4).
  - `eda-clean-config` paths at fs=4 (Nyquist 2.0) and fs=64 (Nyquist 32):
    (a) cutoff 1.0 Hz → filtered (filter_applied=true) both fs;
    (b) near-Nyquist 1.8 Hz @4 → filtered ok;
    (c) above-Nyquist 5 Hz @4 → **silent pass-through by default**
    (filter_applied=false, output == input); with explicit
    `pass_through_if_nyquist_violated:false` → `InvalidCutoffFrequency`
    error, exactly per SPEC A.2; at fs=64 the same 5 Hz cutoff filters
    normally;
    (d) explicit `lowpass_cutoff_hz:null` → no filtering at any fs.
    Full matrix in `eda_config_paths.csv`.
  - Deep-validation @4 Hz (§9): full raw → clean → decompose → peaks chain
    on 2 recordings per dataset (wes S1_midterm_1, S3_Final; big-ideas
    003/015, `eda_deepval_report.json`, `plots/*_chain.png`).
- **Result:** Pipeline completes AND produces scientifically plausible output
  at 4 Hz:
  - No exceptions, zero NaNs in cleaned/tonic/phasic/events.
  - Reconstruction invariant |tonic+phasic−clean| ≤ 4.4e-16 (matches
    signal-contracts.md invariant).
  - SCR rates: wearable-exam-stress 0.36–5.73/min (median 2.26, all 15
    sessions, computed against session durations); big-ideas 6.33/min (015)
    and 10.88/min (003) — inside/at the edge of the 0–10/min plausibility
    band for a 2 h free-living window.
  - Event structure sane: onsets strictly increasing, min ICI 0.5 s,
    median ICI 2.75–4.25 s, median amplitude 0.03–0.08 μS (floor ≈0.01 μS),
    median rise time 0.5–0.75 s (2–3 samples at 4 Hz — quantization-limited).
  - Morphology: tonic is smooth and slow (2nd-difference RMS ~250× smaller
    than cleaned signal), phasic zero-mean (|mean| ≤ 1e-4 μS) with ~50%
    negative samples; plots show SCR bumps riding a slow SCL.
- **Remaining concern:** none blocking. The default pass-through means 4 Hz
  EDA is **not low-pass filtered at all** by default (contract allows
  "bypassed or set to 1.8 Hz"; implementation chose bypass). This is
  documented behavior, not silent wrongness, but consumers should know
  high-frequency noise is retained at 4 Hz unless they pass an explicit
  cutoff via `eda-clean-config`.

## 3. BUG-014 — WESAD S3 chest EDA 700 Hz SCR anomaly — **RESOLVED (not a Lamina defect)**

- **Original finding:** S3 baseline 147 SCRs/4 min (36.8/min) vs S3 stress
  9.8/min; ambiguous whether 700 Hz rate-dependent over-segmentation or
  dataset characteristic.
- **Validation performed:** identical adapter windows for all 5 subjects ×
  {baseline, stress}; `eda-peaks` at native 700 Hz vs harness-side
  anti-aliased decimation to 10 Hz and 4 Hz (`bug014_scr_counts.json`,
  `plots/bug014_s3_baseline_700_vs_10.png`).
- **Result:**
  - S3_baseline: 147 (700 Hz) → 130 (10 Hz) → 49 (4 Hz). The 700→10 Hz
    counts are the same order (−12%): **event counts scale sanely; no
    700 Hz-specific explosion**. (4 Hz merges small bumps via smoothing —
    expected information loss, also visible in S6_baseline 42→34→2.)
  - Raw S3 baseline chest EDA shows periodic ~1 Hz conductance oscillations
    (~0.02 μS pk-pk) — respiration-coupled crosstalk on the chest strap.
    Phasic @700 Hz and @10 Hz overlap almost exactly; the bumps legitimately
    exceed the 0.01 μS amplitude threshold, so the detector counts them.
  - Group known-groups SCR medians unchanged vs baseline (baseline 0.5/min,
    stress 8.5/min, delta +8.0/min — direction preserved).
- **Remaining concern (P3):** `eda_findpeaks_events` has no periodicity/
  respiration-crosstalk gating, so ~1 Hz regular modulation on chest EDA is
  reported as SCRs. Dataset-facing caveat, not a rate-handling bug.

## 4. BUG-015 — wrist BVP HR vs Empatica HR.csv — **UNCHANGED (reference-side)**

- **Original finding:** Lamina-vs-E4 HR MAE 26–47 bpm on wearable-exam-stress;
  ambiguous Lamina defect vs E4 reference error.
- **Validation performed:** rerun adapter; recomputed Lamina BVP IBI vs E4
  IBI.csv aligned MAE for S2_Final and S5_midterm_2.
- **Result:** every BVP session's peaks and `hr_*` metrics are **bit-identical
  to baseline** (ppg path untouched by remediation). HR MAE persists at
  26.38–47.13 bpm (median 35.16). IBI alignment reproduces baseline exactly:
  28.9 ms (S2_Final, n=8361) and 45.4 ms (S5_midterm_2, n=4056).
- **Conclusion:** the gap persists and the evidence still points
  reference-side: E4 HR.csv contains implausible seated-exam HR (150–170 bpm)
  while Lamina's beat-level timing agrees with E4's own confident IBI output
  within ~29–45 ms. No new Lamina-side defect; unchanged P3 caveat on using
  E4 HR.csv as a reference.

## 5. WESAD known-groups revalidation (§16)

Same 15 recordings, same formulas (4-min windows; HR = ECG peaks/4 min;
SCR = chest-EDA count/4 min; RR = rsp_cycles mean rate):

| metric | baseline delta (stress−baseline) | current delta | direction |
|---|---|---|---|
| HR | +16.75 bpm | **+12.0 bpm** | preserved (positive) |
| SCR count | +8.0 /min | **+8.0 /min** | preserved (identical) |
| RR | +4.64 brpm | **+4.638 brpm** | preserved (identical) |

The HR delta attenuation is fully explained by NEW-C1: S3_baseline
55.25→110.25 bpm and S2_baseline 71.75→74.5 bpm (double-detection inflates
*baseline* HR, shrinking the stress−baseline gap; stress medians unchanged
at 79.0). SCR and RR known-groups are bit-identical to baseline.

## 6. EDA sampling-rate sweep (§8) — `eda_sweep.csv`, `plots/eda_sweep.png`

Synthetic EDA (slow tonic drift + 8 SCR bumps, known onsets) generated at
256 Hz, anti-aliased-decimated to 4/8/16/32/64 Hz; full clean+decompose+peaks
per rate; matching = greedy one-to-one, 1.0 s onset tolerance (runner rules).

| fs | n_scr | recall | FP | timing MAE | notes |
|----|-------|--------|----|-----------|-------|
| 4 | 10 | 1.00 | 2 | 0.000 s | pass-through clean; onsets exact |
| 8 | 10 | 1.00 | 2 | 0.125 s | |
| 16 | 9 | 0.875 | 2 | 0.125 s | 5 Hz lowpass active from here |
| 32 | 8 | 0.875 | 1 | 0.138 s | |
| 64 | 8 | 0.875 | 1 | 0.145 s | |

- No hidden 100 Hz assumptions, no invalid cutoffs, no crashes/NaN at any rate.
- Recall 1.0 at 4–8 Hz; one small bump (0.25 μS) falls below threshold after
  5 Hz smoothing at ≥16 Hz (expected threshold interaction, not a defect).
- Timing error ≤ 0.145 s everywhere, well inside the 1.0 s tolerance; small
  positive bias (+0.125–0.145 s) at ≥16 Hz consistent with filter group delay.

## NEW issues

### NEW-C1 — post-remediation `ecg-peaks` over-detects on WESAD chest ECG @700 Hz (S3: 2× beats) — **P1**

- **Observation:** with identical adapter and bridge invocation, WESAD chest
  ECG peak counts changed on 5/15 recordings: S3_baseline 221→441,
  S3_amusement 217→430, S2_baseline 287→298, S2_stress 307→309,
  S4_amusement 250→256 (other 10 recordings bit-identical).
- **Evidence:** `plots/s3_baseline_ecg_peaks.png` — S3 baseline QRS has a tall
  R spike followed ~0.3–0.4 s later by a broad S/T-wave hump (~0.5 mV);
  post-remediation detects **both** (441 peaks ≈ 110 bpm at rest; RR median
  0.415 s with 97% short-then-long alternation — classic T-wave
  double-detection). Baseline count 221 ≈ 55 bpm matches visual beat count.
- **Root cause (source diff pre/post remediation, src/ecg/peaks.rs):** the
  adaptive signal-peak estimate SPKI is now clamped
  (`y_eff = y_val.min(spki)` and `.min(2.5*spki)` in the searchback/running
  updates), so SPKI can no longer be raised by large R peaks; the adaptive
  threshold stays low and the secondary S/T humps (beyond the 200 ms
  refractory) pass. Likely an intended sensitivity increase for BUG-011
  (record 228 recall) with a T-wave side effect.
- **Impact:** WESAD cross-device HR consistency metrics (hr_mae/hr_bias/
  hr_rmse/hr_pearson_r) changed on 6 recordings; known-groups HR delta
  attenuated 16.75→12.0 bpm (direction preserved). Ground-truth impact must
  be quantified by Wave A on mit-bih (esp. record 228 and neighbors 104).
- **Repro:** `python -m validation run --dataset wesad --recordings S3_baseline
  --seed 0 --results-dir <dir>` → `ecg_chest_ecg_n_peaks = 441` (baseline 221).

### NEW-C2 — `eda-decompose` edge transient creates spurious boundary SCRs — **P3**

- **Observation:** phasic component shows a large onset/offset transient at
  both signal boundaries (visible in `plots/eda_sweep.png` t≈0 and t≈300 s);
  the sweep had 1–2 false-positive SCRs at boundaries at every rate
  (FP column in `eda_sweep.csv`); the first event of wes S1_midterm_1 also
  starts at `onset_index=0`.
- **Impact:** minor FP inflation at recording edges; rates remain plausible.
- **Suggestion:** document or pad/trim the tonic filter; consumers can drop
  events within the first/last seconds.

## Honest-failure log

No adapter run crashed. All runs completed and wrote results; the only
"failures" encountered were harness-side (fixed probe script issues), not
Lamina-side. No silent patching was needed anywhere.
