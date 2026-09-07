# Wave D Revalidation Notes — Respiration + PPG (post-remediation fec2668)

Validator: Wave D subagent. Repo: `$HOME/work` clone of /mnt/agents/lamina-reval @ caa7484
(branch `reval/rsp-ppg`). Lamina source NOT modified; all runs against release bridge built
from `validation/lamina_bridge` (binary copied to
`validation/lamina_bridge/target/release/lamina_bridge`).

## Methodology (unchanged from baseline)

- `python -m validation run --dataset bidmc --seed 42` and
  `--dataset wrist-ppg-exercise --seed 42`, results under
  `validation/revalidation/rsp_ppg/results/`.
- Data: prefetched tarballs extracted to `$HOME/data/bidmc` (12 records, verified readable)
  and `$HOME/data/wrist` (19 records, verified readable); `LAMINA_BIDMC_DIR` /
  `LAMINA_WRIST_DIR` pointed there. `LAMINA_PPG_METRICS_EXTRA` redirected into the
  revalidation results dir so committed baseline CSVs were not touched.
- Matching: repo's own `validation.metrics.events.peak_detection_metrics` (greedy 1:1,
  0.5 s respiration tolerance, 150 ms ECG/PPG) reused for all probes — methodology identical.
- Contract probes called the bridge binary directly (`rsp-cycles`, `rsp-clean`,
  `ecg-peaks`, `ppg-peaks`). `evaluate_ecg_quality` is not bridge-exposed, so it was
  exercised through a scratch Rust harness in `$HOME/scratch/quality_probe`
  (path dependency on the Lamina crate; no Lamina source edits).

## Headline deltas

### BIDMC respiration (12/12 ok)
- Mean cycle F1: **0.9437 baseline → 0.9437 current (Δ 0.0000)**. Every rsp metric
  (P/R/F1/TP/FP/FN/timing) is byte-identical on all 12 records.
- bidmc05: **F1 0.658 → 0.658** (P 0.490, R 1.000, 98 detected vs 48 annotated). No change.
- PPG side (PLETH peak counts) byte-identical on all 12 records.
- ECG peak counts changed on 4 records (Wave B territory): bidmc01 730→782,
  bidmc03 153→628, bidmc04 738→739, bidmc06 656→812.

### Wrist PPG exercise (19/19 ok)
- All PPG/HR metrics byte-identical to baseline: HR MAE overall **20.75 bpm**;
  walk 15.88 / run 32.90 / bike-low 14.73 / bike-high 20.27;
  stationary (bike) 16.81 vs motion (walk+run) 23.61 bpm.
- s1_walk HR bias +32.96 bpm (unchanged). BUG-016 behavior persists exactly.
- ECG-channel metrics on wrist changed (Wave B scope; e.g. s6_low_resistance_bike ECG F1
  0.657→0.736, s5_low_resistance_bike 0.716→0.997; some records worsened, e.g. s2_walk
  ECG F1 0.895→0.688) — flagged for the ECG wave, not revalidated here.

## RSP contract checks (task §10) — see rsp_contract.csv

- **(a) sampling_rate respected: PASS.** Same physical signal at 25/50/100 Hz → 59 cycles
  at each rate, max cross-rate timing deviation 0.04 s (one sample at 25 Hz), F1 0.9916 and
  timing MAE 0.0088 s identical across rates. No hidden 100 Hz assumption observed.
- **(b) max_breath_interval_sec: PASS.** 10 s intervals accepted under cap 12 and cap 20;
  15 s intervals rejected under cap 12 (recall 0.05) and fully accepted under cap 20
  (F1 1.0). Absent key behaves exactly as cap 20 → **new default 20.0 confirmed
  behaviorally**. Minor observation: with cap 12 on the 15 s signal, one spurious
  10.48 s cycle survives from filter edge transients (F1 0.095 instead of 0.0) — cosmetic.
