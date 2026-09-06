# PPG-group validation — run notes

Branch: `validation/adapters-ppg`. Adapter: `validation/validation/datasets/ppg.py`.
Environment: `$HOME/work-ppg` clone of /mnt/agents/lamina @3856c2a; Rust via rsproxy;
bridge built by `validation/scripts/setup-env.sh`.

## Exact commands

```bash
git clone /mnt/agents/lamina $HOME/work-ppg && cd $HOME/work-ppg
git checkout -b validation/adapters-ppg
sh validation/scripts/setup-env.sh $HOME/work-ppg

# BIDMC data (mirror; physionet.org/content 403 from this IP)
mkdir -p $HOME/data/bidmc && cd $HOME/data/bidmc
for i in $(seq -w 1 12); do for ext in dat hea breath; do
  curl -sO "https://archive.physionet.org/physiobank/database/bidmc/bidmc$i.$ext"; done; done

# wrist data (wfdb client; slow ~14 KB/s — parallel workers recommended)
python3 -c "import wfdb; [wfdb.dl_database('wrist', dl_dir='$HOME/data/wrist', records=[r]) for r in wfdb.get_record_list('wrist')]"

# smoke
cd $HOME/work-ppg
LAMINA_BIDMC_DIR=$HOME/data/bidmc python -m validation run --dataset bidmc --smoke --seed 42 --results-dir validation/results/ppg
LAMINA_WRIST_DIR=$HOME/data/wrist python -m validation run --dataset wrist-ppg-exercise --recordings s1_walk s3_run --seed 42 --results-dir validation/results/ppg

# full (per-dataset CLI commands as specified)
LAMINA_BIDMC_DIR=$HOME/data/bidmc python -m validation run --dataset bidmc --results-dir validation/results/ppg --seed 42
LAMINA_WRIST_DIR=$HOME/data/wrist python -m validation run --dataset wrist-ppg-exercise --results-dir validation/results/ppg --seed 42
```

Final artifacts here were produced by an equivalent in-process driver
(`run_dataset` for both adapters + `write_all` + manifest, seed 42) so that
`metrics.csv`/`summary.csv`/`datasets.json` contain BOTH datasets — the CLI
`write_all` overwrites shared CSVs per invocation, so two sequential CLI runs
would leave only the last dataset's rows. This is a framework writer
behavior (runner not in PPG scope), worked around without modifying it.

`metrics_extra.csv` rows are written by the adapters during iteration
(default path `validation/results/ppg/metrics_extra.csv`, env override
`LAMINA_PPG_METRICS_EXTRA`), per-dataset idempotent replace. Per-activity
aggregate rows (`ACTIVITY:<name>`) were appended post-run from the
per-recording rows.

## Data notes / deviations from the task brief

- BIDMC: the brief's `bidmc_XX_Signals.csv`/`_Breaths.csv`/`_Numerics.csv`
  triplet does NOT exist on the verified mirror — it serves the WFDB layout
  (`bidmcXX.dat/.hea/.breath`). Adapter reads WFDB locally. Breath
  annotations: `wfdb.rdann(..., 'breath')`, `aux_note` = annotator
  (ann1/ann2), `sample` = 0-based index @125 Hz marking inspiratory peaks
  (verified: median offset vs band-passed RESP maxima = 0.0 s, 100% within
  0.5 s on bidmc01). ann1 -> `rsp_peak_indices`; ann2 agreement in
  metrics_extra. Subset: bidmc01..bidmc12 (12 records, as specified).
- wrist: 19 records enumerated via `wfdb.get_record_list('wrist')`; ALL 19
  used (download ~85 MB total). Channel names are `chest_ecg`, `wrist_ppg`,
  `wrist_low_noise_accelerometer_{x,y,z}` (+gyro/mag not exercised).
  Deviation from brief: the dataset ships chest-ECG beat annotations
  (`.atr`), so the HR reference is annotation-derived (independent of
  Lamina) rather than Lamina-ecg-peaks-derived; the originally planned
  Lamina-ECG dependency became a consistency cross-check in metrics_extra.
- wrist PPG channels contain short NaN gaps (device dropouts, e.g. 234
  samples in s1_walk); the bridge rejects non-finite input (good fail-fast).
  Adapter linearly interpolates gaps and records counts in recording
  metadata.
- pulsedb: kept INACCESSIBLE stub (site unreachable, Box/Drive blocked,
  Kaggle auth-gated); real metadata (~5.2M 10-s segments, .mat, ODbL 1.0).
- mimic-iii-waveform / mimic-iii-ext-ppg: kept INACCESSIBLE stubs
  (PhysioNet credentialed, 403 verified).

## Headline results (seed 42, lamina 0.1.0)

- bidmc: 12/12 ok. RESP cycle detection vs breath annotations: mean F1
  0.944 (range 0.658-0.988), mean |timing error| ~0.05-0.12 s. bidmc05
  outlier (F1 0.658) = ~6 brpm biphasic morphology (BUG-PPG-003).
  PPG/ECG structural rates plausible; PPG-vs-ECG HR consistency MAE
  0.24-0.52 bpm on bidmc01/02 (metrics_extra). bidmc03 ECG near-flatline
  (BUG-PPG-004).
- wrist-ppg-exercise: 19/19 ok. ECG peak F1 vs annotations: mean 0.877
  (5 records <0.75 — motion/morphology; BUG-PPG-001/002). HR (8 s win /
  2 s step, Lamina ppg-peaks vs annotation reference): overall MAE 20.7 bpm,
  bias -3.1 bpm; per-activity MAE: walk 15.9, run 32.9, low bike 14.7,
  high bike 20.3 bpm (metrics_extra ACTIVITY rows).

## Framework test suite

`python -m validation test`: 56 passed, 2 failed —
`test_registry.py::test_stub_check_access_is_honest_placeholder` and
`test_stub_iter_recordings_raises_not_implemented`. Both are Stage-1
assumptions ("all 18 adapters are stubs") that any implemented Stage-2
adapter necessarily breaks. `validation/tests/` is outside PPG edit scope;
framework owner should scope those tests to stub-only adapters.
