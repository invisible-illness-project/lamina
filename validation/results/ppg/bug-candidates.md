# PPG-group validation — bug candidates

Taxonomy follows `docs/validation/BUGS.md` (`confirmed` / `suspected` /
`dataset-issue` / `ambiguous` / `limitation`); the task brief's terms map as
confirmed_defect→confirmed, suspected_defect→suspected,
dataset_adapter_issue→dataset-issue, expected_limitation→limitation.
No Lamina source was modified. All evidence reproducible with the commands in
the final report (branch `validation/adapters-ppg`).

---

## BUG-PPG-001 — ecg-peaks adaptive threshold catastrophically under-detects on clean narrow-biphasic QRS morphology and fails to recover

### Status
Suspected

### Component
`lamina::ecg::ecg_findpeaks_config` (Pan-Tompkins threshold/searchback logic), exercised via bridge op `ecg-peaks` with default config.

### Dataset
Wrist PPG During Exercise v1.0.0, record `s6_low_resistance_bike` (chest_ecg @ 256 Hz), PhysioNet `pn_dir='wrist'`.

### Reproduction
```
python -m validation run --dataset wrist-ppg-exercise --seed 42 --results-dir validation/results/ppg
# or directly: bridge ecg-peaks on chest_ecg of s6_low_resistance_bike (fs=256)
```

### Input
280 s chest-strap ECG @ 256 Hz. QRS = narrow biphasic spike (~+180/-260 ADC
units), beat every ~0.52 s (~115 bpm), visually clean; rolling 10 s
peak-to-peak amplitude steady (497-740 units, no dropout). Dataset `.atr`
annotations: 536 beats, one per QRS (verified by inspection, zoom 46-49 s).

### Observed
- Full-record run: 465 detected vs 536 annotated, but per-30-s detection
  counts are `[66, 2, 0, 3, 54, 71, 67, 105, 97]` vs annotations
  `[52, 57, 58, 59, 61, 62, 61, 62, 64]` — a ~90 s detection blackout
  (30-120 s) followed by ~1.6x over-detection (240-280 s).
- Isolated windows behave differently from the full record: window 30-60 s
  alone yields 3/54 beats (default config), while 60-90 s alone yields
  61/57. In the full-record run, 60-120 s is also blacked out: the failure
  on 30-60 s contaminates the adaptive threshold for the following ~60 s
  (searchback does not recover).
- Config sweep on window 30-60 s: `threshold_multiplier=0.1` (default 0.25)
  -> 68 detections (vs 54 annotated); `integration_window_sec=0.08` -> 6;
  `refractory_period_sec=0.15` -> 3. Sensitivity is dominated by the
  threshold multiplier.
- Band-passed (5-15 Hz, order 2, via bridge `filter` op) segment has clear
  QRS blobs: scipy `find_peaks(|bp|, distance=0.2 s)` finds 108 lobes
  (2 lobes per biphasic spike) — the front-end filter output is fine; the
  failure is in thresholding/decision, not filtering.

### Expected
A Pan-Tompkins-style detector should not miss >90% of beats for 30+ s on a
clean, steady-amplitude ECG, and searchback should recover the threshold
within a few seconds after a difficult segment.

### Assessment
Suspected defect in threshold estimation/adaptation (threshold_multiplier
relative to a mean/peak estimate that this morphology inflates) plus
insufficient searchback recovery. Impact: silent, large HR underestimation
followed by over-estimation on wearable chest ECG. Severity: Medium-High
for continuous-monitoring use. Cross-checks ruling out adapter issues:
annotations align exactly (ECG F1 0.88-1.00 on 14 of 19 records, e.g.
s3_run F1=0.999, s9_walk F1=0.993; the 5 records with F1<0.75 are the
motion/morphology cases discussed here and in BUG-PPG-002);
fs/index-base verified; same channel/timeline.

---

## BUG-PPG-002 — ecg-peaks / ppg-peaks over-count beats under periodic motion artifact (foot-strike spikes), inflating HR

### Status
Limitation (expected/by-design; recorded for visibility)

### Component
`lamina::ecg::ecg_findpeaks_config`, `lamina::ppg::ppg_findpeaks_config` (no motion-artifact rejection).

### Dataset
Wrist PPG During Exercise v1.0.0; clearest on `s1_walk` (chest_ecg and wrist_ppg @ 256 Hz).

### Reproduction
Same run command as BUG-PPG-001; metrics `ecg_ecg_f1`, `hr_mae`, `hr_bias`
in `validation/results/ppg/metrics.csv`; per-record consistency rows in
`metrics_extra.csv`.

