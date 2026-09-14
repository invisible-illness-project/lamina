# Lamina Validation Notebooks

Interactive Jupyter notebooks for step-by-step algorithm validation, signal morphology inspection, and performance benchmark visualization across physiological modalities (ECG, PPG, EDA, Respiration, rPPG).

## Notebook Inventory

| Notebook | Focus & Modality | Dataset | Bridge Operations Exercised |
| :--- | :--- | :--- | :--- |
| [`01_ecg_mitdb.ipynb`](./01_ecg_mitdb.ipynb) | **ECG Beat Detection** | [MIT-BIH Arrhythmia DB](https://physionet.org/content/mitdb/) (record 100) | `ecg-clean`, `ecg-peaks` |
| [`02_wearable_ppg_wrist.ipynb`](./02_wearable_ppg_wrist.ipynb) | **PPG Pulse Peaks & HRV** | [PPG-BP Database](https://figshare.com/articles/dataset/PPG-BP_Database/5459284) | `ppg-clean`, `ppg-peaks` |
| [`03_eda_wesad.ipynb`](./03_eda_wesad.ipynb) | **Electrodermal Activity** | [WESAD Dataset](https://archive.ics.uci.edu/ml/datasets/WESAD) | `eda-clean`, `eda-decompose`, `eda-peaks` |
| [`04_respiration_bidmc.ipynb`](./04_respiration_bidmc.ipynb) | **Respiration Rate** | [BIDMC PPG and Respiration DB](https://physionet.org/content/bidmc/) | `rsp-clean`, `rsp-cycles` |
| [`05_rppg_scamps.ipynb`](./05_rppg_scamps.ipynb) | **Remote PPG (rPPG)** | [SCAMPS Synthetic Video DB](https://github.com/microsoft/SCAMPS) | `rppg-algorithm`, `rppg-polarity` |

---

## Quickstart: How to Run the Notebooks

### 1. Prerequisites

Ensure you have Rust (`cargo`) and Python installed.

Install Python dependencies:
```bash
pip install wfdb numpy scipy pandas matplotlib jupyter nbconvert
```

### 2. Build the `lamina_bridge` Binary (REQUIRED)

> [!IMPORTANT]
> **Prevent `FileNotFoundError`**: All notebooks initialize the Rust bridge via `LaminaBridge(auto_build=False)`. To prevent unexpected compilation pauses during interactive notebook execution, the Python client expects a pre-compiled binary at `validation/lamina_bridge/target/release/lamina_bridge`.

Before opening or executing any notebook, compile `lamina_bridge`:

```bash
# From repository root:
cargo build --release --manifest-path validation/lamina_bridge/Cargo.toml

# Or from validation/lamina_bridge directory:
cd validation/lamina_bridge && cargo build --release
```

Alternatively, run the automated setup bootstrap:
```bash
sh validation/scripts/setup-env.sh "$(pwd)"
```

### 3. Launching Notebooks

#### Interactive Mode (Jupyter Web UI / VS Code)

```bash
jupyter notebook validation/notebooks/01_ecg_mitdb.ipynb
```

#### Headless Mode (Command Line Execution)

```bash
jupyter nbconvert --execute --inplace validation/notebooks/01_ecg_mitdb.ipynb
```

---

## Troubleshooting

### `FileNotFoundError: .../lamina_bridge/target/release/lamina_bridge`

**Symptom**:
```
FileNotFoundError: [Errno 2] No such file or directory: '/Users/.../lamina/validation/lamina_bridge/target/release/lamina_bridge'
```

**Cause**: You attempted to run a notebook cell before compiling the `lamina_bridge` Rust binary.

**Fix**: Run `cargo build --release --manifest-path validation/lamina_bridge/Cargo.toml` from the repository root, then re-run the notebook cell.

### Data Fetching & Caching

Datasets used in the notebooks are downloaded credential-free on first run into `$HOME/data/` (or the directory defined by the `LAMINA_NB_DATA` environment variable). Downloaded raw data files are never committed to the repository.
