# Lamina Validation Framework

Reproducible public-dataset validation suite for the Lamina crate.
**Validation-only**: Lamina (`src/`, `tests/`, `benches/`, root `Cargo.toml`,
`lamina_dart/`) is the implementation under test and is never modified. The
binding contract is [SPEC.md](./SPEC.md); verified API signatures live in
[API-INVENTORY.md](./API-INVENTORY.md).

## Layout

- [`lamina_bridge/`](./lamina_bridge/) — Rust bin crate: JSON-in/JSON-out CLI bridge over Lamina
  (SPEC §3). Exposes native Rust signal algorithms to Python. See [`lamina_bridge/README.md`](./lamina_bridge/README.md).
- [`notebooks/`](./notebooks/) — Interactive Jupyter notebooks (01 to 05) for end-to-end dataset validation and visualization. See [`notebooks/README.md`](./notebooks/README.md).
- `validation/` — Python package: canonical `Signal` schema, bridge client,
  metrics, 18-dataset registry, runner, manifest, report, consolidation
  (`consolidate.py`), CLI (SPEC §4–§7).
- `run_phase*.py` — Phased validation suite scripts (Phases 1 through 12) for detailed algorithm, signal morphology, cross-language, and hygiene testing.
- `fixtures/` — tiny committed synthetic signals with known ground truth (§8).
- `tests/` — pytest suite for the framework itself (no network, no downloads).
- `results/` — dataset run outputs (SPEC §6), see "Results layout" below.
- `revalidation/`, `revalidation-v2/`, `revalidation-v3/` — Historical revalidation run reports, baseline comparisons, and findings.
- `remediation-v2/`, `remediation-v3/`, `remediation-v4/` — Independent remediation audit reports (e.g. REV3-001 through REV3-004) and validation metric verifications.

## Quickstart

```bash
# 1. Environment Bootstrap: Rust toolchain, Python deps, build lamina_bridge
sh validation/scripts/setup-env.sh "$(pwd)"

# 2. Alternatively, build lamina_bridge manually via Cargo:
cargo build --release --manifest-path validation/lamina_bridge/Cargo.toml

# 3. Run dataset-level validation runner (from repository root):
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

## Lamina Bridge Architecture

### Purpose & Design Motivation

`lamina_bridge` is a dedicated JSON CLI inter-process communication (IPC) binary compiled from Rust (`validation/lamina_bridge/`). It acts as a lightweight, zero-overhead bridge between the Python validation framework/Jupyter notebooks and the core Lamina Rust implementation (`src/`).

Why `lamina_bridge` exists:
- **Pure Rust Isolation**: Lamina's core crate (`src/`) remains 100% pure Rust and is tested without modifying its source or wrapping it in unsafe C-extensions / PyO3 bindings.
- **Panic Safety**: Every operation dispatch inside the bridge is wrapped in Rust's `std::panic::catch_unwind`. A panic in Lamina will never crash the calling Python script or Jupyter kernel; instead, the bridge intercepts the unwind and returns a structured JSON error envelope (`exit code 2`).
- **Cross-Language Verification**: Exercises exact JSON serialization, deserialization, enum alignments, and algorithm contracts shared between Rust, Python, and Dart.

### CLI Protocol & Protocol Contract

The bridge reads input payloads from disk, executes the requested operation, and writes output envelopes:

```bash
lamina_bridge --op <OP> --input <in.json> --output <out.json>
```

Checking versions (no input file required):
```bash
validation/lamina_bridge/target/release/lamina_bridge version
```

**Exit Codes**:
- `0`: Success (`ok: true` envelope written).
- `1`: Structured Lamina domain error (`SignalError`).
- `2`: Caught Rust panic (intercepted via `catch_unwind`).
- `3`: Protocol / IO error (invalid CLI flags, unparseable input JSON, missing files).

### Python Client (`validation/validation/bridge.py`)

The Python class `LaminaBridge` manages binary resolution, invocation, and temporary JSON file IO:

```python
from validation.bridge import LaminaBridge

# Use prebuilt release binary
bridge = LaminaBridge(auto_build=False)

# Clean ECG signal (360 Hz)
cleaned = bridge.ecg_clean(raw_signal, fs=360.0)