### Observed
- `s1_walk`: Lamina ecg-peaks F1 = 0.667 vs dataset `.atr` annotations
  (973 detected vs 563 annotated); windowed HR from Lamina ecg-peaks
  differs from annotation HR by MAE 63.6 bpm. Lamina ppg-peaks HR bias
  +33.0 bpm vs annotation HR. Inspection (ECG/PPG 100-108 s) shows sharp
  foot-strike spikes (~0.55 s spacing, walking cadence) interleaved with
  true beats (~1.05 s spacing, HR ~57 bpm); Lamina accepts both.
- Dataset-wide, HR-from-PPG MAE vs annotation HR stratified by activity:
  walk 15.9 bpm (n=6), run 32.9 bpm (n=5), low-resistance bike 14.7 bpm
  (n=5), high-resistance bike 20.3 bpm (n=3). Errors grow with motion
  intensity, as expected for a detector without motion cancellation.

### Expected / assessment
No defect claimed: Lamina implements classical detectors without
accelerometer-guided artifact rejection, and this dataset is explicitly
designed to stress exactly that. Recorded as an expected limitation
(severity Medium for exercise-HR use) and as a quantitative motion-
stratified baseline. Note: per-record ECG F1 was low (<0.75) on 5 of 19
records — some of that may compound BUG-PPG-001 (threshold pathology), so
the two entries are not fully independent.

---

## BUG-PPG-003 — rsp-cycles over-detects breaths at very low respiratory rates with biphasic inspiratory morphology

### Status
Limitation (borderline ambiguous; see notes)

### Component
`lamina::rsp::rsp_cycles_config` (peak-picking cycle detector; no periodicity screening).

### Dataset
BIDMC PPG and Respiration Dataset v1.0.0, record `bidmc05` (RESP @ 125 Hz, mirror WFDB layout).

### Reproduction
```
python -m validation run --dataset bidmc --seed 42 --results-dir validation/results/ppg
```

### Observed
- `bidmc05`: F1 = 0.658 (precision 0.490, recall 1.000) vs ann1 breath
  annotations (48 breaths, ~6 brpm, intervals 9.7-14.3 s). Lamina detects
  98 cycles. Plot of band-passed RESP (0.05-0.5 Hz) shows a small
  pre-inspiratory bump ~4-5 s before each true inspiratory peak; Lamina
  counts both.
- Config relaxation (`max_breath_interval_sec=20`, `min_amplitude=0.15`)
  still yields 95 detections -> F1 0.671: not fixable via the exposed
  config; it is the peak-picking strategy itself.
- Note: annotated breath intervals up to 14.3 s also exceed the default
  `max_breath_interval_sec=12`, so some true cycles are structurally
  rejectable (masked here by dense false detections).

### Assessment
Mostly an expected limitation of a simple peak-based cycle detector at
~6 brpm with biphasic morphology (ICU patient). The hard-coded default
ceiling of 12 s/breath is worth documenting for low-RR populations.
Severity: Low. All other 11 BIDMC records: F1 0.93-0.99 (mean 0.944 over
all 12).

---

## BUG-PPG-004 — bidmc03 ECG lead II near-flatline: Lamina ecg-peaks finds 153 beats vs 610 PPG beats

### Status
Dataset-issue (signal quality), behavioral observation recorded

### Component
`lamina::ecg::ecg_findpeaks_config` observation under extreme low-amplitude input.

### Dataset
BIDMC `bidmc03`, ECG lead II @ 125 Hz.

### Observed
II channel std = 0.053 mV, p2p = 1.26 mV (vs healthy lead II QRS ~1 mV
amplitude): the electrode signal is near-flatline/low-voltage. Lamina
ecg-peaks finds 153 peaks in 8 min (~19/min, implausible) while Lamina
ppg-peaks on the same recording finds 610 (~76/min, plausible). Root cause
is the recorded signal (dataset issue), not the detector; entry kept as a
structural anomaly flag. No fix proposed.

---

## Notes on observations that were investigated and NOT classified as defects

- BIDMC breath annotations (`.breath`, aux_note `ann1`/`ann2`) are 0-based
  sample indices at 125 Hz marking inspiratory peaks — verified empirically
  (median offset vs band-passed RESP maxima 0.0 s; 100% within 0.5 s on
  bidmc01). Inter-annotator agreement F1 = 0.98-0.99 (metrics_extra).
- ECG @125 Hz (BIDMC) with the default 5-15 Hz detection bandpass:
  no anomaly observed beyond BUG-PPG-004; peaks/min plausible (91/min on
  bidmc01/02, matching PPG 90/min).
- Wrist records: all 15 channels declared at 256 Hz; non-ECG channels were
  resampled by the dataset authors (`sample_times_for_all_signals_apart_from_ecg`
  channel). Short NaN gaps (e.g. 234 samples in s1_walk) are wrist-device
  dropouts; the adapter linearly interpolates them and records counts in
  recording metadata. Bridge correctly rejects non-finite JSON input
  (good fail-fast behavior — not a defect).
