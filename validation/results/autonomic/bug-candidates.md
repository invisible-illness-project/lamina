# Autonomic/Wearable Group — Bug Candidates

Validation-only findings for the Lamina crate, discovered while validating
against WESAD, Autonomic Aging, Wearable Exam Stress, and BIG IDEAs.
Taxonomy follows `docs/validation/BUGS.md` (confirmed / suspected /
dataset-issue / ambiguous / limitation). **No Lamina source was modified.**

Entries are appended below as datasets complete.

## BUG-A01 — `hrv` op computes RMSSD over all detected intervals; no ectopy/artifact filtering (confounds age trend)

### Status
Limitation (expected/by-design; documented for visibility)

### Component
Lamina `hrv_rmssd` / `hrv_mean_nn` via bridge op `hrv` (upstream of it:
`ecg_findpeaks_config` at fs=1000 Hz).

### Dataset
Autonomic Aging 1.0.0, records 0276, 0554, 0637 (age stratum 70+).

### Reproduction
```
python validation/results/autonomic/extra_analysis.py aging
# or: bridge.ecg_peaks(ecg1, 1000) -> bridge.hrv(peaks, n, 1000)
```

### Input
Record 0554, ECG1, first 10 min @1000 Hz. Visual inspection shows sustained
ventricular bigeminy (every second beat is a wide-complex PVC; see run
artifacts). Lamina `ecg-peaks` correctly marks every QRS (933 peaks in 10 min,
median RR 0.663 s); RR sequence alternates ~0.44 s / ~0.83 s.

### Expected Behavior
Published physiology expects RMSSD to *decrease* with age group. A naive
known-groups check on Lamina RMSSD should ideally reproduce that direction.

### Actual Behavior
RMSSD medians: young(18-39) 52.3 ms, middle(40-69) 24.9 ms, old(70+) 142.8 ms
— old stratum inflated by genuine bigeminy/ectopy (record 0554 RMSSD 315.6 ms).
Monotonic-decrease check FAILS (value 0 in metrics_extra.csv). This is a
population-trend consistency check being confounded by arrhythmia in the
sample, not a detection error: Lamina's peaks are correct (verified visually),
and the HRV op documents no ectopy filtering.

### Evidence
`validation/results/autonomic/metrics_extra.csv` rows
`autonomic-aging,0554,rmssd_ms,315.59`; RR percentiles for 0554
(25% = 0.436 s, 75% = 0.830 s — alternating); peak-overlay plot inspected.

### Severity
Informational

### Suggested Investigation
None required for Lamina correctness; consumers of `hrv` on elderly/clinical
recordings should pre-filter ectopic intervals. A documented "normal-to-normal
interval" option could be a feature request, not a bug fix.

### Scope
Validation finding only. No source modification performed.

## BUG-A02 — WESAD S3 chest-EDA: baseline window shows 147 SCRs/4 min (implausibly high)

### Status
Ambiguous (insufficient evidence to classify as Lamina defect vs dataset
characteristic)

### Component
Bridge op `eda-peaks` (`eda_clean` -> `eda_decompose` -> `eda_findpeaks_events`)
at fs=700 Hz.

### Dataset
WESAD 1.0, subject S3, baseline condition window.

### Reproduction
```
python -m validation run --dataset wesad --recordings S3_baseline \
    --results-dir <dir>
```

### Input
S3 chest EDA @700 Hz, 4-min middle window of the longest baseline segment.

### Expected Behavior
Baseline (resting) condition should show a low SCR rate (typically < ~5/min),
lower than the stress condition.

