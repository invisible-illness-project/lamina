# Lamina Validation Framework

Reproducible public-dataset validation suite for the Lamina crate.
**Validation-only**: Lamina (`src/`, `tests/`, `benches/`, root `Cargo.toml`,
`lamina_dart/`) is the implementation under test and is never modified. The
binding contract is [SPEC.md](./SPEC.md); verified API signatures live in
[API-INVENTORY.md](./API-INVENTORY.md).

## Layout

- `lamina_bridge/` — Rust bin crate: JSON-in/JSON-out CLI bridge over Lamina
  (SPEC §3). Auto-built by the Python client via cargo.
- `validation/` — Python package: canonical `Signal` schema, bridge client,
  metrics, 18-dataset registry, runner, manifest, report, consolidation
  (`consolidate.py`), CLI (SPEC §4–§7).
- `run_phase*.py` — Phased validation suite scripts (Phases 1 through 12) for detailed algorithm, signal morphology, cross-language, and hygiene testing.
- `fixtures/` — tiny committed synthetic signals with known ground truth (§8).
- `tests/` — pytest suite for the framework itself (no network, no downloads).
- `results/` — dataset run outputs (SPEC §6), see "Results layout" below.
- `revalidation/`, `revalidation-v2/`, `revalidation-v3/` — Historical revalidation run reports, baseline comparisons, and findings.
- `remediation-v2/`, `remediation-v3/`, `remediation-v4/` — Independent remediation audit reports (e.g. REV3-001 through REV3-004) and validation metric verifications.

## Phased Validation Suite

In addition to dataset category runs, Lamina provides a **12-Phase Validation Suite** (`run_phase*.py`). These targeted scripts run comprehensive unit, regression, counterexample, cross-language, and numerical hygiene checks across all signal modalities.

### Phased Scripts Overview

| Script | Phase | Domain & Description | Primary Output Artifact |
| :--- | :--- | :--- | :--- |
| [`run_phase1_phase2.py`](./run_phase1_phase2.py) | **Phases 1 & 2** | **ECG MIT-BIH 48-Record Benchmark**: Evaluates QRS peak detection (`bridge.ecg_peaks`) across all 48 records of the MIT-BIH Arrhythmia Database with a 150 ms matching tolerance. Computes Precision, Recall, and F1 per record and flags regressions against historical baselines. | `validation/revalidation-v2/results/phase1_phase2_ecg_mitdb.json` |
| [`run_phase3_4_5.py`](./run_phase3_4_5.py) | **Phases 3, 4 & 5** | **ECG Mechanism & Morphology Matrix**: Evaluates peak detector physical response (Phase 3), runs a 6-case regression matrix against baseline wander, noise, low amplitude, arrhythmia, inverted beats, and ectopics (Phase 4), and validates low-SNR wearable wrist ECG record `s6` (Phase 5). | `validation/revalidation-v2/results/phase3_4_5_ecg.json` |
| [`run_phase6_7.py`](./run_phase6_7.py) | **Phases 6 & 7** | **PPG/Respiration & 4-Hz Wearable EDA**: Validates respiration rate and breath peak detection on BIDMC dataset (Phase 6), and 4-Hz low-frequency skin conductance response (SCR) peak detection and SCL decomposition on Wearable Exam Stress & BigIdeas datasets (Phase 7). | `validation/revalidation-v2/results/phase6_7.json` |
| [`run_phase8_9.py`](./run_phase8_9.py) | **Phases 8 & 9** | **HRV Counterexamples & Cubic Spline Resampling**: Tests HRV metrics (SDNN, RMSSD, pNN50, LF/HF) against a 7-case counterexample matrix covering clean sinus, RSA, ectopics, bigeminy, trigeminy, and missing beats (Phase 8), and verifies cubic spline interpolation boundary handling (Phase 9). | `validation/revalidation-v2/results/phase8_9.json` |
| [`run_phase10.py`](./run_phase10.py) | **Phase 10** | **rPPG Polarity & Extraction Algorithms**: Validates remote photoplethysmography algorithms (CHROM, POS, GREEN, ICA) and verifies signal polarity auto-detection (`SignalPolarity::Standard`, `Inverted`, `AutoDetect`) and peak counting across inversions. | `validation/revalidation-v2/results/rppg_results.csv`<br>`phase10_rppg.json` |
| [`run_phase11.py`](./run_phase11.py) | **Phase 11** | **Protobuf & Cross-Language Parity**: Verifies serialization/deserialization and enum alignment across Rust (`lamina`), Python (`sensor_messages`), Dart (`lamina_dart`), and CLI bridge contracts (`CorrectionPolicy`, `SignalPolarity`, `RppgAlgorithmId`). | `validation/revalidation-v2/results/protobuf_roundtrip.json` |
| [`run_phase12.py`](./run_phase12.py) | **Phase 12** | **API & Numerical Hygiene**: Subjects all bridge RPC ops to 14 adversarial probes (NaNs, `+Inf`/`-Inf`, empty signals, zero/negative sample rates, invalid filter cutoffs), verifying 100% panic safety and structured `LaminaBridgeError` handling. | `validation/revalidation-v2/results/api_hygiene.json` |

