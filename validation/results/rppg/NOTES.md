# rPPG-group validation notes (scamps)

Run: `python -m validation run --dataset scamps --results-dir validation/results/rppg --seed 42`
(lamina 0.1.0, bridge 0.1.0; see manifest.json for commit/versions).

## Dataset scope and honesty labels

- **scamps — VALIDATED (synthetic-data validation).** Evaluated the public
  10-video example set (`scamps_videos_example.tar.gz`, MATLAB v7.3 `.mat`
  files with raw RGB face-crop frames `Xsub` 240x240 @ 30 fps, 600 frames =
  20 s each). Ground truth `d_ppg` is embedded per-frame in the same file, so
  video/reference synchronization is exact by construction (no sync
  uncertainty). This validates Lamina's rPPG algorithms on *synthetic* data;
  it is not a substitute for real-video validation.
- All real-video rPPG datasets (pure, ubfc-rppg, cohface, ubfc-phys, mmpd,
  ibvp) were **inaccessible** from this environment; each stub adapter records
  the exact verified reason in `check_access().detail` (probed 2026-09-06).
  Notably ubfc-phys is downloadable in principle, but the smallest subject
  archive is 91.7 GB — far beyond the environment's download timebox.

## Validation-side orchestration (assumptions, all outside Lamina)

- **ROI**: dataset-provided per-frame `skin_mask` (threshold > 0.5) on the
  `Xsub` face crop; per-frame RGB means over masked pixels;
  `valid_pixel_counts` = mask area (~17k px). This deliberately bypasses face
  detection to isolate the rPPG algorithms. Using the shipped skin mask is a
  validation-side heuristic, not a Lamina feature.
- **Timestamps**: uniform 30 fps grid (matches `t_ppg` in the SCAMPS waveform
  CSVs and the dataset documentation); no video-container decode was needed
  (frames are raw arrays in the `.mat`), so no fps/timestamp ambiguity exists.
- **Lamina calls**: bridge op `rppg-algorithm` (window 3 s, step 0.5 s,
  Lamina defaults otherwise) -> waveform; bridge op `ppg-peaks` on the
  waveform at `mean_sampling_rate_hz` (~30 Hz) -> estimated peaks.
- **Reference peaks**: validation-side scipy detector on `d_ppg`
  (order-3 Butterworth 0.75–2.5 Hz bandpass + `find_peaks`, min distance
  0.4 s), independent of the implementation under test.
- **HR series**: 6 s windows / 1 s step; HR = mean(60/IBI) over IBIs whose
  midpoint is in-window; windows with < 2 peaks are NaN and dropped by the
  rate metrics (14 usable windows per 20 s recording).
- **Peak matching** (`ppg_peak_indices` reference) IS used here because sync
  is verifiably exact: the adapter asserts the rPPG waveform timeline equals
  the frame grid within 1 ms before exposing ground-truth peak indices
  (held for all 30 recordings).

## Recordings

30 recordings = 10 videos x 3 algorithms (`green`, `chrom`, `pos`), each
(video, algorithm) pair reported separately so per-algorithm rollups come from
metrics.csv directly.

## Headline results (mean over 10 videos; GT HR range 84.5–131.5 bpm)

| algorithm | HR MAE (bpm) | HR RMSE | HR bias | HR pearson | peak F1 | waveform r (raw) | waveform |r| |
|-----------|--------------|---------|---------|------------|---------|------------------|---------------|
| chrom     | 11.82        | 13.44   | -2.69   | 0.312      | 0.768   | +0.362           | 0.485         |
| green     | 11.39        | 13.14   | +7.57   | 0.109      | 0.245   | -0.921           | 0.921         |
| pos       | 16.19        | 18.67   | +0.96   | 0.105      | 0.540   | -0.823           | 0.823         |

Reading: the raw green/POS traces are *excellent* signals in magnitude
(|r| 0.92 / 0.82 vs ground truth) but are produced in inverted
(intensity-phase) sign relative to CHROM's output and to what Lamina's
Elgendi `ppg-peaks` expects; see bug-candidates.md BUG-R01. CHROM is the only
algorithm whose output phase is directly peak-detectable, and it tracks HR
within ~2 bpm MAE on the cleanest videos (P000001, P000007, P000009).

## Stratification

Motion/AU stratification is **not meaningful** for this example set: all 10
example videos have zero head rotation (mean |pitch|=|roll|=|yaw|=0) and only
low-amplitude AU45 (blink) activity; per-video values are still recorded in
metrics_extra.csv. GT HR spans 84.5–131.5 bpm; errors grow on the mid-range
HR videos (P000005/P000006, all algorithms), consistent with those signals
sitting lower in the 0.75–2.5 Hz signal band with weaker pulse SNR.

## metrics_extra.csv columns

`value` = raw Pearson r between the Lamina rPPG waveform and bandpass-aligned
ground-truth `d_ppg`; `value_sign_corrected` = |r| (standard rPPG practice —
intensity-based algorithms have physically ambiguous sign); plus GT/estimated
mean HR, peak counts, timeline-alignment flag, and motion stats.
