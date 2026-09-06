# Lamina Public-Dataset Validation Framework — SPEC (The Contract)

Version: 1.0 (Stage 1 — framework core). This document is the binding contract for the
validation framework. Adapter agents (Stage 2+) MUST conform to the interfaces defined here.

Hard constraint: **validation only**. Nothing outside `validation/` and `docs/validation/`
may be modified (`.gitignore` excepted). Lamina (`src/`, `tests/`, `benches/`, root
`Cargo.toml`, `lamina_dart/`) is the implementation under test and is never changed.

---

## §1. File layout

```text
validation/
├── SPEC.md                     # this file
├── API-INVENTORY.md            # verified Lamina public API signatures
├── README.md                   # quickstart
├── requirements.txt            # Python deps
├── __init__.py                 # repo-root namespace shim (redirects to validation/validation/)
├── lamina_bridge/              # Rust bin crate: JSON bridge to Lamina (§3)
│   ├── Cargo.toml
│   ├── build.rs                # extracts LAMINA_VERSION from ../../Cargo.toml
│   └── src/main.rs
├── validation/                 # Python package (§4)
│   ├── __init__.py
│   ├── __main__.py             # CLI (§7)
│   ├── _version.py
│   ├── schema.py               # canonical Signal representation (§2)
│   ├── bridge.py               # LaminaBridge (auto-build + typed wrappers)
│   ├── registry.py             # 18-dataset registry (§9)
│   ├── runner.py               # run_dataset / run_all + result writers (§6)
│   ├── manifest.py             # reproducibility manifest (§6)
│   ├── report.py               # human-readable report generator
│   ├── fixtures_gen.py         # deterministic fixture generator (§8)
│   ├── metrics/
│   │   ├── __init__.py
│   │   ├── events.py           # event matching + peak detection metrics (§5)
│   │   ├── rate.py             # rate metrics (§5)
│   │   └── hrv.py              # HRV metrics (§5)
│   └── datasets/
│       ├── __init__.py
│       ├── base.py             # DatasetAdapter ABC, DatasetInfo, AccessReport (FINAL)
│       ├── ecg.py              # stub adapters (Stage 2 TODO)
│       ├── ppg.py
│       ├── autonomic.py
│       └── rppg.py
├── fixtures/                   # committed tiny synthetic .npz fixtures (§8)
├── tests/                      # pytest suite for the framework itself (§8)
└── results/                    # gitignored run outputs (§6)
docs/validation/
├── BUGS.md                     # bug log (taxonomy header; entries appended, never fixed here)
└── public-dataset-validation.md# generated report
```

`python -m validation ...` MUST work with CWD = repository root. The root-level
`validation/__init__.py` shim redirects the package `__path__` to `validation/validation/`
so both `python -m validation` (from repo root) and `import validation.*` resolve the same
submodules. All package code lives in submodules; both `__init__.py` files stay minimal.

---

## §2. Canonical signal schema (`schema.py`)

```python
@dataclass
class Signal:
    samples: np.ndarray          # 1-D float64
    sampling_rate: float         # Hz, > 0, finite
    modality: str                # one of MODALITIES
    timestamps: np.ndarray | None = None   # seconds; None = uniform from t=0
    units: str | None = None
    subject_id: str | None = None
    recording_id: str | None = None
    channel: str | None = None
    annotations: dict            # e.g. {"peak_indices": np.ndarray}
    metadata: dict
```

Modalities: `ecg, ppg, bvp, eda, rsp, acc, gyro, temp, abp, rgb_video, thermal_video`.
Helpers: `validate()`, `n_samples`, `duration_sec`, `get_timestamps()`,
`resample_signal(signal, target_fs)` (scipy `resample_poly` polyphase, rational ratio).

---

## §3. Bridge protocol (`lamina_bridge`)

Lamina is exercised **only** through the compiled Rust bridge binary
(`validation/lamina_bridge`), so the Rust crate remains the implementation under test.

