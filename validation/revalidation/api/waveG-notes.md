# Wave G — Numerical / API Robustness Validation (task §14)

**Scope:** independent revalidation of Lamina `main @ fec2668` (repo bridge commit
`caa7484`), validation-only. All 17 bridge ops exercised through the compiled
`lamina_bridge` binary; Lamina source untouched.

**Artifacts**
- `api_results.csv` — 462 atomic (op, case) rows.
- `probe_api_robustness.py` — the probe (deterministic; fixtures + seeded synthetics).
- `evidence/raw/<op>__<case>.out.json` — verbatim bridge envelopes for all 338
  non-ok outcomes; `evidence/inputs/` — the exact input JSON for each failure.

## Methodology

- Per op (where applicable): sampling_rate ∈ {0, −100, NaN, +inf, 1e9, missing,
  null}; signal ∈ {empty, 1-sample, 2-sample, 1000 zeros, 1000 ones, missing,
  null element, NaN/±inf at first/middle/last, ×1e12, ×1e-12}; config
  thresholds {negative, 0, NaN, above/== Nyquist, inverted min/max};
  unsupported method/algorithm/polarity/policy/kind strings; missing required
  fields. Baselines: committed fixtures (`ecg/ppg/rsp/eda_synthetic.npz`) +
  seeded synthetics.
- **Non-finite floats cannot be expressed in JSON.** The probe injects raw
  `NaN` / `Infinity` / `-Infinity` tokens and explicit `null` array elements
  directly into the input JSON text (bypassing `json.dumps`) to test both the
  protocol boundary and (where reachable) Lamina's `NonFiniteInput` path.
- Per-case timeout 30 s; outcome classes `ok | lamina_error | bad_request |
  panic | timeout | crash` (crash = signal kill / missing or unparseable
  output envelope). `error_correct=yes` means the error identifies the actual
  invalid input; `no` means misleading diagnosis or silent acceptance.
- Exit-code contract (SPEC §3.1) held exactly across all 462 runs: ok→0,
  lamina_error→1, bad_request→3 (panic→2 was never exercised — no panics).

## Headline results

| metric | count |
|---|---|
| total rows | 462 |
| ok | 124 |
| lamina_error | 164 |
| bad_request | 174 (116 = raw NaN/Infinity/null tokens rejected by serde_json at the protocol boundary) |
| **panic** | **0** |
| **timeout** | **0** |
| **crash (process abort)** | **0** |
| error_correct = no | **45** (21 × P2, 24 × P3; no P0/P1) |

**No panics, no aborts, no hangs across the full 462-case matrix.** The
`catch_unwind` containment was never even needed. All baselines pass.

## Finding F1 (P2, systemic): the BUG-002 wrong-variant pattern persists in 6 other validators

BUG-002 itself is fixed — `ecg-peaks threshold_multiplier ∈ {−0.5, 0, 1.0, 1.5}`
now returns `Invalid cutoff frequency: Threshold multiplier must be strictly
between 0.0 and 1.0` (correct). But the *same* mistake — mapping
finite-but-out-of-range **config** values onto `SignalError::NonFiniteInput`
("Input signal contains non-finite values (NaN or Infinity)" on provably
finite signals) — survives everywhere else:

| op / case | source |
|---|---|
| `eda-peaks` min_amplitude<0, min_prominence<0 | `src/eda/peaks.rs:71,74` |
| `ppg-peaks` alpha<0 | `src/ppg/peaks.rs:117` |
| `rsp-cycles` min_amplitude<0 | `src/rsp/peaks.rs:118` |
| `rppg-algorithm`/`rppg-polarity` min_window_fraction >1 or <0 | `src/rppg/config.rs:52` |
| `signal-peaks` min_prominence<0 | `src/signal/peaks.rs:73` |
| `hrv-correct` percent_threshold ∉ (0,1) | `src/hrv/quality.rs:138` — **in the new post-remediation HRV code** |

Root cause (per stage-1 baseline): `SignalError` has no `InvalidParameter`
variant, so validators keep reusing `NonFiniteInput`. (Same file also maps
`min_quality`/`max_gap_sec` onto `NonFiniteInput` at `src/rppg/config.rs:134,140`
— not reachable through the bridge config DTO.)

## Finding F2 (P2/P3): `InvalidWindowSize(0)` used for non-window parameters, echoing a value never supplied

Every time/interval/amplitude-relation validation emits
`Invalid window size: 0 (must be > 0)` — the literal `0` is hardcoded even when
the caller supplied −1.0 or 5.0, and "window size" mislabels the parameter:

- **P2** (concept wholly wrong): `rsp-cycles` min/max breath-interval bounds,
  including `min_breath_interval_sec=10 > max=1` (inverted) — all four surface
  as "Invalid window size: 0" (`src/rsp/peaks.rs:108-115`); `eda-peaks`
  min_distance_sec<0, min_rise_time_sec<0, min_rise≥max_rise (`src/eda/peaks.rs:77-87`).
- **P3** (right area, wrong echoed value/concept): `ecg-peaks`
  integration_window_sec/refractory_period_sec ≤ 0 (`src/ecg/peaks.rs:111,114` —
  a refractory period is not a window); `ppg-peaks` w_peak_sec/w_beat_sec < 0;
  `rppg-*` window_sec<0, step_sec≤0, step>window; `eda-peaks` max_rise_time_sec=0.

## Finding F3 (P2): `sample-entropy` accepts m=0

Contract (`src/complexity/entropy.rs` rustdoc) requires m ≥ 1, but m=0 is not
validated: the op returns a plausible-looking finite number (2.1487 on the
probe signal) that is mathematically undefined. No error, no flag.