# Extract QRS peaks
peaks_info = bridge.ecg_peaks(cleaned, fs=360.0)
qrs_indices = peaks_info["peaks"]
```

---

## Jupyter Notebooks & Interactive Usage

The [`notebooks/`](./notebooks/) directory provides 5 self-contained, interactive validation notebooks:

| Notebook | Domain | Dataset & Operations |
| :--- | :--- | :--- |
| [`01_ecg_mitdb.ipynb`](./notebooks/01_ecg_mitdb.ipynb) | **ECG Beat Detection** | MIT-BIH record 100 QRS detection (`ecg-clean`, `ecg-peaks`) vs `.atr` annotations |
| [`02_wearable_ppg_wrist.ipynb`](./notebooks/02_wearable_ppg_wrist.ipynb) | **PPG Pulse Peaks** | PPG-BP database wrist PPG filtering and pulse peak detection (`ppg-clean`, `ppg-peaks`) |
| [`03_eda_wesad.ipynb`](./notebooks/03_eda_wesad.ipynb) | **Electrodermal Activity** | WESAD dataset skin conductance response (SCR) and tonic/phasic decomposition (`eda-clean`, `eda-decompose`) |
| [`04_respiration_bidmc.ipynb`](./notebooks/04_respiration_bidmc.ipynb) | **Respiration Rate** | BIDMC dataset breath cycle extraction and RR estimation (`rsp-clean`, `rsp-cycles`) |
| [`05_rppg_scamps.ipynb`](./notebooks/05_rppg_scamps.ipynb) | **Remote PPG (rPPG)** | SCAMPS video-derived facial BVP extraction and polarity auto-detection (`rppg-algorithm`, `rppg-polarity`) |

### Prerequisites for Running Notebooks

Before starting a Jupyter environment, perform the following two setup steps:

#### Step 1: Install Rust & Python Dependencies

Make sure Rust (`cargo`) is installed on your system. If needed, install Rust from [rustup.rs](https://rustup.rs).

Install required Python libraries:
```bash
pip install wfdb numpy scipy pandas matplotlib jupyter nbconvert
```

#### Step 2: Build `lamina_bridge` in Release Mode

The notebooks instantiate `LaminaBridge(repo_root=REPO_ROOT, auto_build=False)`. Because `auto_build=False` is set to prevent background compilation delays during interactive execution, **`lamina_bridge` MUST be built beforehand**.

Run either of the following commands from your terminal:

```bash
# Option A: From repository root
cargo build --release --manifest-path validation/lamina_bridge/Cargo.toml

# Option B: From validation/lamina_bridge directory
cd validation/lamina_bridge && cargo build --release
```

Alternatively, run the environment setup script:
```bash
sh validation/scripts/setup-env.sh "$(pwd)"
```

#### Step 3: Launch Notebooks

```bash
jupyter notebook validation/notebooks/01_ecg_mitdb.ipynb
```

Or execute headlessly:
```bash
jupyter nbconvert --execute --inplace validation/notebooks/01_ecg_mitdb.ipynb
```

---

## Troubleshooting & Common Errors

| Error Symptom | Cause | Solution |
| :--- | :--- | :--- |
| `FileNotFoundError: [Errno 2] No such file or directory: '.../validation/lamina_bridge/target/release/lamina_bridge'` | The `lamina_bridge` Rust binary has not been compiled yet. Notebooks set `auto_build=False` and require a pre-built binary. | Run `cargo build --release --manifest-path validation/lamina_bridge/Cargo.toml` from the repository root before running notebooks. |
| `LaminaBridgeError: cargo not found on PATH; cannot auto-build lamina_bridge` | Rust toolchain is missing or `$HOME/.cargo/bin` is not in environment `PATH`. | Install Rust from [rustup.rs](https://rustup.rs) or add `export PATH="$HOME/.cargo/bin:$PATH"` to your environment. |
| `ModuleNotFoundError: No module named 'validation'` | Python cannot locate the validation package when executing scripts or notebooks. | Notebooks automatically resolve `REPO_ROOT` and append it to `sys.path`. When running Python scripts directly, run from repo root via `python -m validation ...` or set `PYTHONPATH=.`. |
| `FileNotFoundError: .../data/mitdb/100.dat` | Record data has not been cached locally yet. | Notebook cells and `wfdb` automatically fetch credential-free datasets into `$HOME/data`. Ensure internet connectivity on first run, or download via `python -c "import wfdb; wfdb.dl_database('mitdb', dl_dir='$HOME/data/mitdb', records=['100'])"`. |

---

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

## Regenerating fixtures

```bash
python -m validation.fixtures_gen
```

## Statuses

`validated` · `partially_validated` · `inaccessible` · `unsupported_format` ·
`failed` · `not_attempted`. Potential Lamina bugs discovered during validation
are documented (never fixed) in `docs/validation/BUGS.md`.