### §3.1 Invocation

```bash
lamina_bridge --op <OP> --input <in.json> --output <out.json>
lamina_bridge version            # version op: no input file required, JSON to stdout
```

Input envelope (all fields optional unless the op requires them):

```json
{
  "signal": [1.0, "..."],
  "sampling_rate": 360.0,
  "config": { "...op-specific fields, absent = Lamina default..." },
  "peaks": [100, 460], "signal_length": 10800,
  "timestamps_sec": [], "red": [], "green": [], "blue": [], "valid_pixel_counts": []
}
```

Output envelope (always written, even on failure):

```json
{
  "ok": true,
  "op": "ecg-peaks",
  "lamina_version": "0.1.0",
  "bridge_version": "0.1.0",
  "result": { "...op-specific..." },
  "error": null
}
```

On failure `ok=false`, `result=null`, and
`error = {"kind": "bad_request"|"lamina_error"|"panic", "message": "..."}`.

Exit codes: `0` ok · `1` Lamina returned `Err` · `2` panic caught (`catch_unwind`
wraps every op dispatch; a panic NEVER aborts the process without a JSON error) ·
`3` protocol/IO/JSON error (malformed input, missing required field, unknown op).

Non-finite floats (`NaN`, `±inf`) are serialized as JSON `null`; ops that can
legitimately produce non-finite values (e.g. `sample-entropy`) additionally emit an
explicit boolean flag.

### §3.2 Ops

Config JSON fields map 1:1 onto Lamina config-builder methods; **an absent field means
the Lamina default is used** (the bridge never substitutes its own defaults, except
where noted). Pipeline semantics mirror Lamina's own tests.

