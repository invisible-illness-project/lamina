# Wave E — rPPG (SCAMPS) revalidation @ fec2668 (BUG-005 polarity)

Branch `reval/rppg`. Validation-only: no Lamina source touched; everything under
`validation/revalidation/rppg/` is new. Methodology preserved: same adapter
(`validation/validation/datasets/rppg.py::ScampsAdapter`, unchanged since the
baseline snapshot), seed 0, same metrics (`peak_detection_metrics` greedy
one-to-one @ 150 ms; `rate_metrics` over 6 s/1 s HR windows; 0.75–2.5 Hz
bandpass correlation).

## Data provenance (environment note)

`/mnt/agents` FUSE reads of the prefetched split tarball failed deterministically
after ~0.9–1.0 GB per container lifetime (`Transport endpoint is not connected`
on parts 09–12; parts 00–08 md5-verified against `scamps.MANIFEST.txt`). The 10
`.mat` videos were therefore re-fetched from the MANIFEST's documented origin
(`.../neurips-2022/scamps_videos_example.tar.gz`, HTTP range GETs).
Authenticity is demonstrated a posteriori: the full pipeline output over this
data is **bitwise identical** to the baseline (below) — any byte-level data
difference would perturb the skin-mask ROI means and every downstream metric.
Per-video ROI traces were computed once with the adapter's own `_load_video`
(its built-in `.trace_cache` mechanism) and checkpointed.

## 1. Unchanged-adapter rerun (mission §1) — old methodology

`SCAMPS_DATA_DIR=$HOME/data python -m validation run --dataset scamps --seed 0
--results-dir validation/revalidation/rppg/run-current` → 30/30 ok.

**All 510 scamps metric rows are bitwise identical to baseline**
(`scamps_results.csv`: status=match for every row, max |delta| = 0.0). The
waveform-correlation extras also match (max |Δ| = 1.1e-16).

| algorithm | HR MAE baseline | HR MAE current | delta | peak F1 baseline=current |
|-----------|-----------------|----------------|-------|--------------------------|
| green     | 11.39           | 11.39          | 0.0   | 0.245                    |
| chrom     | 11.82           | 11.82          | 0.0   | 0.768                    |
| pos       | 16.19           | 16.19          | 0.0   | 0.540                    |

