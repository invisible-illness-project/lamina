# Validation Datasets

Selection rationale, Lamina-capability mapping, and per-dataset access notes
for the 18 registered validation targets (`validation/validation/registry.py`).

**Terminology used throughout the validation docs:**

- *"Lamina supports X"* — the API exists (see
  [`validation/API-INVENTORY.md`](../../validation/API-INVENTORY.md)) and is
  exercised through the bridge.
- *"Lamina is empirically validated against X"* — a public dataset with
  ground truth (or a defensible structural/known-groups proxy) was run and
  the numbers are in `validation/results/`.

These are deliberately distinct claims: every capability in the table below
is "supported"; only the datasets marked **validated** / **partially
validated** carry empirical evidence.

## Selection rationale

Datasets were chosen to cover each Lamina capability area with at least one
dataset that has defensible ground truth, plus stratification axes that
stress the implementation:

| capability area | ground-truth anchor | stress/stratification axis |
| --------------- | ------------------- | -------------------------- |
| ECG cleaning + beat detection | mit-bih-arrhythmia (expert beat annotations, 48 records) | mit-bih-noise-stress (calibrated 0–24 dB noise gradient) |
| PPG beat detection / HR | wrist-ppg-exercise (chest-ECG `.atr` reference) | activity stratification: walk / run / bike |
| respiration cycles | bidmc (two-annotator breath annotations) | ICU low-RR morphologies |
| autonomic / EDA | wesad, autonomic-aging (known-groups, population trend — structural) | wearable-exam-stress, big-ideas (consumer-device fs, long sessions) |
| HRV | mitdb (annotation-derived comparison) | autonomic-aging (age strata, ectopy confound) |
| rPPG algorithms | scamps (embedded GT waveform, exact sync) | pure / ubfc-rppg / cohface / ubfc-phys / mmpd / ibvp (real video — all inaccessible this run) |
| scale/format robustness | pulsedb, mimic-iii-waveform, mimic-iii-ext-ppg (large critical-care corpora) | credentialed-access barrier (all three inaccessible) |

## Capability mapping (supported vs validated)

| Lamina capability (bridge op) | supported | empirically validated against |
| ----------------------------- | --------- | ----------------------------- |
| `ecg-clean` / `ecg-peaks` | yes | mit-bih-arrhythmia (F1 0.982), mit-bih-noise-stress (SNR gradient), wrist chest-ECG (F1 0.877) |
| `ppg-clean` / `ppg-peaks` | yes | wrist-ppg-exercise (HR MAE 20.7 bpm, motion-stratified), wearable-exam-stress BVP (vs E4 reference, ambiguous), SCAMPS waveforms |
| `rsp-clean` / `rsp-cycles` | yes | bidmc (cycle F1 0.944 vs breath annotations) |
| `eda-clean` / `eda-decompose` / `eda-peaks` | yes | wesad chest-EDA (structural + known-groups). **Not validated on fs ≤ 10 Hz streams**: confirmed defect BUG-A03 blocks E4 EDA @ 4 Hz (wearable-exam-stress, big-ideas) |
| `hrv` (RMSSD, mean-NN) | yes | mitdb (mean-NN rel. err 4 %; RMSSD confounded by ectopy — BUG-A01), autonomic-aging (population trend, structural) |
| `rppg-algorithm` (green/chrom/pos) | yes | scamps only (synthetic; HR MAE 11.4–16.2 bpm). No real-video dataset was accessible |
| `signal-peaks`, `filter`, `sample-entropy` | yes | framework fixtures only (synthetic `.npz`, `validation/fixtures/`); no public-dataset ground truth exercised |

## Per-dataset notes

### ECG

- **mit-bih-arrhythmia** — *validated*, 48/48 records. ODC-BY 1.0,
  `https://physionet.org/content/mitdb/` (fetched via wfdb without
  credentials). 30 min 2-lead ECG @ 360 Hz, expert beat annotations.
  Adapter: beat references use the MIT-BIH beat symbol set only; channel
  MLII, fallback first channel (V5 for records 102/104).
- **mit-bih-noise-stress** — *validated*, 7/7 acquired records. ODC-BY 1.0.
  physionet.org returned HTTP 403 from the validation host; files were
  fetched from the throttled static mirror, so 7 of 12 annotated records
  were acquired (full 0–24 dB SNR grid for base record 118 + both extremes
  for 119; BUG-ECG-006). Calibration noise records (bw/em/ma) carry no beat
  annotations and are excluded.