| op | Lamina calls | input | result |
|----|--------------|-------|--------|
| `version` | — | — | `lamina_version`, `bridge_version` |
| `ecg-clean` | `ecg_clean(sig, fs, method)` | signal, fs; cfg `method` (default `"none"`) | `signal` |
| `ecg-peaks` | `ecg_clean(sig, fs, "none")` → `ecg_findpeaks_config(cleaned, fs, cfg)` | signal, fs; cfg = `EcgPeakDetectionConfig` fields (`lowcut, highcut, filter_order, integration_window_sec, refractory_period_sec, searchback, threshold_multiplier`); `return_cleaned: bool` (bridge-only, default false) | `peaks` (0-based sample indices), `count`, optional `cleaned` |
| `ppg-clean` | `ppg_clean(sig, fs)` | signal, fs | `signal` |
| `ppg-peaks` | `ppg_clean` → `ppg_findpeaks_config(cleaned, fs, cfg)` | signal, fs; cfg = `PpgPeakDetectionConfig` fields (`lowcut, highcut, filter_order, w_peak_sec, w_beat_sec, alpha, refractory_period_sec`) | `peaks`, `count` |
| `eda-clean` | `eda_clean(sig, fs)` | signal, fs | `signal` |
| `eda-decompose` | `eda_decompose(sig, fs, cfg)` | signal, fs; cfg `tonic_cutoff_hz, filter_order` | `tonic`, `phasic` |
| `eda-peaks` | `eda_clean` → `eda_decompose(cleaned, fs, decfg)` → `eda_findpeaks_events(phasic, fs, peakcfg)` | signal, fs; cfg = decompose fields + `EdaPeakDetectionConfig` fields (`min_amplitude, min_prominence, min_distance_sec, min_rise_time_sec, max_rise_time_sec`) | `events`: list of `{onset_index, peak_index, amplitude, rise_time_sec}`; `peaks` (peak indices); `count` |
| `rsp-clean` | `rsp_clean_config(sig, fs, cfg)` | signal, fs; cfg `lowcut, highcut, filter_order` | `signal` |
| `rsp-cycles` | `rsp_cycles_config(raw, fs, cfg)` **and** `rsp_rate_config(raw, fs, cfg)` (both on the RAW input — Lamina re-cleans internally) | signal, fs; cfg = `RspProcessingConfig` fields (`lowcut, highcut, filter_order, min_breath_interval_sec, max_breath_interval_sec, min_amplitude`) | `cycles`: list of `{inspiration_index, expiration_index, next_inspiration_index, duration_sec, respiratory_rate_bpm, amplitude}`; `rate` (per-sample bpm); `count` |
| `hrv` | bool mask from peak indices → `peaks_to_intervals(mask, fs)` → `hrv_rmssd`, `hrv_mean_nn` | `peaks` (indices), `signal_length`, fs | `intervals_ms`, `n_intervals`, `rmssd_ms` (null when insufficient), `mean_nn_ms` (null when insufficient) |
| `signal-peaks` | `signal_findpeaks_config(sig, cfg)` | signal; cfg = `PeakDetectionConfig` fields (`min_height, min_distance, min_prominence, min_width, threshold`) | `peaks`, `count` |
| `filter` | `FilterSpec::{lowpass,highpass,bandpass,notch}` + `signal_filtfilt` (`zero_phase=true`, default) or `SosFilter::forward` | signal, fs; cfg `kind` (required), `cutoff` or `cutoffs` (2 for bandpass/notch), `order` (bridge default 2), `zero_phase` | `signal` |
| `sample-entropy` | `sample_entropy(sig, m, r)` | signal; cfg `m` (bridge default 2), `r` (absent → `0.2 * std(signal)`) | `sample_entropy` (null if non-finite), `is_infinite`, `m`, `r` |
| `rppg-algorithm` | build `OpticalSignal` from vectors → per-window `preprocess` → `<Algorithm>.extract_window(&win, &RppgConfig)`; windows slide per `RppgWindowConfig` and outputs are concatenated (overlap-trimmed: samples already covered by an earlier window are dropped) | `timestamps_sec, red, green, blue` (+ optional `valid_pixel_counts`); cfg `algorithm` (`green`\|`chrom`\|`pos`, default `chrom`), `window_sec, step_sec, min_window_fraction, normalize_channels, detrend` | `waveform`, `timestamps_sec`, `algorithm`, `n_windows`, `mean_sampling_rate_hz` |

Notes:
- The `rppg-algorithm` op exercises the raw algorithm layer (`RppgAlgorithm::extract_window`)
  over sliding windows; it intentionally does NOT reproduce `extract_rppg`'s
  quality-weighted overlap-add (which requires video frames/ROI). Overlap-trimmed
  concatenation keeps each output sample exactly once.
- All peak indices are 0-based sample indices into the input signal.

---

## §4. Python package

- `schema.py` — §2.
- `bridge.py` — `LaminaBridge(repo_root=None, binary_path=None, auto_build=True, timeout_sec=120)`:
  locates `validation/lamina_bridge`, auto-builds via `cargo build --release` when the
  binary is missing or any `src/*.rs`/`Cargo.toml` is newer; `run_op(op, payload) -> dict`
  (raises `LaminaBridgeError` on exit code 3 or `ok=false`); typed wrappers
  (`version, ecg_clean, ecg_peaks, ppg_clean, ppg_peaks, eda_clean, eda_decompose,
  eda_peaks, rsp_clean, rsp_cycles, hrv, signal_peaks, filter, sample_entropy,
  rppg_algorithm`) returning the `result` payload.
- `registry.py` — §9.
- `runner.py` — §6.
- `manifest.py` — §6.
- `report.py` — §6/§13-of-task.
- `metrics/` — §5.

## §5. Metrics

