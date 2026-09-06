# ECG Validation — Bug Candidates

Findings from validating Lamina 0.1.0 (git 3856c2a) against mit-bih-arrhythmia
(mitdb) and mit-bih-noise-stress (nstdb) via the `lamina_bridge` JSON bridge.
Entry format follows `docs/validation/BUGS.md`. Validation findings only — no
source modification performed. Entries are staged here (allowed write scope for
the ECG validation agent); the framework owner can promote them to
`docs/validation/BUGS.md`.

## BUG-ECG-001 — `ecg_clean` silently ignores its `method` argument

### Status
Confirmed

### Component
Lamina `ecg_clean(signal, fs, method)` via bridge op `ecg-clean`
(config field `method`).

### Dataset
mit-bih-arrhythmia 1.0.0, record 100, first 100 s of MLII @ 360 Hz
(reproduces on any input).

### Reproduction
```python
from validation.bridge import LaminaBridge
b = LaminaBridge(auto_build=False)
sig = ...  # any finite ECG segment
a = b.ecg_clean(sig, 360.0, method="none")
c = b.ecg_clean(sig, 360.0, method="neurokit")
d = b.ecg_clean(sig, 360.0, method="nonexistent-method-xyz")
assert np.array_equal(a, c) and np.array_equal(a, d)  # holds
```

### Input
Finite ECG segment (36000 samples @ 360 Hz, mitdb/100 MLII).

### Expected Behavior
Different `method` values select different cleaning pipelines (per the API
signature), and unknown method names should be rejected with an error.

### Actual Behavior
Output is byte-identical for `none`, `neurokit`, and a bogus method string;
no error is raised. Matches API-INVENTORY.md sharp edge #1 (`method` ignored,
always 0.5 Hz high-pass order 5).

### Evidence
`max abs diff` = 0.0 for all pairs; verified 2026-09-06 against bridge
`lamina_version 0.1.0`.

### Severity
Low (documented API footgun; default pipeline itself performs well:
mean mitdb F1 = 0.988).

### Suggested Investigation
`ecg_clean` parameter is `_method: &str` — either implement method dispatch
or change the signature to avoid implying configurability.

### Scope
Validation finding only. No source modification performed.

---

## BUG-ECG-002 — `ecg-peaks` fails with "Input signal contains non-finite
values" whenever `threshold_multiplier >= 1.0`, on provably finite input

### Status
Confirmed

### Component
Lamina `ecg_findpeaks_config` / `EcgPeakDetectionConfig.threshold_multiplier`
via bridge op `ecg-peaks`.

### Dataset
mit-bih-arrhythmia 1.0.0 (record 100 MLII, 650000 samples @ 360 Hz) and a
30 s synthetic sinusoid + noise; both verified finite
(`np.isfinite(sig).all() == True`).

### Reproduction
```python
from validation.bridge import LaminaBridge
b = LaminaBridge(auto_build=False)
b.ecg_peaks(sig, 360.0, config={"threshold_multiplier": 1.0})
# LaminaBridgeError: bridge op 'ecg-peaks' failed [lamina_error]:
# Input signal contains non-finite values (NaN or Infinity)
```

### Input
Any finite signal tested. Boundary is exact: 0.99 works, 1.0 and 1.0000001
fail. All other config fields tested (`refractory_period_sec`,
`integration_window_sec`, `lowcut`/`highcut`, `searchback`, `filter_order`)
work normally.

### Expected Behavior
A valid config value (the field is a user-facing multiplier of the detection
threshold; Lamina default is 0.25) should either be accepted or rejected with
a config-related error message.

### Actual Behavior
Every `threshold_multiplier >= 1.0` returns `lamina_error` with the message
"Input signal contains non-finite values (NaN or Infinity)", which is factually
wrong about the input — either the check mis-attributes an internal NaN
(e.g. division inside threshold adaptation) or a config-range validation
reuses the wrong error variant.

### Evidence
Boundary sweep on mitdb/100 and synthetic signal (0.9 ok, 1.0/1.1/1.5/2.0/3.0
all fail with identical message), run 2026-09-06, lamina_version 0.1.0.

### Severity
Medium — silently blocks legitimate threshold configurations and misleads
users with a wrong diagnosis; default (0.25) is unaffected, so the main
validation runs are not impacted.

### Suggested Investigation
Trace where `threshold_multiplier` enters the adaptive-threshold computation
in `ecg/peaks.rs`; check for a `>= 1.0` validation branch returning
`EmptySignal`/non-finite error, or a threshold estimator producing NaN when
the multiplier reaches 1.0 (e.g. normalization by `(1 - multiplier)`).

### Scope
Validation finding only. No source modification performed.

---

## BUG-ECG-003 — Default `ecg-peaks` under-detects low-amplitude normal beats
in the presence of tall PVCs (mitdb/228 recall 0.60)

### Status
Suspected

### Component
Lamina `ecg_findpeaks` adaptive threshold (default
`EcgPeakDetectionConfig`: lowcut 5 / highcut 15 Hz, integration window
0.150 s, refractory 0.200 s, searchback on, threshold_multiplier 0.25).

### Dataset
mit-bih-arrhythmia 1.0.0, record 228 (MLII @ 360 Hz; multiform PVCs,
first-degree AV block per header comments).

