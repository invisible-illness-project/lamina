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
- `fixtures/` — tiny committed synthetic signals with known ground truth (§8).
- `tests/` — pytest suite for the framework itself (no network, no downloads).
- `results/` — run outputs (SPEC §6), see "Results layout" below.

## Quickstart

```bash
# Environment: Rust toolchain, Python deps, bridge build (~8 min)
sh validation/scripts/setup-env.sh "$(pwd)"

# From the repository root:
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

## Documentation

- [`docs/validation/public-dataset-validation.md`](../docs/validation/public-dataset-validation.md) — full validation report (results per area, robustness, failures, bugs, reproducibility)
- [`docs/validation/architecture.md`](../docs/validation/architecture.md) — bridge/adapters/metrics/runner design
- [`docs/validation/datasets.md`](../docs/validation/datasets.md) — dataset selection, capability mapping, access notes
- [`docs/validation/metrics.md`](../docs/validation/metrics.md) — metric definitions, tolerances, reference-vs-Lamina distinction
- [`docs/validation/BUGS.md`](../docs/validation/BUGS.md) — consolidated bug log (documented, never fixed here)

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