- `metrics/events.py`
  - `match_events(reference_indices, detected_indices, fs, tolerance_sec) -> MatchResult`
    Greedy one-to-one: iterate reference events in ascending order; for each, take the
    nearest not-yet-matched detection within `±tolerance_sec`; a detection matches at
    most one reference. Returns matched pairs + timing errors (sec, detected − reference).
  - `peak_detection_metrics(...) -> dict`: `tp, fp, fn, precision, recall, f1,
    mean_abs_timing_error_sec, median_timing_error_sec, n_reference, n_detected`
    (nulls when denominators are 0).
- `metrics/rate.py` — `rate_metrics(reference, estimated) -> dict`:
  `mae, rmse, bias, pearson_r, n` (elementwise over aligned arrays).
- `metrics/hrv.py` — `hrv_metrics(reference, estimated) -> dict`:
  `mae, rmse, bias, mean_relative_error, pearson_r, n`.
  Callers must distinguish Lamina-detected-beat HRV from annotation-derived HRV.

## §6. Runner, results, manifest

`runner.run_dataset(adapter, bridge, options) -> DatasetRunResult`:
- iterates `adapter.iter_recordings(...)` with deterministic sampling
  (`subjects`, `recordings`, `max_recordings`, `seed`, `smoke`);
- each recording processed in a `try/except` — one failure never aborts the dataset run;
  failures are recorded with traceback into `results/logs/<dataset>.log`;
- writes per-run: `recordings.csv` (one row per recording: dataset, subject, recording,
  modality, status, error), `metrics.csv` (long format: dataset, recording, metric,
  value), `summary.csv` (one row per dataset: status, subjects, recordings, key metrics),
  `datasets.json` (per-dataset structured metadata per task §9), `manifest.json`.

`manifest.build_manifest(repo_root, seed, tolerances, config) -> dict`: git commit via
`git rev-parse HEAD`, `lamina_version` (from bridge `version` op), Python version,
package versions via `importlib.metadata` (numpy, scipy, pandas, wfdb, matplotlib,
pytest), tolerances, seed, config snapshot, UTC timestamp. Written to
`results/manifest.json`.

`report.generate(results_dir, out_path)` → `docs/validation/public-dataset-validation.md`:
skeleton with Executive Summary, Dataset Inventory table (from `datasets.json` or, if
absent, from the registry with status `not_attempted`), Methodology, Results
placeholders, Robustness, Failures, Known/Potential Bugs (links BUGS.md), Limitations,
Reproducibility.

## §7. CLI (`python -m validation`)

```bash
python -m validation list [--category ecg|ppg|autonomic|rppg]
python -m validation check-access [--dataset KEY]
python -m validation run --dataset KEY|all [--smoke] [--subjects S ...] \
    [--recordings R ...] [--max-recordings N] [--seed N] [--results-dir DIR]
python -m validation report [--results-dir DIR] [--output PATH]
python -m validation test [-v]            # runs pytest validation/tests
```

`run --dataset all` attempts every dataset, skips inaccessible ones, continues after
individual failures, and records every status. Exit codes: 0 success; 1 any dataset
failed/inaccessible during `run` (results still written); 2 CLI usage error.

## §8. Fixtures & framework tests

`validation/validation/fixtures_gen.py` generates tiny deterministic fixtures (fixed
seed, committed, < 200 KB total) into `validation/fixtures/`:

| fixture | fs | duration | content |
|---|---|---|---|
| `ecg_synthetic.npz` | 360 Hz | 10 s | synthetic ECG (gaussian QRS train @ 60 bpm, 1 beat/s) + mild noise; `true_peaks` sample indices |
| `ppg_synthetic.npz` | 100 Hz | 12 s | 2-harmonic pulse wave @ 75 bpm; `true_peaks` |
| `rsp_synthetic.npz` | 100 Hz | 30 s | sinusoidal breathing @ 15 brpm; `true_peaks` (inspiratory peaks) |
| `eda_synthetic.npz` | 100 Hz | 30 s | 2 µS tonic + 3 known SCR bumps; `true_onsets`, `true_peaks` |