## Finding F4 (P3): silent acceptance of other out-of-range config

- `ppg-peaks` alpha = 0 and alpha = 1.5 accepted silently (only alpha < 0 rejected, via the wrong variant — F1).
- `hrv-correct` classify_threshold ≤ 0 or > 1 accepted; threshold 0/−0.1 classify everything ectopic (verified: 13/20 and 20/20 `ectopic_rr` on a clean RR series). `classify_intervals` is infallible by design but has no bounds.

## Finding F5 (P3): fs = 1e9 — no crash, but silent numeric destruction in 3 ops

All finite-positive; none panic. Behavior splits:
- `ecg-peaks`/`ppg-peaks`: graceful `InsufficientSamples` (window scales with fs: "requires at least 150000000 samples") — correct.
- `ecg-clean`, `rsp-clean`, `eda-decompose`, `eda-peaks`, `rsp-cycles`: benign.
- **`ppg-clean`, `eda-clean`, `filter` lowpass: output silently ≈ 0**
  (verified: `filter` lowpass cutoff=10 Hz @ fs=1e9 on a 5 Hz unit sine →
  max|out−in| = 0.998, i.e. signal annihilated; biquad coefficients degenerate
  at normalized cutoff ~1e-8). No error, no warning. There is no upper fs
  sanity check anywhere.

## Finding F6 (P3): Lamina's documented `NonFiniteInput` contract is unreachable through the bridge

Contract §2.1 says DSP pipelines reject non-finite samples with
`SignalError::NonFiniteInput`. But NaN/±inf cannot be expressed in JSON: raw
tokens fail serde parsing → `bad_request: invalid input JSON: expected value
at line 1 column N` (does not name the offending field); explicit `null`
elements fail `Vec<f64>` deserialization similarly. So the Lamina-level
validation (verified present in source, e.g. `eda_clean_config`) can never be
exercised via the bridge; Python callers passing arrays containing NaN get an
opaque JSON syntax error, not a signal error. Protocol-level rejection is
*safe* (nothing non-finite reaches the numerics) but the diagnosis is poor.

## Verified-correct remediations / contract conformance

- **BUG-002**: fixed for `threshold_multiplier` (all 4 out-of-range values, correct range message). ✔
- **BUG-003 / contract §2.5**: `eda-clean` at fs=4/8/10 Hz returns the input
  unchanged to 4.4e-16 (true pass-through); `eda-clean-config` reports
  `filter_applied=false, cutoff_hz=5.0, nyquist_hz=2.0`; with
  `pass_through_if_nyquist_violated=false` it correctly raises
  `InvalidCutoffFrequency` ("Cutoff frequency (5) must be strictly less than
  Nyquist frequency (2)"); explicit `"lowpass_cutoff_hz": null` disables
  filtering; fs=11 applies the filter for real. ✔
- **Method/policy dispatch**: `ecg-clean method="bogus"/"hamilton"` →
  `Invalid cutoff frequency: Unsupported ECG cleaning method: bogus` (message
  names the true culprit; variant reuse is cosmetically odd). rppg
  algorithm/polarity, filter kind, hrv-correct policy bogus strings → clear
  `bad_request`. ✔
- Empty signal → `EmptySignal`; short signals → `InsufficientSamples` with
  real numbers; fs≤0 → `InvalidSamplingRate` with the value echoed; missing
  fields → `bad_request` naming the field; `hrv`/`hrv-correct` empty and
  single-interval inputs → empty/null results, not errors (per SPEC). ✔

## Contract discrepancies noted (documentation level, P3)

- **cutoff == Nyquist**: signal-contracts.md §2.1 states `Fs ≥ 2·fcutoff` is
  acceptable, but `error.rs` and every validator enforce *strictly less than*
  Nyquist (`filter`, `eda-decompose`, `rsp-clean`, `ppg-peaks`, `ecg-peaks`
  all reject `highcut == Nyquist`). Implementation is self-consistent and
  scientifically defensible; the architecture document is looser than the code.
  (`eda-clean` treats `>= Nyquist` via the pass-through path.)
- Case sensitivity is inconsistent: `ecg-clean` lowercases the method
  (`"NEUROKIT"` accepted), but rppg `algorithm`/`polarity` and filter `kind`
  are case-sensitive (`"AUTO"`, `"LOWPASS"` rejected at the bridge).
- `hrv` peak envelope goes through a bool mask: duplicate peak indices are
  silently deduplicated and unsorted input silently reordered — harmless, but
  `UnsortedEvents` is unreachable for this op.
- `hrv-correct` with neither `rr_intervals_ms` nor peaks blames
  `missing required field 'sampling_rate'` (ordering artifact); `reject_invalid`
  on an all-artifact series → `Signal array is empty` (blames the signal, not
  the classification).
- `sample-entropy` tolerance errors reuse `InvalidCutoffFrequency`
  ("Tolerance threshold r (0) must be > 0.0 and finite") — message correct,
  variant odd; the bridge's `0.2*std` default collapses to r=0 on constant
  signals (pre-existing BUG-006 side note, still present).

## Reproduction

```bash
cd <repo>   # with validation/lamina_bridge/target/release/lamina_bridge built
python3 validation/revalidation/api/probe_api_robustness.py
```

Every `error_correct=no` row in `api_results.csv` has its exact input JSON in
`evidence/inputs/<op>__<case>.json` and the verbatim bridge response in
`evidence/raw/<op>__<case>.out.json`; e.g.

```bash
validation/lamina_bridge/target/release/lamina_bridge --op hrv-correct \
  --input validation/revalidation/api/evidence/inputs/hrv-correct__pt_one.json \
  --output /tmp/out.json   # exit 1, NonFiniteInput on finite input
```
