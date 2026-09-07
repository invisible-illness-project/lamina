# Revalidation Methodology

Independent revalidation of Lamina `main @ fec2668` (commit
`fec2668409b871094eeee35538b72dec776224f8`) after the remediation commit.
Validation-only: no Lamina source (`src/`), tests, benches, bindings
(`lamina_dart/`, `lamina_py/`), `Cargo.toml`, or pre-existing `validation/`
files were modified at any point. All revalidation artifacts live under
`validation/revalidation/` (new directory).

## What was preserved from the original validation

- **Event matching**: greedy one-to-one matching, tolerances **150 ms**
  (ECG/PPG), **0.5 s** (respiration cycles), **1.0 s** (SCR onsets), via the
  repo's own `validation.metrics.events` functions.
- **Seeds**: ecg/ppg/bidmc/wrist runs seed **42**; autonomic (wesad,
  wearable-exam-stress, big-ideas, autonomic-aging) and rppg (scamps) runs
  seed **0** — identical to the baseline invocations.
- **Adapters unchanged**: all dataset adapters and runner configs untouched;
  reruns used the original commands (`python -m validation run --dataset ...`)
  against the same prefetched data.
- **Baseline reference**: pre-remediation results snapshotted at
  `/mnt/agents/output/reval-baseline/` (byte-identical to committed
  `validation/results/`), joined row-by-row for every delta.
- **Bridge ops extended additively only**: existing ops kept their JSON field
  names and numerical semantics; the historical `ecg-clean` method token
  `"none"` maps to the new `""` token (verified to hit the same 0.5 Hz
  high-pass path as baseline; direct bridge probe on record 228 reproduces the
  suite output exactly).

## What changed, and why (task §5 documented methodology changes)

1. **New bridge ops for new APIs**: `eda-clean-config`, `rppg-polarity`,
   `hrv-correct` were added to the validation bridge to exercise the new
   post-remediation APIs (`EdaCleaningConfig`, `SignalPolarity`/
   `to_bvp_waveform`, `classify_intervals`/`clean_rr_intervals`/
   `CorrectionPolicy`). These ops did not exist at baseline; results obtained
   through them are labeled `new-methodology` in `record_results.csv` /
   `dataset_results.csv` and are never mixed into old-methodology columns.
2. **Class-conditional recall for mitdb record 228** (NEW analysis alongside
   the old all-beat methodology): the baseline was symbol-agnostic; the
   revalidation additionally tabulates the same greedy match against `.atr`
   beat symbols (N / V / other) because BUG-011's remediation claim is
   specifically about normal-beat recall after tall PVCs. Old all-beat numbers
   are still reported for comparability (`ecg/228_class_recall.csv`).
3. **HRV corrected path**: baseline RMSSD used the bridge `hrv` op on raw
   detected peaks. The revalidation adds a NEW column computed via
   `hrv-correct policy=reject_invalid` on the same detected peaks
   (`hrv/aging_results.csv`: `rmssd_current_raw` = old methodology,
   `rmssd_corrected` = new methodology). The corrected path did not exist at
   baseline; deltas against baseline are labeled accordingly.
4. **Polarity matrix** (NEW analysis for BUG-005): 10 SCAMPS videos × 3
   algorithms × 3 polarity modes via the new `rppg-polarity` op, correlated
   against the synthesis ground truth `d_ppg`. The old-methodology rerun is
   reported separately (bitwise identical to baseline).
5. **Threshold-multiplier sensitivity probes** (`threshold_multiplier=0.5`
   etc.) were run as **diagnostics only** to characterize the operating point
   of the post-remediation ECG detector. They are explicitly NOT a proposed
   fix and are labeled as such wherever cited.

## Counting conventions used in summary.json

- `bugs_resolved` counts verdict == RESOLVED, including BUG-011
  (resolved on the target record; its fleet-wide side effect is tracked
  separately as BUG-NEW-03) and BUG-013/BUG-014 (resolved as symptom /
  not-a-Lamina-defect). BUG-020 (NOT REPRODUCED — anomaly gone but original
  attribution overturned) and BUG-015 (unchanged, reference-side) are **not**
  counted as resolved. This gives 14 resolved of 20; counting BUG-020's
  symptom disappearance would give 15 — hence the report's "14–15 of 20".
- `bugs_remaining` = 20 − resolved = 6 (BUG-005 P2, BUG-012 P1, BUG-015 P3,
  BUG-017 P3, BUG-018 P2, BUG-020 P3).
- `regressions_found` = row count of `regressions.csv` (53). All rows trace
  to the single BUG-NEW-03 SPKI-clamp mechanism, so
  `regressions_by_severity` = {P0: 53}; per-row confidence is in the CSV
  (high 43, medium 10).
- `new_issues_found` = 15 normalized BUG-NEW-* items (including the two
  cross-language items); wave-local IDs (BUG-NEW-B1, NEW-C1, NEWD-01,
  BUG-NEW-E1, BUG-OBS-01, Wave G F1–F6, Wave H P1/P2 findings) are mapped in
  `bugs_confirmed.csv` notes.
- Where a wave's notes and its CSV disagree, the CSV is authoritative (see
  REVALIDATION_REPORT.md → Inconsistencies).

## Data provenance

All datasets were prefetched as tarballs with sha256 manifests under
`/mnt/agents/data/` (`<dataset>.tar.gz` + `<dataset>.MANIFEST.txt`):
autonomic-aging, bidmc, big-ideas, mitdb, nstdb, scamps (split parts),
wearable-exam-stress, wesad (reassembled from parts, sha256
`881cb03f…f507` verified), wrist. Waves extracted them to `$HOME/data`
(FUSE/noexec on /mnt) and pointed the adapters' env vars there
(`LAMINA_DATA_DIR`, `LAMINA_BIDMC_DIR`, `LAMINA_WRIST_DIR`,
`SCAMPS_DATA_DIR`). One deviation: the SCAMPS split-tarball FUSE reads failed
deterministically after ~0.9–1.0 GB per container lifetime, so the 10 `.mat`
videos were re-fetched via HTTP range GETs from the origin documented in
`scamps.MANIFEST.txt`; authenticity is demonstrated a posteriori by the
pipeline output being bitwise identical to baseline (510/510 metric rows).

## Environment

- rustc/cargo 1.98.1 (`rustc 1.98.1 (48a229cea 2026-09-01)`, rsproxy.cn
  sparse mirror); OS `Linux 5.10.134-18.0.12.lifsea8.x86_64`.
- Validation bridge `lamina_bridge` built `--release` against
  post-remediation Lamina (repo bridge commit `caa7484`).
- Python 3.x with numpy/scipy/pandas/wfdb/matplotlib; probe scripts are
  committed next to each wave's CSVs (`validation/revalidation/*/…​.py`).
- Baseline verification (stage 1): `cargo fmt -- --check` PASS;
  `cargo test --all-features` PASS 139/139 (root crate, 14 integration
  binaries); `cargo clippy --all-targets --all-features -- -D warnings` FAIL
  (2 test-target lints; library target clean).
- No root `Cargo.lock` is tracked upstream (gitignored); 165 packages
  resolved during the baseline build. `validation/lamina_bridge/Cargo.lock`
  is tracked.
- Repo content verified byte-identical to upstream
  `invisible-illness-project/lamina@fec2668` for all source/test/doc files
  (`git ls-tree -r` comparison; only a stray `.pyc` and 4 FUSE-lost exec bits
  differ).
