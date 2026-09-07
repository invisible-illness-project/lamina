#!/usr/bin/env python3
"""Wave F revalidation: HRV correction-policy semantics matrix (task §1, §3).

Drives the lamina_bridge `hrv-correct` op on crafted RR interval series and
verifies, behaviorally, that every exposed CorrectionPolicy does what its name
implies. Also checks IntervalQuality classification incl. the hardcoded
300/2000 ms artifact bounds at the 299/300/2000/2001 boundaries.

Validation-only: writes CSVs into validation/revalidation/hrv/.
"""
from __future__ import annotations

import csv
import json
import sys
from pathlib import Path

import numpy as np

REPO = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(REPO))
from validation.bridge import LaminaBridge  # noqa: E402

OUT = Path(__file__).resolve().parent
RNG = np.random.RandomState(20260907)

POLICIES = ["none", "reject_invalid", "interpolate_linear",
            "interpolate_cubic", "percent_threshold"]
PCT = 0.2


def clean_sinus(n: int = 60, base: float = 800.0, jitter: float = 10.0) -> np.ndarray:
    return base + RNG.uniform(-jitter, jitter, size=n)


def series_clean() -> np.ndarray:
    return clean_sinus()


def series_ectopic() -> np.ndarray:
    rr = clean_sinus()
    rr[30] = 500.0    # premature (short coupling)
    rr[31] = 1100.0   # compensatory pause (sum preserved ~1600)
    return rr


def series_bigeminy() -> np.ndarray:
    rr = np.empty(60)
    rr[0::2] = 440.0 + RNG.uniform(-3, 3, size=30)
    rr[1::2] = 830.0 + RNG.uniform(-3, 3, size=30)
    return rr


def series_artifact_spike() -> np.ndarray:
    rr = clean_sinus()
    rr[30] = 150.0
    return rr


def series_long_gap() -> np.ndarray:
    rr = clean_sinus()
    rr[30] = 3500.0
    return rr


def series_short_burst() -> np.ndarray:
    rr = clean_sinus()
    rr[30:33] = 250.0
    return rr


def series_sinus_arrhythmia(n: int = 60) -> np.ndarray:
    i = np.arange(n)
    return 800.0 + 80.0 * np.sin(2 * np.pi * i / 12.0)


# Boundary probes for the hardcoded 300-2000 ms artifact rule. Values are
# repeated in homogeneous neighborhoods so the local-median ectopy rule does
# NOT fire (median ~= the value itself) and the artifact bound is isolated.
SERIES_BOUNDARY = {
    "b299_artifact": [299.0, 299.0, 299.0, 299.0, 299.0],
    "b300_lower_in": [300.0, 300.0, 305.0, 300.0, 295.0 * 0 + 300.0],
    "b2000_upper_in": [2000.0, 2000.0, 1990.0, 2000.0, 2000.0],
    "b2001_artifact": [2001.0, 2001.0, 2001.0, 2001.0, 2001.0],
}

SERIES = {
    "a_clean_sinus": series_clean(),
    "b_single_ectopic": series_ectopic(),
    "c_bigeminy": series_bigeminy(),
    "d_artifact_spike": series_artifact_spike(),
    "e_long_gap": series_long_gap(),
    "f_short_burst": series_short_burst(),
    "g_sinus_arrhythmia": series_sinus_arrhythmia(),
}

EXPECTED_QUALITY = {
    # index -> expected label (others normal_nn)
    "b_single_ectopic": {30: "ectopic_rr", 31: "ectopic_rr"},
    "c_bigeminy": "all_ectopic",
    "d_artifact_spike": {30: "artifact_rr"},
    "e_long_gap": {30: "artifact_rr"},
    "f_short_burst": {30: "artifact_rr", 31: "artifact_rr", 32: "artifact_rr"},
    "g_sinus_arrhythmia": {},
    "a_clean_sinus": {},
}


def rmssd(x: np.ndarray) -> float | None:
    if len(x) < 2:
        return None
    return float(np.sqrt(np.mean(np.diff(x) ** 2)))


def linear_reference(rr: np.ndarray, qual: list[str]) -> np.ndarray:
    """Independent reimplementation of documented linear-interp semantics."""
    n = len(rr)
    valid = [i for i, q in enumerate(qual) if q == "normal_nn"]
    if not valid:
        return np.array([])
    if len(valid) == 1:
        return np.full(n, rr[valid[0]])
    out = np.empty(n)
    for i in range(n):
        if qual[i] == "normal_nn":
            out[i] = rr[i]
        elif i <= valid[0]:
            out[i] = rr[valid[0]]
        elif i >= valid[-1]:
            out[i] = rr[valid[-1]]
        else:
            right = next(j for j in valid if j > i)
            left = max(j for j in valid if j < i)
            alpha = (i - left) / (right - left)
            out[i] = (1 - alpha) * rr[left] + alpha * rr[right]
    return out


