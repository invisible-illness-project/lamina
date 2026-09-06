# Validation Metrics

Definitions, tolerances, and the reference-vs-Lamina distinction. Code:
`validation/validation/metrics/{events,rate,hrv}.py`; contract: SPEC §5.

## Event detection metrics (`events.py`)

**Matching.** `match_events(reference, detected, fs, tolerance_sec)` performs
greedy one-to-one nearest-within-tolerance matching: reference events are
iterated in ascending order; each takes the nearest not-yet-matched
detection within ±tolerance; a detection matches at most one reference.
Timing error is signed, `detected − reference`, in seconds.

**Tolerances** (defaults in `runner.DEFAULT_TOLERANCES`, recorded in every
run manifest):

| comparison | tolerance | rationale |
| ---------- | --------: | --------- |
| ECG/PPG beat peaks (`peak_match_sec`) | **150 ms** | standard beat-detector evaluation window; generous relative to the timing accuracy actually observed (mitdb mean abs error 4.6 ms) |
| respiratory cycles (`rsp_match_sec`) | **0.5 s** | breath onsets/peaks are marked to annotator precision; 0.5 s is well inside the shortest physiological inter-breath interval |
| EDA SCR onsets (`scr_match_sec`) | **1.0 s** | SCR onsets are slow physiological events (rise times of seconds); 1 s is tight relative to SCR dynamics |

**Reported per recording** (`peak_detection_metrics`): `tp`, `fp`, `fn`,
`precision` (TP / detected), `recall` (TP / reference), `f1`,
`mean_abs_timing_error_sec`, `median_timing_error_sec`, `n_reference`,
`n_detected`, `tolerance_sec`. Fields are `null` when a denominator is 0
(never silently 0/0).

## Rate metrics (`rate.py`)

`rate_metrics(reference, estimated)` over aligned windowed series:
`mae`, `rmse`, `bias`, `pearson_r`, `n`. Windows with fewer than 2 detected
peaks yield NaN HR and are dropped from the comparison (the drop is visible
in `hr_windows_covered_fraction`-style extra metrics where relevant). Heart
rate is computed as mean(60/IBI) over inter-beat intervals whose midpoint
falls inside the window; window geometry is adapter-documented (e.g. 6 s
windows / 1 s step for SCAMPS).

## HRV metrics (`hrv.py`)

`hrv_metrics(reference, estimated)`: `mae`, `rmse`, `bias`,
`mean_relative_error`, `pearson_r`, `n` over per-recording HRV scalars.

**Reference-vs-Lamina distinction (important).** Two different things are
reported and never conflated:

- *annotation-derived HRV* — RMSSD / mean-NN computed from the dataset's own
  beat annotations (the reference; `*_annotation` rows in
  `metrics_extra.csv`);
- *Lamina-detected-beat HRV* — the same quantities computed from Lamina's
  detected peaks end-to-end (`*_lamina` rows), which is what a user
  actually obtains.

The `*_comparison_*` rows compare the two. On mitdb this separates cleanly:
mean-NN agrees (MAE 33.7 ms, relative error 4.0 %, r 0.765) while RMSSD does
not (MAE 313 ms, r 0.10), because RMSSD is dominated by ectopic intervals
and Lamina's `hrv` pipeline has no ectopy/artifact filter (BUG-A01). RMSSD
validation claims should therefore be read as applying to normal-sinus data
only.

## Structural metrics

Where no defensible ground truth exists (WESAD SCRs, autonomic-aging beats,
BIDMC/wrist PPG waveforms), the runner emits `structural_*` metrics: peak /
cycle counts, mean rates, plausibility ranges, and internal-consistency
checks (e.g. Lamina chest-ECG HR vs Lamina wrist-BVP HR on WESAD). These
verify that the pipeline executes and produces physiologically plausible
output; they are **not** accuracy claims and are labelled `structural` in
`metrics.csv` and in the report wherever they appear. Known-groups contrasts
(e.g. WESAD stress vs baseline) are reported as medians over recordings with
the designed direction of the effect stated.

## Supplementary metrics (`metrics_extra.csv`)

Group-level extras that don't fit the runner's schema: inter-annotator
agreement (BIDMC breath ann1 vs ann2), known-groups deltas (WESAD),
age-stratum RMSSD medians (autonomic-aging), signal-active-gated HR/IBI
agreement (wearable-exam-stress), raw and sign-corrected waveform
correlations (SCAMPS). Each row carries `units` and a `notes` provenance
string.
