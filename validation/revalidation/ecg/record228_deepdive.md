# Record 228 Deep-Dive (BUG-011 revalidation)

Revalidation @ post-remediation `fec2668` (repo `lamina-reval` master + bridge `caa7484`).
Methodology identical to baseline: wfdb → MLII channel → bridge `ecg-peaks`
(defaults, `ecg-clean "none"→""` no-op-equivalent 0.5 Hz high-pass) → greedy
one-to-one matching, 150 ms tolerance. Class-conditional recall is derived by
tabulating the *same* all-beat greedy match against `.atr` beat symbols
(`N` = normal, `V` = PVC, other = remaining beat symbols). Machine-readable:
`228_class_recall.csv`.

## Headline (all-beat, old methodology)

| metric | baseline (f3a195a) | current | delta |
|---|---|---|---|
| F1 | 0.7492 | **0.9913** | +0.2421 |
| recall | 0.6016 | **0.9985** | +0.3970 |
| precision | 0.9928 | 0.9842 | −0.0086 |
| TP / FP / FN | 1235 / 9 / 818 | **2050 / 33 / 3** | +815 / +24 / −815 |
| n_detected | 1244 | 2083 (n_ref 2053) | +839 |
| HR MAE (bpm) | 14.87 | **1.41** | −13.46 |
| timing MAE (ms) | 2.40 | 2.17 | −0.23 |

## Class-conditional recall (new methodology, current run)

| class | n | matched | recall |
|---|---|---|---|
| N (normal) | 1688 | 1685 | **0.9982** |
| V (PVC) | 362 | 362 | **1.0000** |
| other | 3 | 3 | 1.0000 |

Baseline class-conditional numbers were not recorded by the original
validation (baseline was symbol-agnostic); the remediation plan cites baseline
normal-beat recall 0.602 (numerically equal to the all-beat recall, i.e. the
818 FNs were overwhelmingly normal beats following tall PVCs — the SPKI
inflation mechanism). Current normal-beat recall 0.9982 clears the plan's
pass bar (≥ 0.950).

## Cost analysis — did the fix buy recall at the price of FPs or PVCs?

- Detections rose 1244 → 2083 (+839). Of these, **+815 are true positives**
  (recovered beats) and only **+24 are false positives**. PVC recall is 1.000
  (362/362) — no PVC-detection cost.
- Remaining 3 FNs are all `N` beats at t = 615.82 s, 1218.20 s, 1475.38 s.
- The 33 FPs are mostly isolated; one cluster of 6 FPs within 7 s at
  t = 1499.6–1506.8 s (noise-burst-like segment) accounts for most of the
  precision cost.

## Neighbor side-effect scan (same patient-cluster style)

| record | baseline F1 | current F1 | delta | baseline FP/FN | current FP/FN | verdict |
|---|---|---|---|---|---|---|
| 221 | 0.9992 | 0.9994 | +0.0002 | 0 / 4 | 0 / 3 | unchanged |
| 222 | 0.9917 | 0.9395 | −0.0522 | 11 / 30 | 251 / 61 | **REGRESSED (FP inflation)** |
| 223 | 0.9981 | 0.9983 | +0.0002 | 0 / 10 | 0 / 9 | unchanged |
| 226 | — | — | — | — | — | not in MIT-BIH 48-record set (skipped) |
| 231 | 0.9994 | 0.7053 | −0.2941 | 1 / 1 | 819 / 269 | **SEVERE REGRESSION** |
| 232 | 0.9992 | 0.6491 | −0.3500 | 2 / 1 | 1664 / 125 | **SEVERE REGRESSION (det 3319 ≈ 1.86× n_ref)** |
| 233 | 0.9998 | 0.9998 | +0.0000 | 0 / 1 | 0 / 1 | unchanged |

232's detection count nearly doubles the reference (3319 vs 1780) — the same
peaks≈2×ref signature confirmed by Wave C as R-spike + T-wave double
detection. The extra detections on 231/232 are overwhelmingly FPs, not
recovered beats (232: TP actually *fell* 1779 → 1655 while detections rose
+86%).

## Diagnostic probe (diagnostic only, NOT a proposed fix)

Re-running with `threshold_multiplier = 0.5` (default 0.25) through the bridge:
228 F1 0.9966 (TP 2046/FP 7/FN 7) — the 228 fix is *robust* to a higher
threshold, while 231/232/222/100/111/117/123 fully recover (e.g. 232 → F1
0.998, FP 1; 100 → F1 1.000, FP 0; 117 → F1 1.000). Interpretation: the SPKI
clamping (`y_val.min(spki)`, `min(2.5*spki)`) keeps the adaptive threshold
near the noise floor on real ECG, so T-waves and secondary waveform humps
exceed THRESHOLD I and pass the 200 ms refractory. The fleet-wide regression
is an operating-point problem, not a failure of the 228 mechanism fix.

## Verdict

**BUG-011: RESOLVED on the target record** (normal-beat recall 0.6016 →
0.9982, plan bar ≥ 0.950 met; no PVC cost; +24 FP acceptable in isolation).
However the same change causes severe FP/FN regressions on neighbors 231/232/222
and 9 other mitdb records — tracked separately as the fleet-wide
double-detection regression (see `waveB-notes.md`, BUG-NEW-B1 / Wave C
BUG-NEW-C1, P0).
