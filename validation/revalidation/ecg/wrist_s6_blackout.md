# Wrist s6_low_resistance_bike Blackout Revalidation (BUG-012)

Post-remediation `fec2668`, bridge `caa7484`. Adapter-identical path:
wfdb `chest_ecg` @ 256 Hz, NaN interpolation (`_interp_nans`, 0 NaNs found),
bridge `ecg-peaks` defaults, greedy 150 ms matching vs `.atr` annotations
(536 beats, 280.0 s). Per-bin counts use the baseline's exact binning
(recovered: `np.histogram` with `linspace(0, 280, 10)` → 9 bins ≈ 31.1 s).
Data: `wrist_s6_bins.csv`, plot `wrist_s6_blackout.png`.

## Per-bin detection counts (baseline-identical bins)

| bin (s) | annotations | baseline det | current det |
|---|---|---|---|
| 0–31 | 52 | 66 | 77 |
| 31–62 | 57 | **2** | 67 |
| 62–93 | 58 | **0** | 91 |
| 93–124 | 59 | **3** | 92 |
| 124–156 | 61 | 54 | 100 |
| 156–187 | 62 | 71 | 108 |
| 187–218 | 61 | 67 | 110 |
| 218–249 | 62 | 105 | 115 |
| 249–280 | 64 | 97 | 111 |

## Metrics

| metric | baseline | current (default) |
|---|---|---|
| F1 | 0.657 | 0.736 |
| recall | 0.614 | **0.966** |
| precision | 0.708 | 0.595 |
| TP / FP / FN | 329 / 136 / 207 | 518 / 353 / 18 |
| longest detection gap | ~90 s (bins 1–3) | **0.68 s** |
| leading/trailing silence | — | 0.04 s / 0.09 s |
| timing MAE of matched beats | n/a | 34.8 ms (elevated) |

## Verdict

**Blackout: ELIMINATED.** The ~90 s detection void (bins 1–3: 2/0/3
detections vs ~57 annotations each) is gone; longest gap anywhere in the
record is 0.68 s. BUG-012's primary symptom is resolved.

**But the plan's pass bar is not met:** REMEDIATION_PLAN §4.3 requires
"zero 30 s blackouts; F1 ≥ 0.900" — current F1 is 0.736 because of 353 FPs
(precision 0.595). Bins 2–8 show 1.5–1.85× over-detection vs annotations,
the same T-wave/secondary-hump double-detection signature as the mitdb
fleet regression (BUG-NEW-B1 / Wave C BUG-NEW-C1). The elevated timing MAE
(34.8 ms) is consistent with a fraction of matches locking onto waveform
features offset from the annotated R peak.

## Diagnostic (characterization only, NOT a fix)

`threshold_multiplier` sensitivity through the bridge (same signal/matching):

| multiplier | n_det | TP | FP | FN | F1 | longest gap | bins |
|---|---|---|---|---|---|---|---|
| 0.25 (= default; identical output) | 871 | 518 | 353 | 18 | 0.736 | 0.68 s | 77,67,91,92,100,108,110,115,111 |
| 0.35 | 792 | 520 | 272 | 16 | 0.783 | 0.79 s | 65,62,63,88,84,104,103,114,109 |
| 0.50 | 600 | 522 | 78 | 14 | **0.919** | 1.29 s | 56,62,60,69,73,77,63,72,68 |

Raising the threshold recovers near-annotation counts (bins ≈ 56–77) without
re-introducing any blackout (longest gap 1.29 s) — evidence that the default
operating point sits too close to the noise floor after the SPKI clamp.
Default `threshold_multiplier` confirmed = 0.25 (the 0.25 run reproduces the
default output bit-for-bit).

No blackout localization needed (none present). No code fix proposed.