### PPG / cardiovascular

- **bidmc** — *validated*, 12/12 recordings. ODC-BY 1.0; mirror WFDB layout
  (physionet.org/content 403 from this IP). 8-min ICU recordings @ 125 Hz
  with two-annotator breath references (inter-annotator F1 0.985). PPG/ECG
  channels are structural (no beat ground truth); bidmc03 ECG is a
  near-flatline dataset anomaly (BUG-PPG-004).
- **wrist-ppg-exercise** — *validated*, 19/19 recordings, 8 subjects.
  ODC-BY 1.0 via wfdb (slow host; parallel download recommended).
  Wrist PPG + chest ECG + 3-axis accelerometry @ 256 Hz; activities
  walk/run/low-/high-resistance bike. Short wrist-device NaN gaps are
  linearly interpolated by the adapter with counts recorded in metadata;
  non-ECG channels were resampled by the dataset authors.
- **pulsedb** — *inaccessible*: official hosts unreachable and mirrors
  blocked from the validation environment (verified 2026-09-06).
- **mimic-iii-waveform** — *inaccessible*: PhysioNet credentialed access
  (CITI training + signed DUA); credential application out of scope for the
  environment (403 verified).
- **mimic-iii-ext-ppg** — *inaccessible*: same credentialed barrier as the
  parent database (403 verified).

### Autonomic / wearable

- **wesad** — *validated*, 15 recordings (S2–S6 × baseline/amusement/
  stress). MIT-licensed dataset. Chest RespiBAN @ 700 Hz (ECG/EDA/RESP) +
  wrist E4 BVP @ 64 Hz. No beat/SCR ground truth → structural metrics plus
  the designed known-groups contrast (stress vs baseline). Adapter extracts
  fixed 4-min condition windows from the continuous protocol using the
  dataset's condition markers.
- **autonomic-aging** — *validated*, 12 recordings spanning age strata.
  ODC-BY 1.0. ECG1/ECG2 @ 1000 Hz, no beat annotations → structural plus
  RMSSD-vs-age population trend; the old stratum contains ventricular
  bigeminy, which confounds the trend because Lamina `hrv` has no ectopy
  filter (BUG-A01).
- **wearable-exam-stress** — *partially validated*, 5 subjects × 3 exam
  sessions: 15 BVP recordings ok, 15 EDA recordings failed on BUG-A03
  (E4 EDA @ 4 Hz vs Lamina's hardcoded 5 Hz lowpass). ODC-BY 1.0. The HR
  reference is the E4's proprietary onboard estimate — imperfect; treated
  as a consistency check, not ground truth (BUG-A04).
- **big-ideas** — *failed*: data accessible (public S3 mirror of per-subject
  CSVs), both evaluated EDA recordings fail on BUG-A03. BVP/ACC files
  (~919 MB+) exceed the download budget; glucose is out of Lamina scope.

### rPPG

- **scamps** — *validated (synthetic)*, 30 recordings = 10 videos × 3
  algorithms. MIT license. The public example set
  (`scamps_videos_example.tar.gz`) ships raw RGB face crops with embedded,
  exactly synchronized ground-truth PPG. Validation-side orchestration:
  dataset skin-mask ROI, uniform 30 fps grid, 3 s/0.5 s windows, reference
  peaks from an independent scipy detector. Validates algorithm correctness
  on synthetic data; not a substitute for real-video validation.
- **pure** — *inaccessible*: download requires an e-mail application to TU
  Ilmenau.
- **ubfc-rppg** — *inaccessible*: official Google Sites page unreachable;
  Kaggle mirror requires authentication.
- **cohface** — *inaccessible*: Zenodo record files are restricted (EULA,
  academic signatory) and the host is IP-blocked from the environment.
- **ubfc-phys** — *inaccessible*: downloadable in principle, but the
  smallest subject archive is 91.7 GB, far beyond the environment's
  download timebox.
- **mmpd** — *inaccessible*: requires a signed release agreement from a
  faculty e-mail address.
- **ibvp** — *inaccessible*: requires a EULA signed by an academic
  supervisor.

All access reasons were probed and recorded on 2026-09-06 in each stub
adapter's `check_access().detail`; re-running `python -m validation
check-access` re-verifies them.
