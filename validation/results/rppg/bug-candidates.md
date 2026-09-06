# rPPG-group bug candidates

Template follows `docs/validation/BUGS.md`. Statuses:
confirmed_defect / suspected_defect / dataset_adapter_issue / ambiguous /
expected_limitation. Validation findings only — no Lamina source modified.

## BUG-R01 — Inconsistent output sign convention across rppg algorithms silently degrades `ppg-peaks` HR for `green`/`pos`

### Status
suspected_defect

### Component
`rppg` — `RppgAlgorithm::{GreenChannel, Pos}::extract_window` vs
`Chrom::extract_window` (exercised via bridge op `rppg-algorithm`), in
composition with `ppg_findpeaks_config` (Elgendi 2013, morphology-sensitive).

### Dataset
scamps example set (synthetic, 10 videos, ground-truth `d_ppg` embedded
per-frame; sync exact by construction).

### Reproduction
```
python -m validation run --dataset scamps --results-dir validation/results/rppg --seed 42
```
then compare waveform sign and downstream HR per algorithm (values below from
`metrics.csv` / `metrics_extra.csv`). Sign-flip probe (validation-side):
`ppg_peaks(-waveform, fs)` vs `ppg_peaks(waveform, fs)`.

### Input
Per-frame skin-mask RGB means from SCAMPS `Xsub` face crops, uniform 30 fps,
20 s per video, GT HR 84.5–131.5 bpm.

### Expected Behavior
All three algorithms should emit waveforms in a consistent phase convention
(or document that theirs differ), so that feeding any of them into
`ppg-peaks` — the documented downstream path in API-INVENTORY §rppg —
yields comparable HR accuracy.

### Actual Behavior
- `chrom` output is in blood-volume phase (raw waveform r vs GT: +0.36 mean,
  positive on 7/10 videos) and works directly with `ppg-peaks`
  (peak F1 0.768).
- `green` and `pos` outputs are in *inverted* (intensity) phase (raw r:
  -0.921 and -0.823 mean; sign-corrected |r| = 0.921 / 0.823 — i.e. the
  underlying traces are excellent). Fed as-is into `ppg-peaks`, green's peak
  F1 collapses to 0.245 and pos HR MAE is worst (16.2 bpm).
- Sign-flip probe (3 videos): HR MAE with `ppg_peaks(-waveform)` vs as-is —
  green: 8.32/1.24/1.39 vs 11.68/5.79/6.51; pos: 7.73/1.42/0.95 vs
  27.50/12.17/11.67; chrom gets *worse* when flipped (10.75/21.40/24.75 vs
  2.26/10.58/1.19), confirming a phase-convention difference, not a generic
  detector asymmetry.
- Additionally, `chrom`'s sign is itself unstable across videos (raw r
  negative on P000004/P000005/P000006), which is expected for rPPG algorithms
  in general but reinforces that the composition contract with `ppg-peaks`
  is sign-fragile.

### Evidence
`validation/results/rppg/metrics.csv` (per-recording hr_* and
ppg_rppg_waveform_* metrics), `validation/results/rppg/metrics_extra.csv`
(raw and sign-corrected waveform correlations), sign-flip probe numbers
above (reproducible with the adapter's `_load_video` traces).

### Severity
Medium (silent accuracy degradation on the documented downstream path;
no error is raised).

### Suggested Investigation
Decide a crate-wide phase convention (e.g. blood-volume/BVP phase) and
normalize `GreenChannel`/`Pos` outputs to it, or document per-algorithm sign
behavior and/or make `ppg-peaks` sign-robust. Physically, camera intensity
decreases as blood volume increases, so an intensity-phase output is not
"wrong" — the defect, if any, is the undocumented inconsistency between
algorithms and the sign-sensitive composition with Elgendi peak detection.

### Scope
Validation finding only. No source modification performed.

---

## BUG-R02 — setup-env.sh copies the bridge binary under the wrong filename

### Status
dataset_adapter_issue (validation-framework infra, not Lamina proper)

### Component
`validation/scripts/setup-env.sh` (framework script; not modifiable by
adapter agents).

### Reproduction
Run `sh validation/scripts/setup-env.sh <repo>` on a clean machine, then
`python -m validation run ...` → `LaminaBridgeError` / FileNotFoundError for
`validation/lamina_bridge/target/release/lamina_bridge`.

### Expected / Actual
Expected: script installs the built bridge where `bridge.py` looks.
Actual: the script copies `$HOME/.lamina-bridge-target/release/lamina-bridge`
(hyphen) but cargo names the binary `lamina_bridge` (underscore, per
`lamina_bridge/Cargo.toml`), so the `cp` silently no-ops (`|| true`) and the
binary is missing at runtime. Workaround used here: manual `cp` of
`$HOME/.lamina-bridge-target/release/lamina_bridge` into
`validation/lamina_bridge/target/release/`.

### Severity
Low (one-time environment friction; easy to diagnose).

### Scope
Validation finding only. No source modification performed.