### Reproduction
```bash
python -m validation run --dataset mit-bih-arrhythmia --recordings 228 \
    --seed 42 --results-dir validation/results/ecg
```
or `bridge.ecg_peaks(sig, 360.0)` on record 228 MLII and compare against
`wfdb.rdann('228', 'atr')` beat annotations at 150 ms tolerance.

### Input
Record 228 MLII: normal beats with R amplitude ~0.7 mV interleaved with PVCs
at ~2.2 mV (see evidence plot).

### Expected Behavior
Recall broadly comparable to other mitdb records (dataset median per-record
recall = 0.999); published Pan-Tompkins-class detectors achieve > 0.95 recall
on this record.

### Actual Behavior
recall 0.602, precision 0.993, F1 0.749. Missed beats cluster where small
normal beats follow tall PVCs: in a 20 s window at minute 10, 26 annotated
beats but only 6 detections — all 6 on the tall PVCs, all small normal beats
missed. The adaptive threshold appears to be pulled up by the tall PVC
amplitude and never recovers for the small normals (searchback does not
rescue them). With `threshold_multiplier: 0.1`, recall rises to 0.999
(F1 0.990), so the peaks are detectable by the same pipeline at a lower
threshold.

### Evidence
`validation/results/ecg/mit-bih-arrhythmia/metrics.csv` (record 228 row);
plot `validation/results/ecg/mit-bih-arrhythmia/record228_minute10.png`;
threshold sweep recorded above. Per-minute missed-beat histogram shows the
deficit concentrated in minutes 9–16 and 27–28 (818 of 2053 beats missed).

### Severity
Medium — affects recordings with large QRS amplitude disparity (e.g.
multiform ectopy); results remain excellent on typical records.

### Suggested Investigation
Check the learning/adaptation rates of the signal and noise level estimates
(SPKI/NPKI-style) in `ecg/peaks.rs`: after a run of large-amplitude beats the
signal-level estimate may dominate the threshold; consider bounding the level
estimates or weighting recent small peaks. Also verify the searchback
threshold ratio.

### Scope
Validation finding only. No source modification performed.

---

## BUG-ECG-005 — Paced-beat under-detection on mitdb/104 is strongly
channel-dependent (V5 F1 0.68 vs V2 F1 0.98)

### Status
Ambiguous

### Component
Lamina `ecg_findpeaks` default pipeline on paced/fusion-beat morphology.

### Dataset
mit-bih-arrhythmia 1.0.0, record 104 (no MLII; channels V5, V2; paced rhythm
with many pacemaker fusion beats per header comments).

### Reproduction
`bridge.ecg_peaks` on each channel of record 104, matched against
`wfdb.rdann('104', 'atr')` beat annotations at 150 ms tolerance.

### Input
Record 104, both channels @ 360 Hz.

### Expected Behavior
Reasonable beat recall on either ECG lead.

### Actual Behavior
V5: precision 0.998, recall 0.520, F1 0.683 (2229 reference beats, 1160
detected). V2: precision 0.970, recall 0.995, F1 0.982 (2287 detected).
The adapter's documented fallback (MLII absent → first channel) evaluates V5,
so the reported dataset metric for record 104 is the V5 number. It is unclear
how much of the gap is a Lamina morphology weakness vs a property of the V5
lead for this patient.

### Evidence
Per-channel metric comparison above, run 2026-09-06; adapter notes.

### Severity
Low

### Suggested Investigation
Inspect V5 segments around missed paced beats; compare against published
detectors on the same lead. Consider evaluating the better-quality lead when
MLII is absent (adapter-side choice) — not changed here to keep the fallback
rule simple and documented.

### Scope
Validation finding only. No source modification performed.

---

## BUG-ECG-006 — nstdb evaluation covers 7 of 12 records (dataset scope note)

### Status
Dataset-issue (scope limitation of this validation run, not a Lamina defect)

### Component
Dataset adapter `mit-bih-noise-stress`.

### Dataset
mit-bih-noise-stress 1.0.0 (nstdb).

### Reproduction
N/A — documentation note.

### Input
nstdb comprises 12 records (118e00..118e24, 119e00..119e24, plus 3 calibration
noise records bw/em/ma that carry no beat annotations).

### Expected Behavior
Full-database evaluation.

### Actual Behavior
physionet.org returns HTTP 403 from the validation host, so files were fetched
from the static mirror `https://archive.physionet.org/physiobank/database/nstdb`,
where `.dat` downloads are throttled to ~20 KB/s. 7 records were acquired
(118e00/06/12/18/24, 119e00/119e24 — full SNR grid for base record 118 plus
both extremes for 119). The remaining 119e06/12/18 were skipped to bound
download time (~15 min for 18 MB).

### Evidence
`validation/results/ecg/mit-bih-noise-stress/summary.csv` (7/7 ok);
adapter `info().notes`.

### Severity
Informational

### Suggested Investigation
If full coverage is required, download 119e06/12/18 from the mirror in the
background (~6 min at observed throttle) and re-run the adapter; the SNR
gradient is already fully covered by base record 118.

### Scope
Validation finding only. No source modification performed.
