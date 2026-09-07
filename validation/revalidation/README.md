# validation/revalidation/ — Lamina post-remediation revalidation

This directory contains the complete independent revalidation of Lamina after
remediation commit `fec2668409b871094eeee35538b72dec776224f8`, organized by
validation wave (A–H) plus consolidated cross-wave artifacts. **Read
`REVALIDATION_REPORT.md` first** — it is the authoritative summary and links
into everything else.

## Top-level consolidated artifacts (Stage 4+5)

| File | Contents |
|---|---|
| `REVALIDATION_REPORT.md` | Final report: executive summary, environment, per-bug verdicts (BUG-001..020), dataset results, regression analysis, remaining bugs BUG-NEW-01..15, remediation priorities |
| `summary.json` | Machine-readable summary (commit, dataset/bug/regression counts, test gate results) |
| `dataset_results.csv` | Per-dataset aggregate baseline/current/delta/status rows (key metrics per dataset) |
| `record_results.csv` | Normalized union of all per-record rows across waves (1626 rows): `dataset,record,modality,metric,baseline,current,delta,status,notes` |
| `regressions.csv` | Every material regression (53 rows): `dataset,record,metric,baseline,current,delta,likely_mechanism,confidence` |
| `bugs_confirmed.csv` | Normalized bug ledger: BUG-001..020 verdicts + BUG-NEW-01..15 new issues, with evidence files |
| `api_results.csv` | Wave G API-robustness matrix (462 rows), copied unchanged from `api/` |
| `methodology.md` | What was preserved from the original validation, what changed and why, counting conventions, data provenance, environment |

## Wave directories

| Dir | Wave | Scope | Key files |
|---|---|---|---|
| `bugs/` | A | API-level & synthetic bugs (BUG-001/002/004/006/008/009/010/018-api/019) | `bugs_api_synthetic.csv`, `waveA-notes.md`, `evidence/` |
| `ecg/` | B | MIT-BIH arrhythmia + noise-stress, record 228 deep-dive, wrist s6 blackout, sampling sweep | `mitdb_records.csv`, `nstdb_results.csv`, `regression_matrix.csv`, `record228_deepdive.md`, `wrist_s6_blackout.md`, `bug_verdicts.csv`, `waveB-notes.md` |
| `eda/` | C | wearable-exam-stress, big-ideas, WESAD; 4 Hz EDA floor; SCR plausibility | `dataset_results_eda.csv`, `eda_sweep.csv`, `eda_config_paths.csv`, `eda_deepval_report.json`, `bug_verdicts.csv`, `waveC-notes.md`, `plots/` |
| `rsp_ppg/` | D | BIDMC respiration/PPG, wrist PPG exercise; rsp contract checks | `bidmc_records.csv`, `wrist_ppg_records.csv`, `rsp_contract.csv`, `rsp_sampling.csv`, `bug_verdicts.csv`, `waveD-notes.md`, `results/` |
| `rppg/` | E | SCAMPS rPPG; polarity matrix; AutoDetect audit | `scamps_results.csv`, `polarity_matrix.csv`, `signflip_check.csv`, `bug_verdicts.csv`, `waveE-notes.md`, `plots/`, `run-current/` |
| `hrv/` | F | HRV quality/correction architecture; policy matrix; aging trend | `policy_matrix.csv`, `cubic_vs_linear.csv`, `aging_results.csv`, `bug_verdicts.csv`, `waveF-notes.md`, probe scripts |
| `api/` | G | Numerical/API robustness: 462-case input matrix across 17 bridge ops | `api_results.csv`, `probe_api_robustness.py`, `waveG-notes.md`, `evidence/` |
| (top level) | H | Cross-language protobuf round-trips (Rust/Python/Dart) | `cross_language_results.csv`, `cross_language_report.md` |

## How to reproduce

1. Build the bridge: `cd validation/lamina_bridge && cargo build --release`
   (toolchain 1.98.1). Root-crate gates: `cargo fmt -- --check`,
   `cargo test --all-features` (139/139 at the revalidated commit).
2. Obtain the datasets: prefetched tarballs + manifests under
   `/mnt/agents/data/` (see `methodology.md` → Data provenance); extract to a
   local dir and set the adapter env vars (`LAMINA_DATA_DIR` etc.).
3. Rerun any dataset exactly as the waves did, e.g.
   `python -m validation run --dataset mit-bih-arrhythmia --seed 42`
   (ecg/ppg/bidmc/wrist: seed 42; autonomic/rppg: seed 0).
4. Wave-specific probes (228 deep-dive, sweeps, policy matrix, API matrix,
   polarity matrix) are documented in each `wave*-notes.md` with scripts
   committed beside the CSVs; the API matrix reproduces via
   `python3 validation/revalidation/api/probe_api_robustness.py`.
5. Join against the pre-remediation baseline snapshot
   (`/mnt/agents/output/reval-baseline/`, byte-identical to committed
   `validation/results/`) for the delta columns.