`validation/tests/` (pytest; no network, no large downloads; bridge tests build via
cargo and are skipped with a clear reason if cargo is unavailable):
- `test_schema.py` — Signal validation, timestamps, resampling helper.
- `test_metrics_events.py` — perfect match, tolerance boundary, greedy one-to-one
  (no double matching), FP/FN accounting, empty inputs.
- `test_metrics_rate.py` / `test_metrics_hrv.py` — known-value checks.
- `test_registry.py` — 18 datasets, unique keys, required info fields, honest
  placeholder `check_access` statuses.
- `test_bridge.py` — version op; fixture round-trips: `ecg-peaks` recovers > 80 % of
  true peaks within 150 ms; `ppg-peaks` count sanity; `filter` output length/finite;
  `sample-entropy` on fixture; `hrv` from known peak train.
- `test_runner.py` — run_dataset on a stub adapter records status, writers emit files.
- `test_manifest.py` — commit/versions/seed present.
- `test_cli.py` — `list` shows 18 datasets; `check-access` exits 0.

## §9. Dataset registry (18 datasets)

keys (category): `mit-bih-arrhythmia`, `mit-bih-noise-stress` (ecg);
`bidmc`, `wrist-ppg-exercise`, `pulsedb`, `mimic-iii-waveform`, `mimic-iii-ext-ppg` (ppg);
`wesad`, `autonomic-aging`, `wearable-exam-stress`, `big-ideas` (autonomic);
`pure`, `ubfc-rppg`, `cohface`, `ubfc-phys`, `mmpd`, `ibvp`, `scamps` (rppg).

Stage 1 ships stub adapters: each implements `info()` with real metadata (name, source
URL, modalities, license notes) and `check_access()` returning an HONEST placeholder
(`not_attempted`, reason "adapter not yet implemented — Stage 2 TODO"); credentialed
datasets note their access requirements in `info().notes`. Adapter agents replace stubs
without touching `datasets/base.py`, `registry.py` structure, metrics, or runner.

Dataset statuses: `validated, partially_validated, inaccessible, unsupported_format,
failed, not_attempted`.
---

## Appendix A — Revalidation extension (post-remediation)

Added during the independent revalidation of post-remediation Lamina
(`main @ fec2668`). **Additive only**: no existing op changed its JSON field
names or numerical semantics. This section is the binding contract for the
extension ops and for the revalidation-era adjustments below.

### A.1 Revalidation-era adjustments to existing ops

- **`ecg-clean` / `ecg-peaks` method mapping (BUG-001).** Post-remediation
  `ecg_clean` dispatches on the `method` string (`""`, `"neurokit"`,
  `"pantompkins"`, `"biosppy"` — all numerically the same 0.5 Hz HP order-5
  pipeline — and `SignalError::InvalidCutoffFrequency` otherwise). The bridge's
  historical default token `"none"` (ignored by pre-remediation Lamina) is now
  mapped to `""` at the bridge boundary, so both ops keep their baseline
  numerics and JSON schema. Callers may also pass the real method names.
- **`rsp-cycles` config gains optional `precleaned: bool` (BUG-007).** Maps to
  `RspProcessingConfig::with_precleaned`; absent = Lamina default (`false`,
  i.e. internal re-cleaning — baseline behavior unchanged).
- **`eda_findpeaks(phasic, fs)` / `rsp_findpeaks(cleaned, fs)` (BUG-004)** now
  take an explicit `sampling_rate`. The existing bridge ops only used the
  `*_config`/`eda_findpeaks_events` variants (already fs-explicit), so no
  bridge op semantics changed.

### A.2 New ops (ops-table addendum)