The mission's hypothesized regression scenario ("adapter compensated polarity;
default output polarity changed → previously-good recordings regress") does
**not** occur, for two verified reasons: (a) the remediation is additive-only —
`rppg-algorithm` numerics are unchanged (SPEC §A: "no existing op changed its
JSON field names or numerical semantics"), and (b) the baseline adapter never
sign-corrected the waveform fed to `ppg-peaks`; the only "compensation" was the
`abs(corr)` sign-corrected *correlation* extra. Hence the old-methodology
pipeline still measures the raw, polarity-inconsistent waveforms, and BUG-005's
downstream harm (green F1 = 0.245) is still fully visible in the rerun.

## 2. Polarity matrix (mission §2) — new methodology

`polarity_matrix.csv`: 10 videos × 3 algorithms × 3 polarity modes via the new
`rppg-polarity` bridge op (same sliding-window config as the adapter:
3.0 s/0.5 s). `corr_pos`/`corr_neg` = Pearson r of the returned BVP vs +d_ppg /
−d_ppg after the adapter's identical bandpass. `agrees_with_contract` =
corr_pos > 0 (signal-contracts.md §2.4: positive deflection = peak systolic
blood volume; d_ppg is the synthesis ground truth in blood-volume phase —
verified by chrom's positive raw r and by positive skewness, §3 below).

Headline means (10 videos):

| algorithm | mode     | corr_pos | peak F1 | HR MAE |
|-----------|----------|----------|---------|--------|
| green     | normal   | −0.921   | 0.245   | 11.39  |
| green     | inverted | +0.921   | 0.918   | 12.44  |
| green     | auto     | −0.921   | 0.245   | 11.39  |
| chrom     | normal   | +0.362   | 0.768   | 11.82  |
| chrom     | inverted | −0.362   | 0.803   | 13.93  |
| chrom     | auto     | −0.432   | 0.723   | 14.16  |
| pos       | normal   | −0.823   | 0.540   | 16.19  |
| pos       | inverted | +0.823   | 0.920   | 7.90   |
| pos       | auto     | −0.823   | 0.540   | 16.19  |

Notes:
- **2(d) exact sign flip: PASS** — for all 30 video×algorithm pairs,
  `inverted == −normal` exactly (finite-mask identical, max |a+b| = 0;
  `signflip_check.csv`).
- Convention-correct polarity massively fixes peak alignment for the two
  intensity-phase algorithms (green F1 0.245→0.918, pos F1 0.540→0.920, pos HR
  MAE 16.19→7.90 bpm). green HR MAE slightly *worsens* (11.39→12.44) despite
  F1 → 0.918: wrong-phase peaks still fire once per cycle, so IBI-based HR was
  already roughly right; polarity primarily fixes *beat alignment* (F1). Honest
  nuance: HR MAE alone does not diagnose polarity (BUG-005's harm was partly
  invisible to it for green).
- `polarity=normal` (the default) is a pass-through: the returned "BvpWaveform"
  keeps the raw per-algorithm phase, so the default output still violates the
  §2.4 contract for green/pos. `polarity=inverted` works exactly, but choosing
  it correctly requires the very per-algorithm sign knowledge BUG-005 reported.

## 3. AutoDetect audit (mission §3) — BUG-NEW-E1 (P2)

`to_bvp_waveform(AutoDetect)` → `compute_should_flip` (src/rppg/signal.rs:558):
flip iff sample skewness > 0.3.

Measured skewness (raw waveforms):

| signal            | skew range (10 videos) | mean  |
|-------------------|------------------------|-------|
| d_ppg (GT, BVP phase)   | +0.42 … +1.28    | +0.72 |
| green raw         | −0.84 … −1.51          | −1.11 |
| pos raw           | −0.97 … −2.88          | −1.84 |
| chrom raw         | −0.18 … +3.00          | +1.30 |

A contract-phase BVP (sharp systolic peak, slow decay) is **positively**
skewed; an intensity-phase signal is negatively skewed. The heuristic therefore
flips signals that are *already* in BVP phase and leaves intensity-phase
signals unflipped — exactly backwards w.r.t. its rustdoc intent ("skewness
relative to Elgendi pulse expectations").

- Consistency per algorithm: green 10/10 `normal`, pos 10/10 `normal`
  (consistent but wrong); chrom 7/10 `inverted`, 3/10 `normal` (inconsistent,
  threshold-straddling).
- Agreement with the d_ppg-correlation-maximizing convention: **1/30**
  (only P000003-chrom). See `polarity_matrix.csv` auto rows and the audit table
  in the final report.
- Downstream harm if `auto` is trusted: chrom (natively contract-phase)
  corr +0.362 → −0.432, peak F1 0.768 → 0.723, HR MAE 11.82 → 14.16 bpm.
  Synthetic-sinusoid probe additionally shows auto is a silent no-op on
  symmetric waveforms (skew ≈ 0 → never flips), so it cannot fix green/pos even
  in the benign case.

## 4. BUG-005 verdict: PARTIALLY RESOLVED (remaining severity P2)

Delivered and verified: formal pulse-phase contract (signal-contracts.md §2.4),
`SignalPolarity` + `to_bvp_waveform` on `RppgSignal`/`RppgSegment`, exact
negation semantics, bridge op `rppg-polarity` with faithful
`polarity_resolved` reporting. Not delivered: actual cross-algorithm phase
normalization — the raw outputs are unchanged (verified bitwise), the default
mode is pass-through, and the provided AutoDetect resolves to the wrong phase
in 29/30 SCAMPS cases. A user following the documented path
(`to_bvp_waveform` default or `auto`) still feeds wrongly-phased waveforms to
`ppg-peaks` for green/pos — the original silent-degradation scenario survives
under the new API. The contract is only achievable by callers who already know
each algorithm's native sign.

## 5. Morphology evidence (mission §5)

`plots/morphology_P000001_auto.png` — auto-mode BVP vs d_ppg (aligned, same
bandpass, z-scored) for green/chrom/pos: green and pos visibly antiphase
(corr −0.898 / −0.867), chrom antiphase after auto's wrong flip (−0.511).
`plots/morphology_P000001_contract_phase.png` — per-algorithm correct polarity
(green/pos inverted, chrom normal): clean phase lock, corr +0.898/+0.511/+0.867.

## Remaining concerns

- BUG-NEW-E1 (P2): AutoDetect heuristic inverted (above). Suggested direction:
  flip when skew < −0.3, or estimate phase via Elgendi peak prominence
  asymmetry; add SCAMPS-style regression coverage (none of the hermetic tests
  in validation/tests/test_revalidation_ops.py exercise a realistic-morphology
  auto case against ground truth).
- Default `polarity=normal` being a pass-through means the *default* BVP does
  not satisfy the documented convention for 2/3 shipped algorithms (P3 doc/API
  ergonomics: either normalize at extraction or make the default `auto` once
  the heuristic is fixed).
- green HR MAE insensitivity to polarity (above) means HR-only benchmarks can
  mask phase bugs; peak F1/correlation should be reported alongside (P3,
  methodology note for future rPPG validation).

## Environment/repro notes

Container churn (periodic wipes) and the FUSE ~1 GB read ceiling shaped the
procedure (Azure re-fetch; npz trace checkpoints; bridge binary checkpointed).
All analysis scripts and intermediate checkpoints:
`/mnt/agents/output/1a079754-4132-8d65-8000-0fc6cc73e78a/scratch/`
(`matrix.py` = polarity matrix generator, `traces.py` = adapter-faithful trace
cache, `ensure.sh`/`setup.sh` = idempotent env).