def main() -> None:
    bridge = LaminaBridge(repo_root=REPO, auto_build=False)
    rows = []
    verdicts = []
    for name, rr in SERIES.items():
        rr = np.asarray(rr, dtype=float)
        for pol in POLICIES:
            kw = {}
            if pol == "percent_threshold":
                kw["percent_threshold"] = PCT
            note = []
            try:
                r = bridge.hrv_correct(rr, policy=pol, **kw)
            except Exception as exc:  # lamina_error (e.g. EmptySignal)
                rows.append({
                    "series": name, "policy": pol,
                    "param": PCT if pol == "percent_threshold" else "",
                    "n_input": len(rr), "n_nn": "",
                    "rmssd_ms": "", "mean_nn_ms": "",
                    "nn_values_json": "", "quality_labels_json": "",
                    "notes": f"ERROR: {exc}",
                })
                verdicts.append((name, pol, "error", str(exc)))
                continue
            nn = np.array(r["nn_intervals_ms"], dtype=float)
            qual = r["interval_quality"]
            n_input = r["n_input_intervals"]
            assert n_input == len(rr), (n_input, len(rr))

            # ---- policy semantics verification ----
            ok = True
            if pol == "none":
                ok = np.allclose(nn, rr, rtol=0, atol=1e-9)
                note.append("passthrough_exact(ulp)" if ok else "MISMATCH")
            elif pol == "reject_invalid":
                expect = np.array([v for v, q in zip(rr, qual)
                                   if q == "normal_nn"])
                ok = np.allclose(nn, expect, rtol=0, atol=1e-12)
                note.append(f"removed_{n_input - len(nn)}_invalid")
                if not ok:
                    note.append("MISMATCH_vs_classifier_subset")
            elif pol == "percent_threshold":
                expect = np.array([v for v, q in zip(rr, qual)
                                   if q == "normal_nn"])
                ok = np.allclose(nn, expect, rtol=0, atol=1e-12)
                note.append(f"band={PCT};removed_{n_input - len(nn)}")
                if not ok:
                    note.append("MISMATCH_vs_classifier_subset")
            else:  # interpolate_*
                ref = linear_reference(rr, qual)
                ok = (len(nn) == n_input and
                      np.allclose(nn, ref, rtol=0, atol=1e-9))
                note.append(f"filled_{int(np.sum([q != 'normal_nn' for q in qual]))}"
                            f"_len_preserved={len(nn) == n_input}")
                if not ok:
                    note.append("MISMATCH_vs_linear_reference")

            # independent rmssd cross-check vs Lamina's rmssd_ms
            my_rmssd = rmssd(nn)
            lam_rmssd = r["rmssd_ms"]
            if my_rmssd is not None and lam_rmssd is not None:
                if abs(my_rmssd - lam_rmssd) > 1e-9 * max(1, abs(my_rmssd)):
                    ok = False
                    note.append(f"RMSSD_MISMATCH lam={lam_rmssd} mine={my_rmssd}")

            # classification expectation (policy-independent; check once)
            if pol == "none":
                exp = EXPECTED_QUALITY[name]
                if exp == "all_ectopic":
                    # Documented classifier blind spot, not a harness failure:
                    # the rolling median INCLUDES the interval itself, so a
                    # strictly alternating bigeminy pattern makes the local
                    # median equal to the interval's own parity class and the
                    # 20% deviation test never fires.
                    n_ect = sum(q == "ectopic_rr" for q in qual)
                    note.append(
                        f"CLASSIFIER_BLINDSPOT: sustained bigeminy yields "
                        f"only {n_ect}/{len(qual)} ectopic_rr labels "
                        f"(self-inclusive local median == own parity class)")
                else:
                    bad = [i for i, q in enumerate(qual)
                           if q != exp.get(i, "normal_nn")]
                    if bad:
                        ok = False
                        note.append(f"QUALITY_MISMATCH@{bad}")
                    else:
                        note.append("quality_as_expected")

            rows.append({
                "series": name, "policy": pol,
                "param": PCT if pol == "percent_threshold" else "",
                "n_input": n_input, "n_nn": r["n_nn"],
                "rmssd_ms": lam_rmssd, "mean_nn_ms": r["mean_nn_ms"],
                "nn_values_json": json.dumps([round(float(v), 6) for v in nn]),
                "quality_labels_json": json.dumps(qual),
                "notes": ";".join(note),
            })
            verdicts.append((name, pol, "PASS" if ok else "FAIL", ";".join(note)))

    # ---- artifact bound boundary probes (classification only) ----
    for name, rr_list in SERIES_BOUNDARY.items():
        r = bridge.hrv_correct(np.asarray(rr_list, float), policy="none")
        qual = r["interval_quality"]
        val = rr_list[0]
        if val < 300.0 or val > 2000.0:
            expect = "artifact_rr"
        else:
            expect = "normal_nn"  # homogeneous neighborhood -> no ectopy flag
        got = sorted(set(qual))
        ok = all(q == expect for q in qual)
        rows.append({
            "series": name, "policy": "none", "param": "",
            "n_input": len(rr_list), "n_nn": r["n_nn"],
            "rmssd_ms": r["rmssd_ms"], "mean_nn_ms": r["mean_nn_ms"],
            "nn_values_json": json.dumps(r["nn_intervals_ms"]),
            "quality_labels_json": json.dumps(qual),
            "notes": f"boundary_probe expect={expect} got={got} "
                     f"{'PASS' if ok else 'FAIL'}",
        })
        verdicts.append((name, "none", "PASS" if ok else "FAIL",
                         f"expect={expect} got={got}"))

    # ---- percent_threshold band-semantics probe ----
    # Clean 800 ms series with one isolated 700 ms interval: 12.5% deviation
    # from the local median. Must be REJECTED at p=0.05 and RETAINED at
    # p=0.20 if the stated band is really applied. (Note: smooth sinusoidal
    # modulation is never flagged at any p because the self-inclusive local
    # window median tracks the trend — see waveF-notes.md.)
    rr_band = clean_sinus()
    rr_band[30] = 700.0
    r05 = bridge.hrv_correct(rr_band, policy="percent_threshold",
                             percent_threshold=0.05)
    r20 = bridge.hrv_correct(rr_band, policy="percent_threshold",
                             percent_threshold=0.20)
    rem05 = len(rr_band) - r05["n_nn"]
    rem20 = len(rr_band) - r20["n_nn"]
    band_ok = rem05 == 1 and rem20 == 0
    rows.append({
        "series": "h_band_probe_700ms", "policy": "percent_threshold",
        "param": 0.05, "n_input": len(rr_band), "n_nn": r05["n_nn"],
        "rmssd_ms": r05["rmssd_ms"], "mean_nn_ms": r05["mean_nn_ms"],
        "nn_values_json": json.dumps(r05["nn_intervals_ms"]),
        "quality_labels_json": json.dumps(r05["interval_quality"]),
        "notes": f"isolated 700ms (12.5% dev): p=0.05 removed {rem05}, "
                 f"p=0.20 removed {rem20} -> stated band applied="
                 f"{'PASS' if band_ok else 'FAIL'}",
    })
    verdicts.append(("h_band_probe_700ms", "percent_threshold@0.05_vs_0.20",
                     "PASS" if band_ok else "FAIL",
                     f"removed p05={rem05} p20={rem20}"))

    # ---- NaN / non-finite input probe (IntervalQuality::Missing) ----
    try:
        r = bridge.run_op("hrv-correct", {
            "rr_intervals_ms": [800.0, None, 810.0],
            "config": {"policy": "none"}})
        verdicts.append(("nan_probe", "none", "INFO",
                         f"accepted: {r.get('interval_quality')}"))
        nan_note = f"null-in-array accepted; quality={r.get('interval_quality')}"
    except Exception as exc:
        verdicts.append(("nan_probe", "none", "INFO",
                         f"rejected_at_bridge: {exc}"))
        nan_note = f"null-in-array rejected by bridge (exit/protocol): {exc}"
    rows.append({
        "series": "nan_probe", "policy": "none", "param": "",
        "n_input": 3, "n_nn": "", "rmssd_ms": "", "mean_nn_ms": "",
        "nn_values_json": "", "quality_labels_json": "",
        "notes": nan_note,
    })

    with open(OUT / "policy_matrix.csv", "w", newline="") as fh:
        w = csv.DictWriter(fh, fieldnames=list(rows[0].keys()))
        w.writeheader()
        w.writerows(rows)

    print(f"wrote {OUT / 'policy_matrix.csv'} ({len(rows)} rows)")
    fails = [v for v in verdicts if v[2] == "FAIL"]
    for v in verdicts:
        print(v)
    print(f"\nFAILS: {len(fails)}")


if __name__ == "__main__":
    main()