### Actual Behavior
S3 baseline: 147 SCRs in 4 min (36.8/min) vs S3 stress: 39 in 4 min (9.8/min).
For the other 4 subjects baseline SCR rate is 0-10.5/min and stress is higher
(group medians: baseline 0.5/min, stress 8.5/min — expected direction holds).
S3's baseline chest EDA may contain motion/squeeze artifacts or the label
segment may border a non-baseline period; `eda_findpeaks_events` may also
over-segment slow drifts at fs=700 Hz (outside the ~4 Hz design point this
pipeline is typically exercised at for wrist EDA, though chest EDA @700 Hz is
the dataset's native rate).

### Evidence
`validation/results/autonomic/metrics_extra.csv` rows
`wesad,S3_baseline,condition_scr_rate_per_min,36.75` vs
`wesad,S3_stress,...,9.75`; group medians computed over all 5 subjects.

### Severity
Low

### Suggested Investigation
Inspect S3 chest-EDA raw trace in the baseline window (movement artifacts?);
check `eda_findpeaks_events` behavior on a slow-drifting 700 Hz tonic+phasic
signal vs a downsampled (e.g. 10 Hz) version of the same window. If SCR count
explodes only at 700 Hz native rate, suspect a rate-dependent threshold/rise-
time interaction.

## BUG-A04 — Large Lamina-vs-E4 wrist-BVP heart-rate gap (MAE 26-39 bpm) even on signal-active segments

### Status
Ambiguous (evidence insufficient to classify as Lamina defect vs E4 reference
error vs dataset quality)

### Component
Lamina `ppg_clean` -> `ppg_findpeaks_config` (bridge op `ppg-peaks`) at
fs=64 Hz, vs Empatica E4 proprietary HR.csv reference.

### Dataset
wearable-exam-stress 1.0.0, S1..S5 x {midterm_1, midterm_2, Final}.

### Reproduction
```
python -m validation run --dataset wearable-exam-stress --results-dir <dir>
python validation/results/autonomic/extra_analysis.py exam
```

### Input
E4 BVP @64 Hz, sessions 1.5-7 h. Signal-contact quality varies strongly by
session: per-second-std>1 active fraction 0.17-0.35 for midterm_1 (long
off-wrist/flat stretches) vs 0.78-1.00 for midterm_2/Final.

### Expected Behavior
Two HR estimates from the same wrist device should roughly agree (MAE of a
few bpm) when the BVP signal is valid.

### Actual Behavior
Raw aligned HR MAE vs E4 HR.csv: 26.4-47.1 bpm (median 35.2). Restricting to
seconds with BVP energy (active segments) still gives MAE 26.4-38.7 bpm.
Counter-evidence for a pure Lamina defect: (1) E4 HR.csv itself contains
implausible values (e.g. 150-170 bpm during seated exams; S1 midterm_1 E4
median 97.5 bpm vs Lamina 61 bpm); (2) where E4 emits confident IBI values,
Lamina BVP inter-beat intervals agree with E4 IBI.csv within 28.9-45.4 ms
(S2_Final n=8361, S5_midterm_2 n=4056); (3) WESAD cross-device check (Lamina
chest-ECG HR vs Lamina wrist-BVP HR, both by Lamina) shows MAE 5.8-22.4 bpm.

### Evidence
`metrics.csv` (`hr_mae` per recording) and `metrics_extra.csv`
(`hr_mae_active_segments_bpm`, `ibi_aligned_mae_ms`, `bvp_active_fraction`).

### Severity
Low

### Suggested Investigation
Benchmark `ppg-peaks` on a dataset WITH beat-level ground truth (e.g. the PPG
group's datasets) before attributing the gap to Lamina; quantify E4 HR.csv
error against ECG-derived HR in a dataset that has both.

### Scope
Validation finding only. No source modification performed.

## BUG-A03 — `eda_clean` / `eda_peaks` hardcode a 5 Hz lowpass cutoff: any fs ≤ 10 Hz fails (blocks all Empatica E4 EDA @4 Hz)

### Status
Confirmed (reproduced with a minimal synthetic input against the current
implementation via the bridge; the EDA API takes `fs` as a parameter and gives
no documented fs floor).

### Component
Lamina `eda_clean` (and therefore `eda_peaks`, which cleans internally).
`eda_decompose` does NOT fail at fs=4 Hz.

### Dataset
big-ideas 1.0.0 (EDA_003.csv, EDA_015.csv @4 Hz) and wearable-exam-stress
1.0.0 (all sessions, EDA.csv @4 Hz). Also reproducible on synthetic data.

### Reproduction
```
python - <<'EOF'
import numpy as np
from validation.bridge import LaminaBridge
b = LaminaBridge()
sig = 2 + np.cumsum(np.random.RandomState(1).randn(2400)) * 0.001
b.eda_clean(sig, 4.0)   # raises
EOF
```

### Input
Any EDA signal with sampling_rate <= 10 Hz (fs=4, 8, 10 all fail; fs=16 ok).
Empatica E4 wrist EDA is 4 Hz — the most common wearable EDA rate.

### Expected Behavior
A sampling-rate-parameterized cleaning routine should adapt its filter cutoff
to the Nyquist frequency (or document and enforce a minimum fs gracefully),
not unconditionally fail.

### Actual Behavior
`lamina_error`: "Invalid cutoff frequency: Cutoff frequency (5) must be
strictly less than Nyquist frequency (2)" at fs=4 Hz. At fs=10 Hz the same
strict-inequality check fails (Nyquist == 5). Consequence: every
wearable-exam-stress EDA recording and every big-ideas EDA recording fails
(recorded as per-recording failures in the runner outputs; big-ideas dataset
status = failed for this reason alone — the data itself is accessible).

### Evidence
- `validation/results/autonomic/big-ideas/recordings.csv`: 2/2 failed with the
  cutoff error (see logs/big-ideas.log).
- Minimal-repro sweep: fs ∈ {4, 8, 10} fail; fs=16 succeeds; `eda-decompose`
  at fs=4 succeeds, isolating the failure to `eda_clean`'s filter spec.

### Severity
Medium (blocks wrist-EDA validation on the most common wearable EDA sampling
rate; no silent wrong results — the error is explicit).

### Suggested Investigation
Locate the hardcoded 5 Hz cutoff in `eda_clean`; either clamp to
`min(5, 0.8 * fs/2)` or return a documented error listing the minimum
supported fs. Also cross-check the API-INVENTORY "sharp edges" note about
hardcoded rates in `eda_findpeaks`/`rsp_findpeaks`.

### Scope
Validation finding only. No source modification performed.