### Running the Phased Validation Suite

To run an individual phase script from the repository root:

```bash
python3 validation/run_phase10.py
```

To run all phases sequentially:

```bash
for script in validation/run_phase*.py; do
    echo "=== Running $script ==="
    python3 "$script"
done
```

## Quickstart

```bash
# Environment: Rust toolchain, Python deps, bridge build (~8 min)
sh validation/scripts/setup-env.sh "$(pwd)"

# Dataset-level validation runner (From repository root):
python -m validation list                      # 18 registered datasets
python -m validation check-access              # honest access statuses
python -m validation run --dataset all --seed 0
python -m validation.consolidate               # merge per-group results → results/ top level
python -m validation report                    # docs/validation/public-dataset-validation.md (skeleton)
python -m validation test                      # framework test suite
```

Report figures regenerate from the results CSVs:

```bash
python3 docs/validation/figures/generate_figures.py
```

## Results layout

Runs are executed per category group; each group holds one run directory per
dataset (or a single group-level run directory covering several datasets):

```
validation/results/
├── summary.csv / recordings.csv / metrics.csv   # consolidated (union of all runs)
├── datasets.json / manifest.json                # consolidated (umbrella manifest)
├── ecg/{mit-bih-arrhythmia,mit-bih-noise-stress}/
├── ppg/                      # bidmc + wrist-ppg-exercise (group-level run)
├── autonomic/{wesad,autonomic-aging,wearable-exam-stress,big-ideas}/
└── rppg/                     # scamps (group-level run)
```

Each run directory contains `summary.csv`, `recordings.csv`, `metrics.csv`,
`datasets.json`, `manifest.json` (git commit, branch, seed, tolerances,
package versions), and `logs/`. The consolidated top-level files are
produced by `python -m validation.consolidate`; the umbrella
`manifest.json` lists every contributing run's own manifest and records
cross-run heterogeneity explicitly.

Phased validation results are stored in `validation/revalidation-v2/results/`.

## Documentation & Audits

- [`docs/validation/public-dataset-validation.md`](../docs/validation/public-dataset-validation.md) — full validation report (results per area, robustness, failures, bugs, reproducibility)
- [`docs/validation/architecture.md`](../docs/validation/architecture.md) — bridge/adapters/metrics/runner design
- [`docs/validation/datasets.md`](../docs/validation/datasets.md) — dataset selection, capability mapping, access notes
- [`docs/validation/metrics.md`](../docs/validation/metrics.md) — metric definitions, tolerances, reference-vs-Lamina distinction
- [`docs/validation/BUGS.md`](../docs/validation/BUGS.md) — consolidated bug log (documented, never fixed here)
- [`validation/revalidation-v3/REVALIDATION_REPORT.md`](./revalidation-v3/REVALIDATION_REPORT.md) — Revalidation v3 audit findings and baseline verification
- [`validation/remediation-v3/REMEDIATION_REPORT.md`](./remediation-v3/REMEDIATION_REPORT.md) — Remediation report for findings REV3-001 through REV3-004
- [`validation/remediation-v4/FINAL_AUDIT.md`](./remediation-v4/FINAL_AUDIT.md) — Independent engineering audit of rPPG `SignalPolarity::AutoDetect` remediation

## Bridge

The bridge is built automatically on first use
(`cargo build --release --manifest-path validation/lamina_bridge/Cargo.toml`)
or invoked directly:

```bash
echo '{"signal": [0.1, 0.2, 0.3], "sampling_rate": 100.0}' > in.json
validation/lamina_bridge/target/release/lamina_bridge \
    --op ecg-clean --input in.json --output out.json
```

Exit codes: `0` ok · `1` Lamina error · `2` panic caught · `3` protocol error.
Every op dispatch is wrapped in `catch_unwind`; a Lamina panic never aborts the
bridge without a JSON error envelope.

## Regenerating fixtures

```bash
python -m validation.fixtures_gen
```

## Statuses

`validated` · `partially_validated` · `inaccessible` · `unsupported_format` ·
`failed` · `not_attempted`. Potential Lamina bugs discovered during validation
are documented (never fixed) in `docs/validation/BUGS.md`.

