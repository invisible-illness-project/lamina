# `lamina_bridge` — Rust JSON Bridge CLI

`lamina_bridge` is a dedicated Rust CLI binary crate (`validation/lamina_bridge`) that links directly against the core `lamina` crate (`src/`) and exposes Lamina's physiological signal processing algorithms via a JSON-in/JSON-out Inter-Process Communication (IPC) protocol.

---

## Purpose & Architecture

`lamina_bridge` serves as the primary runtime execution engine for the Lamina validation suite, evaluation scripts, and Jupyter notebooks.

### Why `lamina_bridge` Exists

1. **Validation Isolation**: The core Lamina library (`src/`) is the implementation under test. By invoking Lamina through a CLI bridge, the core Rust crate remains 100% pure Rust without requiring C-extensions, FFI bindings, or PyO3 wrappers that could alter memory management or signal behaviors.
2. **Panic Safety**: Every operation dispatch inside the bridge is wrapped in Rust's `std::panic::catch_unwind`. If an algorithm encounters a panic, `lamina_bridge` intercepts the panic safely and returns a structured JSON error envelope (`exit code 2`). The calling process (Python runner or Jupyter notebook kernel) never crashes.
3. **Cross-Language Protocol Parity**: Ensures that serialization/deserialization, numerical payload format, enum configurations, and signal signatures remain identical across Rust, Python, and Dart interfaces.

---

## Building `lamina_bridge`

### Prerequisites
- Rust toolchain (`cargo` and `rustc`). Install via [rustup.rs](https://rustup.rs) if needed.

### Build Command (Release Mode)

Always build in `--release` mode for maximum signal processing performance:

```bash
# From repository root:
cargo build --release --manifest-path validation/lamina_bridge/Cargo.toml

# Or from inside validation/lamina_bridge directory:
cd validation/lamina_bridge
cargo build --release
```

The compiled binary will be placed at:
```
validation/lamina_bridge/target/release/lamina_bridge
```
*(On Windows, the executable is `lamina_bridge.exe`)*

> [!NOTE]
> Jupyter notebooks in `validation/notebooks/` instantiate `LaminaBridge(auto_build=False)`. If `lamina_bridge` is not compiled prior to running a notebook, Python will raise a `FileNotFoundError`. Make sure to run `cargo build --release` first.

---

## CLI Specification & Operations Protocol

### Command Syntax

```bash
lamina_bridge --op <OPERATION> --input <INPUT_JSON_PATH> --output <OUTPUT_JSON_PATH>
```

#### Version Check (No input/output files required)
```bash
lamina_bridge version
```
Outputs JSON payload to stdout:
```json
{
  "bridge_version": "0.1.0",
  "lamina_version": "0.1.0"
}
```

### Exit Codes

| Code | Meaning | Envelope Format |
| :--- | :--- | :--- |
| `0` | **Success** | `{"ok": true, "op": "<OP>", "lamina_version": "...", "bridge_version": "...", "result": { ... }}` |
| `1` | **Lamina Domain Error** | `{"ok": false, "op": "<OP>", "error": {"kind": "lamina_error", "message": "..."}}` |
| `2` | **Caught Rust Panic** | `{"ok": false, "op": "<OP>", "error": {"kind": "panic", "message": "..."}}` |
| `3` | **Protocol / IO Error** | Invalid command-line arguments, file IO failure, or invalid input JSON payload. |

---

## Operations Inventory

| `--op` Flag | Function | Input Payload Keys | Result Keys |
| :--- | :--- | :--- | :--- |
| `version` | Query version info | *(none)* | `bridge_version`, `lamina_version` |
| `ecg-clean` | ECG filtering & baseline removal | `signal` (`Vec<f64>`), `sampling_rate` (`f64`), `config` (optional method) | `signal` (`Vec<f64>`) |
| `ecg-peaks` | QRS complex peak detection | `signal`, `sampling_rate`, `config` | `peaks` (`Vec<usize>`), `quality` |
| `ppg-clean` | PPG bandpass filtering | `signal`, `sampling_rate` | `signal` (`Vec<f64>`) |
| `ppg-peaks` | Systolic pulse peak detection | `signal`, `sampling_rate`, `config` | `peaks` (`Vec<usize>`), `onsets` |
| `eda-clean` | Skin conductance smoothing | `signal`, `sampling_rate` | `signal` (`Vec<f64>`) |
| `eda-decompose` | SCL (tonic) / SCR (phasic) separation | `signal`, `sampling_rate`, `config` | `tonic` (`Vec<f64>`), `phasic` (`Vec<f64>`) |
| `eda-peaks` | SCR peak event detection | `signal`, `sampling_rate`, `config` | `peaks` (`Vec<usize>`), `amplitudes` |
| `rsp-clean` | Respiration signal cleaning | `signal`, `sampling_rate`, `config` | `signal` (`Vec<f64>`) |
| `rsp-cycles` | Breath cycle detection | `signal`, `sampling_rate`, `config` | `extrema` (`Vec<usize>`), `rates` |
| `hrv` | Heart Rate Variability calculations | `peaks` (`Vec<usize>`), `signal_length` (`usize`), `sampling_rate` (`f64`) | `sdnn`, `rmssd`, `pnn50`, `lf_hf_ratio` |
| `hrv-correct` | RR interval cleaning & HRV pipeline | `rr_intervals_ms` or (`peaks`, `signal_length`, `sampling_rate`), `config` (policy) | `corrected_rr_ms`, `metrics` |
| `filter` | SosFilter digital filtering | `signal`, `sampling_rate`, `config` (`kind`, `cutoff`, `order`, etc.) | `signal` (`Vec<f64>`) |
| `sample-entropy` | Sample entropy calculation | `signal`, `config` (`m`, `r`) | `sample_entropy` (`f64`) |
| `rppg-algorithm` | Video-based rPPG pulse extraction | `timestamps_sec`, `red`, `green`, `blue`, `valid_pixel_counts`, `config` | `bvp_signal`, `snr_db` |
| `rppg-polarity` | rPPG BVP polarity resolution | `timestamps_sec`, `red`, `green`, `blue`, `config` (`polarity`: `normal`, `inverted`, `auto`) | `waveform`, `polarity_resolved`, `flipped` |

---

## Python Usage Example

```python
from validation.bridge import LaminaBridge

# Initialize client (uses validation/lamina_bridge/target/release/lamina_bridge)
bridge = LaminaBridge(auto_build=False)

# Check operational state
print("Version:", bridge.version())

# Run ECG peak detection
signal = [0.1, 0.2, 1.5, 0.2, 0.1] * 100
peaks_info = bridge.ecg_peaks(signal, fs=360.0)
print("Detected peaks:", peaks_info["peaks"])
```
