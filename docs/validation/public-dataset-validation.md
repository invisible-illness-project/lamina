# Lamina Public-Dataset Validation Report

_Generated 2026-09-06T14:51:17+00:00 by
`python -m validation report`._

## Executive Summary

This report records the empirical validation status of the Lamina crate
against public physiological datasets. What was evaluated and what was not is
summarized by the dataset inventory below. Lamina commit `b46baea73b75d75a23e129d647a63eb05d6db858`, framework 0.1.0, seed 0.

Dataset status counts:

- inaccessible: 2
- not_attempted: 16

## Dataset Inventory

| Dataset | Status | Signals | Subjects | Recordings | Validation Areas |
| ------- | ------ | ------- | -------: | ---------: | ---------------- |
| mit-bih-arrhythmia | not_attempted | ecg | 0 | 0 | ecg-clean, ecg-peaks, hrv |
| mit-bih-noise-stress | not_attempted | ecg | 0 | 0 | ecg-clean, ecg-peaks |
| bidmc | not_attempted | ppg, rsp, ecg | 0 | 0 | ppg-clean, ppg-peaks, rsp-clean, rsp-cycles, hrv |
| wrist-ppg-exercise | not_attempted | ppg, ecg, acc | 0 | 0 | ppg-clean, ppg-peaks, hrv |
| pulsedb | not_attempted | ppg, ecg, abp | 0 | 0 | ppg-clean, ppg-peaks |
| mimic-iii-waveform | inaccessible | ppg, ecg, abp, rsp | 0 | 0 | ppg-clean, ppg-peaks, hrv |
| mimic-iii-ext-ppg | inaccessible | ppg, abp | 0 | 0 | ppg-clean, ppg-peaks |
| wesad | not_attempted | ecg, eda, ppg, rsp, temp, acc | 0 | 0 | ecg-clean, ecg-peaks, ppg-clean, ppg-peaks, eda-clean, eda-decompose, eda-peaks, rsp-clean, rsp-cycles, hrv |
| autonomic-aging | not_attempted | ecg | 0 | 0 | ecg-clean, ecg-peaks, hrv |
| wearable-exam-stress | not_attempted | eda, temp, acc, bvp, ecg | 0 | 0 | eda-clean, eda-decompose, eda-peaks, ecg-clean, ecg-peaks, hrv |
| big-ideas | not_attempted | acc, bvp, eda, temp, ecg | 0 | 0 | ppg-clean, ppg-peaks, eda-clean, eda-peaks, hrv |
| pure | not_attempted | rgb_video, bvp | 0 | 0 | rppg-algorithm, ppg-clean, ppg-peaks |
| ubfc-rppg | not_attempted | rgb_video, bvp | 0 | 0 | rppg-algorithm, ppg-clean, ppg-peaks |
| cohface | not_attempted | rgb_video, bvp | 0 | 0 | rppg-algorithm, ppg-clean, ppg-peaks |
| ubfc-phys | not_attempted | rgb_video, bvp, eda | 0 | 0 | rppg-algorithm, ppg-clean, ppg-peaks, eda-clean, eda-decompose, eda-peaks |
| mmpd | not_attempted | rgb_video, bvp | 0 | 0 | rppg-algorithm, ppg-clean, ppg-peaks |
| ibvp | not_attempted | rgb_video, bvp | 0 | 0 | rppg-algorithm, ppg-clean, ppg-peaks |
| scamps | not_attempted | rgb_video, bvp | 0 | 0 | rppg-algorithm, ppg-clean, ppg-peaks |

## Methodology

- Implementation under test: the Lamina Rust crate, exercised exclusively
  through the `lamina_bridge` JSON bridge (see `validation/SPEC.md` §3).
- Reference signals: dataset-provided annotations (beat annotations, contact
  PPG, respiratory labels) as documented per dataset adapter.
- Event matching: greedy one-to-one nearest-within-tolerance matching
  (`validation/metrics/events.py`); default peak tolerance 150 ms.
- Metrics: TP/FP/FN/precision/recall/F1 + timing error for events; MAE/RMSE/
  bias/correlation for rates; absolute/relative error for HRV (see
  `validation/metrics/`).
- Exclusions and preprocessing assumptions: documented per dataset in the
  adapter metadata and in Limitations below.

## Results

_To be populated by validation runs (Stage 3). Sections: ECG, PPG,
respiration, autonomic/EDA, signal quality, rPPG._

## Robustness

_To be populated: stratification by noise, motion, signal quality, recording
condition._

## Failures

_To be populated honestly from `results/summary.csv` and `results/logs/`._

## Known/Potential Bugs

See [`docs/validation/BUGS.md`](./BUGS.md). Bugs are documented, never fixed,
in this task.

## Limitations

- Datasets recorded as `inaccessible` / `not_attempted` above have no
  empirical validation coverage.
- Where no defensible ground truth exists, only structural/execution
  validation is reported and labelled as such.

## Reproducibility

```bash
python -m validation list
python -m validation check-access
python -m validation run --dataset all --seed 0
python -m validation report
```

See `validation/results/manifest.json` for exact code/data/parameter versions.
