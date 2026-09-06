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
  metrics, 18-dataset registry, runner, manifest, report, CLI (SPEC §4–§7).
- `fixtures/` — tiny committed synthetic signals with known ground truth (§8).
- `tests/` — pytest suite for the framework itself (no network, no downloads).
- `results/` — gitignored machine-readable run outputs (SPEC §6).

## Quickstart

```bash
pip install -r validation/requirements.txt

# From the repository root:
python -m validation list                      # 18 registered datasets
python -m validation check-access              # honest access statuses
python -m validation run --dataset all --seed 0
python -m validation report                    # docs/validation/public-dataset-validation.md
python -m validation test                      # framework test suite
```

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
