# Validation Architecture

How the Lamina validation suite is put together, and why. The binding
contract is [`validation/SPEC.md`](../../validation/SPEC.md); this document
is the readable companion.

## Overview

```text
public datasets (WFDB, E4 CSV, MATLAB v7.3, ...)
        │
        ▼
dataset adapters ── canonical Signal (samples, fs, modality, annotations)
(validation/validation/datasets/)        │
        │                                ▼
        │                         runner.py: per-recording try/except,
        │                                deterministic sampling, metrics,
        │                                result writers
        ▼                                │
LaminaBridge ── JSON files ◄──► lamina_bridge (Rust binary)
(validation/validation/bridge.py)        │
        ▲                                ▼
        └────────────── Lamina crate (implementation under test)
```

## The Rust bridge, and why it exists

The implementation under test is a Rust crate; the validation suite is
Python. The design rule (SPEC §3) is that **all** Lamina functionality is
exercised through a compiled Rust binary, `validation/lamina_bridge`,
invoked as a subprocess:

```bash
lamina_bridge --op ecg-peaks --input in.json --output out.json
```

Why a subprocess bridge instead of Python bindings (e.g. PyO3)?

1. **The crate under test stays untouched.** No binding shim, no feature
   flags, no `cdylib` target creep into Lamina. The bridge is a separate bin
   crate that depends on Lamina exactly as a downstream user would.
2. **Failure isolation.** Every op dispatch is wrapped in
   `catch_unwind`; a Lamina panic becomes a JSON error envelope with exit
   code 2, never an aborted process that takes the validation run with it.
   Lamina errors (`Err`) map to exit code 1, protocol problems to 3. The
   Python client raises `LaminaBridgeError` carrying the structured
   `error.kind` (`bad_request` / `lamina_error` / `panic`) and message —
   which is how confirmed defects like BUG-A03 land verbatim in
   `recordings.csv`.
3. **Honest defaults.** Bridge config fields map 1:1 onto Lamina
   config-builder methods; an absent field means the Lamina default is used.
   The bridge never silently substitutes its own parameter choices, so the
   measured behavior is the behavior a user gets.
4. **Reproducibility.** The bridge binary reports `lamina_version` (extracted
   from the crate's `Cargo.toml` at build time via `build.rs`) and the
   `version` op is recorded in every run manifest along with the git commit.

The ops mirror Lamina's public API surface (SPEC §3.2): `ecg-clean`,
`ecg-peaks`, `ppg-clean`, `ppg-peaks`, `eda-clean`, `eda-decompose`,
`eda-peaks`, `rsp-clean`, `rsp-cycles`, `hrv`, `signal-peaks`, `filter`,
`sample-entropy`, `rppg-algorithm`. Non-finite floats serialize as JSON
`null` with explicit flags where non-finite values are legitimate (e.g.
`sample-entropy`). All peak indices are 0-based sample indices into the
input signal.

## Python side

- **`schema.py`** — the canonical `Signal`: 1-D float64 samples, positive
  finite sampling rate, one of 12 modalities, optional timestamps, units,
  provenance fields, and an `annotations` dict (e.g. reference
  `peak_indices`). `validate()` rejects malformed signals before they reach
  the bridge; `resample_signal()` is the only signal processing on the
  Python side (polyphase, for cross-rate alignment) and is always explicit.
- **`bridge.py`** — `LaminaBridge(auto_build=True)`: locates or cargo-builds
  the bridge binary, `run_op()` plus typed wrappers per op returning the
  `result` payload.
- **`datasets/`** — one adapter per dataset behind the `DatasetAdapter` ABC
  (`info()`, `check_access()`, `iter_recordings()`). Adapters convert native
  formats into `Signal`s and expose dataset-provided references. Access is
  probed honestly: datasets that cannot be obtained report `inaccessible`
  with the verified reason instead of failing mid-run. All validation-side
  heuristics (channel fallback, windowing, reference-peak extraction,
  interpolation of device dropouts) live here and are documented per dataset
  in [`datasets.md`](./datasets.md).
- **`metrics/`** — `events.py` (greedy 1:1 nearest-within-tolerance event
  matching, TP/FP/FN/precision/recall/F1, timing error), `rate.py` (MAE,
  RMSE, bias, Pearson r over aligned windows), `hrv.py` (absolute/relative
  error between Lamina-detected-beat HRV and annotation-derived HRV — kept
  explicitly distinct). Definitions and tolerances:
  [`metrics.md`](./metrics.md).
- **`runner.py`** — `run_dataset()` iterates recordings with deterministic
  sampling (`subjects`/`recordings`/`max_recordings`/`seed`/`smoke`); each
  recording runs in its own try/except so one failure never aborts the
  dataset; failures are logged with tracebacks and recorded per recording.
  Writes `recordings.csv`, `metrics.csv` (long format), `summary.csv`,
  `datasets.json`, `manifest.json` per run.
- **`manifest.py`** — git commit + branch, Lamina version from the bridge,
  Python/package versions, seed, tolerances, config snapshot, UTC timestamp.
- **`consolidate.py`** — merges the per-group run artifacts into the
  canonical top-level `validation/results/` files and writes an umbrella
  manifest that lists every contributing run's own manifest, explicitly
  recording heterogeneity (multiple adapter-branch commits, seeds) instead
  of fabricating a single synthetic run.
- **`report.py`** — regenerates the skeleton of
  `docs/validation/public-dataset-validation.md` from the consolidated
  artifacts (the Results sections are curated prose).

## Data flow invariants

- One recording failure never aborts a dataset run; one dataset failure
  never aborts `run --dataset all`.
- Every number in the report is traceable to a row in
  `validation/results/**/metrics.csv` (or `metrics_extra.csv` for the
  supplementary analyses), and every row to a manifest with exact versions.
- "Validated" means all recordings of the dataset succeeded; partial and
  failed states are first-class statuses, never silently filtered.