- **(c) precleaned: PASS (BUG-007 fixed).** `precleaned` absent ≡ false (identical
  outputs). `precleaned=true` verifiably skips the internal bandpass: on a signal
  containing out-of-band 2 Hz content, 92 cycles are returned vs 44 cleaned.
  Identity test: `rsp_cycles(rsp_clean(raw), precleaned=true)` produces
  **exactly** the cycles of `rsp_cycles(raw, precleaned=false)` (same inspiration
  indices and amplitudes). Double-cleaning (`precleaned=false` on pre-cleaned input)
  still shifts amplitudes slightly (2.0089→2.0332), documenting why the flag exists.

## RSP sampling sweep (task §8) — see rsp_sampling.csv

Synthetic respiration, 60 known non-uniform cycle times (4–8 s), at 25/50/100 Hz,
clean and with σ=0.2 additive noise. P/R/F1 identical across rates
(P 1.0 / R 0.983 / F1 0.9916; the single FN is structural: the final peak has no
successor peak, so it cannot form a cycle). Timing MAE 0.0088 s clean, 0.032–0.055 s
noisy (noise contribution, decreasing with fs). No instability, no wrong distance
constraints, no hidden 100 Hz assumptions detected.

## Per-bug narratives

### BUG-007 — rsp_cycles double-cleaning → RESOLVED
- *Original finding:* `rsp_cycles_config` unconditionally re-cleaned its input, so callers
  passing a pre-cleaned signal got double filtering.
- *Current implementation:* `RspProcessingConfig.precleaned` (src/rsp/peaks.rs:22, default
  false; used at line 178 to skip `rsp_clean_config`); bridge exposes it as optional
  `precleaned` (absent = default).
- *Validation performed:* synthetic drift+artifact signal probes (above); identity test
  against explicit `rsp-clean`.
- *Result:* contract honored exactly; absent flag preserves legacy default.
- *Remaining concern:* none for the contract. Callers who pre-clean but forget the flag
  still double-filter (amplitude shifts ~1%); documented behavior, informational.