| op | Lamina calls | input | result |
|----|--------------|-------|--------|
| `eda-clean-config` | `eda_clean_config(sig, fs, EdaCleaningConfig)` | signal, fs; cfg = `EdaCleaningConfig` fields: `lowpass_cutoff_hz` (**tri-state**: absent = Lamina default `Some(5.0)`; explicit JSON `null` = `None`, no low-pass at all; number = cutoff Hz), `filter_order`, `pass_through_if_nyquist_violated` | `signal`, `filter_applied` (bool — false when pass-through path taken), `cutoff_hz` (resolved, null if disabled), `nyquist_hz`. Above-Nyquist cutoff with `pass_through_if_nyquist_violated: false` surfaces `lamina_error` (`InvalidCutoffFrequency`) |
| `rppg-polarity` | raw-algorithm sliding-window pipeline (identical to `rppg-algorithm`, incl. overlap-trimmed concatenation) → wrap in `RppgSignal` → `RppgSignal::to_bvp_waveform(SignalPolarity)` | identical to `rppg-algorithm` plus cfg `polarity`: `normal`\|`inverted`\|`auto` (default `normal`) | `waveform` (BVP samples), `timestamps_sec`, `sampling_rate_hz`, `algorithm`, `polarity_requested`, `polarity_resolved` (`normal`\|`inverted` — what `auto` inferred, resolved by comparing the returned BVP against the raw waveform), `flipped` (bool), `n_windows`, `mean_sampling_rate_hz` |
| `hrv-correct` | `peaks_to_intervals` (if peaks given) → `classify_intervals(rr, classify_threshold)` → `clean_rr_intervals(rr, CorrectionPolicy)` → `hrv_rmssd`, `hrv_mean_nn` | either `rr_intervals_ms` (float array, ms) **or** the `hrv`-op envelope (`peaks`, `signal_length`, `sampling_rate`); cfg: `policy` = `none`\|`reject_invalid`\|`interpolate_linear`\|`interpolate_cubic`\|`percent_threshold` (default `none`; `percent_threshold` requires cfg `percent_threshold`, 0.0 < p < 1.0), `classify_threshold` (absent = Lamina default 0.20) | `nn_intervals_ms`, `n_input_intervals`, `n_nn`, `interval_quality` (per-input-interval kind: `normal_nn`\|`ectopic_rr`\|`artifact_rr`\|`missing`), `policy` (echo), `rmssd_ms`, `mean_nn_ms` (null when insufficient; mirrors `hrv` op). Empty interval input yields empty outputs + null metrics (not an error) |

### A.3 Notes and known API gaps (observed, not fixed)

- There is **no public `sdnn` function** in `lamina::hrv`; `hrv-correct`
  therefore returns `rmssd_ms` and `mean_nn_ms` only. (`sdnn` exists only
  inside `features::cardiac::CardiacFeatures`, not as a standalone op input.)
- There is **no public per-beat `BeatQuality` classifier**: the `BeatQuality`
  enum is exported (`lamina::hrv::BeatQuality`) but no public function
  produces or consumes it, so the planned `beat-quality` op was **not added**
  (it would require fabricating classifications bridge-side — a hack).
- `classify_intervals` hardcodes the 300–2000 ms artifact bounds; the
  `IntervalCleaningConfig` (`min_valid_interval_ms` / `max_valid_interval_ms`)
  sketched in `docs/validation/REMEDIATION_PLAN.md` §4.4 does **not** exist in
  the shipped API (`clean_rr_intervals` takes only `rr` + `policy`).
- `CorrectionPolicy::InterpolateCubic` currently executes the **same linear
  interpolation code path** as `InterpolateLinear` (shared match arm in
  `src/hrv/quality.rs`); it does not perform cubic spline interpolation as the
  remediation plan describes.

### A.4 Python wrappers & tests

`bridge.py` gains typed wrappers `eda_clean_config(signal, fs, config)`,
`rppg_polarity(ts, r, g, b, polarity=..., ...)`, and
`hrv_correct(rr_intervals_ms=None, *, peaks=..., signal_length=..., fs=..., policy=...)`.
Hermetic tests live in `validation/tests/test_revalidation_ops.py` (same
fixtures + inline synthetics; no network).