### BUG-016 — motion over-counting on wrist PPG → RESOLVED (documentation action)
- *Original finding:* foot-strike spikes accepted as beats; HR MAE stratified by activity.
- *Current implementation:* no detector change (by design); limitation documented in
  `docs/validation/public-dataset-validation.md` ("Motion remains the hard case for wrist
  PPG", Robustness section) with the activity-stratified numbers, which still hold exactly.
- *Validation performed:* full 19-record rerun; per-record PPG/HR metrics byte-identical.
- *Result:* motion HR MAE 23.6 vs stationary 16.8 bpm; run worst (32.9). Behavior matches
  the documented limitation; `signal-contracts.md` §2.3 makes no motion-robustness promise,
  so no contract violation.
- *Remaining concern (P3):* the PPG contract section itself still lacks an explicit
  "single-channel detector, no motion-artifact rejection" caveat; the caveat lives only in
  the validation report.

### BUG-017 — bidmc05 respiration over-detection → NOT RESOLVED (P3)
- *Original finding:* F1 0.658 (P 0.490 / R 1.000), 98 detected vs 48 annotated at ~6 brpm;
  baseline digest attributed this to the 12 s `max_breath_interval_sec` ceiling
  "structurally rejecting true cycles up to 14.3 s".
- *Current implementation:* default raised to 20.0 (src/rsp/peaks.rs:32, confirmed
  behaviorally); the promised "secondary peak prominence gating" is **absent** — no
  prominence parameter exists in `RspProcessingConfig`; peak detection uses
  `PeakDetectionConfig` with only `min_distance`; the only gates are duration
  [1.2, 20] s and `min_amplitude` 0.05.
- *Validation performed:* full BIDMC rerun (identical) + direct probes of bidmc05 RESP with
  cap 12 / 20 / absent.
- *Result:* all three configs yield 98 cycles, F1 0.6575. Detected cycle durations span only
  2.56–7.18 s — zero candidate intervals fall in (12, 20] s, so the ceiling change is a
  **no-op** on this record. The true mechanism is biphasic peak doubling: mean 2.06
  detected cycles per annotated breath, alternating long/short intervals (5.55/4.16 s);
  the pre-inspiratory bump peaks pass the 0.05 amplitude gate (minimum surviving cycle
  amplitude 0.052). Baseline recall was already 1.000, which contradicts the digest's
  structural-rejection story.
- *Remaining concern:* prominence gating (the remediation that would actually address the
  doubling) was planned but not implemented. Low severity (limitation per plan).

### BUG-020 — bidmc03 ECG near-flatline → NOT REPRODUCED (P3)
- *Original finding:* ecg-peaks found 153 beats vs 610 PPG beats; attributed to a
  near-flatline lead II (dataset issue); remediation: document + verify
  `evaluate_ecg_quality` flags low quality.
- *Current implementation:* `evaluate_ecg_quality` exists in `src/multimodal/quality.rs`
  (rule-based: EmptySignal / InsufficientEvents / UnplausibleHeartRate / ExtremeArtifact)
  but is **not exposed through the bridge** (no quality op in the op table).
- *Validation performed:* BIDMC rerun + direct bridge `ecg-peaks`/`ppg-peaks` on bidmc03 +
  scratch Rust harness calling `evaluate_ecg_quality`.
- *Result:* the anomaly no longer reproduces — ecg-peaks now finds 628 beats vs 610 PPG
  (mean 78.5 vs 76.2 bpm); PPG-vs-ECG HR consistency MAE improved 51.6 → 5.1 bpm.
  `evaluate_ecg_quality` on the current peaks: score 1.0, valid, no issues. Fed a
  baseline-like 153-beat detection it reports score 0.6 with `UnplausibleHeartRate` — but
  still `valid=true` (threshold ≥ 0.5). The lead II is low-voltage (p2p 1.26 mV, std
  53 µV) but QRS is clearly detectable, so the original "near-flatline dataset-issue"
  attribution was likely wrong: the 153-beat under-detection looks like a detector defect
  that disappeared with the ECG remediation (correlates with Wave B / BUG-019 territory).
- *Remaining concern:* docs (`public-dataset-validation.md`, `BUGS.md`, `datasets.md`)
  still describe the stale 153-vs-610 anomaly as a dataset issue and should be updated;
  the quality API is unreachable via the bridge; `valid=true` at score 0.6 means the
  quality gate would not have rejected even the baseline broken detection.

## New issues observed (not in baseline bug list)

- **NEWD-01 (ECG regression, hand to Wave B):** post-remediation ecg-peaks over-detects on
  two BIDMC records where baseline agreed with PPG: bidmc06 ECG 656→812 peaks (101.5/min vs
  PPG 81.8/min; HR consistency MAE 0.54→30.24 bpm) and bidmc01 730→782 (consistency MAE
  0.52→8.00 bpm). bidmc04 mildly worse (1.69→2.03). Detected via the
  hr_consistency_ppg_vs_ecg cross-check in ppg_metrics_extra. Severity suggestion: P2 —
  new false-positive ECG detections on ICU lead II @125 Hz.
- **NEWD-02 (docs):** REMEDIATION_PLAN Phase 2 claims "prominence gating" for BUG-017 and
  Phase 3 "document motion artifact limitations"; prominence gating is absent from source
  and the motion caveat is absent from signal-contracts.md §2.3 (present only in the
  validation report). Documentation/plan vs implementation drift. P3.
- Minor: rsp-cycles at cap 12 on a 15 s-interval signal keeps one spurious 10.48 s edge
  cycle from filter transients (boundary effect, cosmetic).

## Reproducibility

All probe code paths: `$HOME/work/validation/revalidation/rsp_ppg/results/` (runner outputs)
plus IPython-driven bridge probes against
`validation/lamina_bridge/target/release/lamina_bridge`. Numbers above are exactly the CSVs
committed next to this file.
