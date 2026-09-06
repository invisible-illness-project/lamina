#!/usr/bin/env python3
"""Extra analyses for the autonomic/wearable dataset group.

Reads the runner outputs under ``validation/results/autonomic/<dataset>/`` and
the locally cached raw data ($HOME/data) and writes ``metrics_extra.csv``
(dataset, recording_id, metric, value, units, notes).

These are CONSISTENCY / KNOWN-GROUPS analyses, not ground-truth accuracy:
- WESAD: condition contrasts (stress vs baseline) for Lamina-derived HR, SCR
  rate and respiratory rate (expected direction: stress > baseline).
- autonomic-aging: bridge HRV (RMSSD, mean NN) per record and age-stratum
  medians (expected direction: RMSSD decreases with age group).
- wearable-exam-stress: Lamina BVP inter-beat intervals vs Empatica E4 IBI.csv
  (proprietary reference) distribution comparison.
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
from validation.datasets.autonomic import (  # noqa: E402
    AutonomicAgingAdapter,
    _parse_e4_ibi,
    _default_cache_dir,
)

RESULTS = REPO / "validation" / "results" / "autonomic"
OUT = RESULTS / "metrics_extra.csv"
ROWS: list[dict] = []


def emit(dataset: str, recording_id: str, metric: str, value, units: str,
         notes: str = "") -> None:
    ROWS.append({
        "dataset": dataset, "recording_id": recording_id, "metric": metric,
        "value": "" if value is None else value, "units": units, "notes": notes,
    })


def _load_metrics(dataset: str) -> dict[tuple[str, str], dict[str, float]]:
    """(subject, recording) -> {metric: value} from the runner metrics.csv."""
    out: dict[tuple[str, str], dict[str, float]] = {}
    path = RESULTS / dataset / "metrics.csv"
    if not path.exists():
        return out
    with open(path, newline="") as fh:
        for row in csv.DictReader(fh):
            key = (row["subject_id"], row["recording_id"])
            try:
                val = float(row["value"])
            except (TypeError, ValueError):
                continue
            out.setdefault(key, {})[row["metric"]] = val
    return out


# ---------------------------------------------------------------------------
def wesad_condition_contrasts() -> None:
    m = _load_metrics("wesad")
    by_cond: dict[str, dict[str, list[float]]] = {}
    for (subj, rid), metrics in m.items():
        cond = rid.split("_", 1)[1] if "_" in rid else ""
        dur_min = 4.0  # _WESAD_WINDOW_SEC (documented in adapter)
        hr = metrics.get("structural_ecg_chest_ecg_n_peaks", np.nan) / dur_min
        scr = metrics.get("structural_eda_chest_eda_n_scr", np.nan) / dur_min
        rr = metrics.get("structural_rsp_chest_rsp_mean_rate_bpm", np.nan)
        by_cond.setdefault(cond, {"hr": [], "scr": [], "rr": []})
        by_cond[cond]["hr"].append(hr)
        by_cond[cond]["scr"].append(scr)
        by_cond[cond]["rr"].append(rr)
        emit("wesad", rid, "condition_mean_hr_bpm", round(hr, 2), "bpm",
             "known-groups analysis; Lamina chest-ECG peaks / 4-min window; "
             "structural (no ground truth)")
        emit("wesad", rid, "condition_scr_rate_per_min", round(scr, 2), "1/min",
             "known-groups analysis; Lamina chest-EDA SCR count / 4-min window; "
             "structural (no ground truth)")
    for metric, unit in (("hr", "bpm"), ("scr", "1/min"), ("rr", "brpm")):
        base = np.nanmedian(by_cond.get("baseline", {}).get(metric, [np.nan]))
        stress = np.nanmedian(by_cond.get("stress", {}).get(metric, [np.nan]))
        emit("wesad", "ALL", f"known_groups_median_{metric}_baseline",
             round(float(base), 3), unit, "median across subjects (baseline)")
        emit("wesad", "ALL", f"known_groups_median_{metric}_stress",
             round(float(stress), 3), unit, "median across subjects (stress)")
        emit("wesad", "ALL", f"known_groups_delta_{metric}_stress_minus_baseline",
             round(float(stress - base), 3), unit,
             "expected positive for hr/scr (stress > baseline); population-trend "
             "consistency check, NOT accuracy validation")


# ---------------------------------------------------------------------------
def autonomic_aging_hrv() -> None:
    bridge = LaminaBridge(auto_build=False)
    adapter = AutonomicAgingAdapter()
    cache = _default_cache_dir() / "autonomic-aging"
    groups: dict[str, list[float]] = {}
    for rec in adapter.iter_recordings(cache_dir=str(cache)):
        sig = rec.signals["ecg1"]
        peaks = bridge.ecg_peaks(sig.samples, sig.sampling_rate)["peaks"]
        h = bridge.hrv(peaks, sig.n_samples, sig.sampling_rate)
        stratum = rec.metadata["age_stratum"]
        emit("autonomic-aging", rec.recording_id, "rmssd_ms", h["rmssd_ms"], "ms",
             f"bridge hrv op on Lamina ECG1 peaks; stratum={stratum}; "
             "structural (no ground truth)")
        emit("autonomic-aging", rec.recording_id, "mean_nn_ms", h["mean_nn_ms"],
             "ms", f"bridge hrv op; stratum={stratum}")
        if h["rmssd_ms"] is not None:
            groups.setdefault(stratum, []).append(h["rmssd_ms"])
    order = ["young_18_39", "middle_40_69", "old_70_plus"]
    meds = []
    for s in order:
        vals = groups.get(s, [])
        med = float(np.median(vals)) if vals else None
        meds.append(med)
        emit("autonomic-aging", "ALL", f"rmssd_median_{s}",
             None if med is None else round(med, 2), "ms",
             f"group median over n={len(vals)} records; population-trend "
             "consistency check (expected: decreasing with age)")
    finite = [m for m in meds if m is not None]
    emit("autonomic-aging", "ALL", "rmssd_monotonic_decrease_with_age",
         int(len(finite) == 3 and finite[0] >= finite[1] >= finite[2]),
         "bool", "1 = medians non-increasing young->middle->old (expected "
         "physiological direction); consistency check, not accuracy")


# ---------------------------------------------------------------------------
def exam_stress_ibi() -> None:
    bridge = LaminaBridge(auto_build=False)
    cache = _default_cache_dir() / "wearable-exam-stress"
    for student in ("S1", "S2", "S3", "S4", "S5"):
        for session in ("midterm_1", "midterm_2", "Final"):
            sdir = cache / student / session
            bvp_p, ibi_p = sdir / "BVP.csv", sdir / "IBI.csv"
            if not (bvp_p.exists() and ibi_p.exists()):
                continue
            rid = f"{student}_{session}"
            try:
                import csv as _csv
                with open(bvp_p) as fh:
                    bvp_epoch = float(fh.readline().split(",")[0])
                    bvp_fs = float(fh.readline().split(",")[0])
                    bvp = np.loadtxt(fh, delimiter=",", ndmin=1)
                epoch, t_ibi, ibi = _parse_e4_ibi(ibi_p)
                peaks = np.asarray(bridge.ppg_peaks(bvp, bvp_fs)["peaks"], float)
                lam_ibi = np.diff(peaks) / bvp_fs  # seconds
                lam_t = peaks[1:] / bvp_fs
                # align each E4 IBI to nearest Lamina interval within 0.5 s
                diffs = []
                for t, v in zip(t_ibi, ibi):
                    j = np.searchsorted(lam_t, t)
                    cand = [lam_ibi[k] for k in (j - 1, j)
                            if 0 <= k < len(lam_ibi)]
                    cand_t = [lam_t[k] for k in (j - 1, j)
                              if 0 <= k < len(lam_ibi)]
                    for c, ct in zip(cand, cand_t):
                        if abs(ct - t) <= 0.5:
                            diffs.append(abs(c - v))
                            break
                mae = float(np.mean(diffs)) if diffs else None
                emit("wearable-exam-stress", rid, "ibi_aligned_mae_ms",
                     None if mae is None else round(mae * 1000, 1), "ms",
                     f"Lamina BVP intervals vs E4 IBI.csv (proprietary "
                     "reference); nearest-interval alignment ±0.5 s; "
                     f"n_matched={len(diffs)}; consistency check")
                # Clean-segment-gated HR agreement: this dataset has long
                # off-wrist/flat BVP stretches; gate to segments with signal
                # energy before comparing with E4 HR.csv.
                per_sec_std = np.std(bvp[: bvp.size // 64 * 64].reshape(-1, 64),
                                     axis=1)
                active = per_sec_std > 1.0  # BVP units; flat segments ~ 0
                hr_path = sdir / "HR.csv"
                with open(hr_path) as fh2:
                    hr_epoch = float(fh2.readline().split(",")[0])
                    fh2.readline()
                    e4_hr = np.loadtxt(fh2, delimiter=",", ndmin=1)
                lam_ibi_t = peaks[1:] / bvp_fs
                lam_hr_inst = 60.0 * bvp_fs / np.diff(peaks)  # bpm
                ok_rr = (lam_hr_inst > 30) & (lam_hr_inst < 210)
                lam_hr_t = lam_ibi_t[ok_rr]
                lam_hr = lam_hr_inst[ok_rr]
                n_grid = min(len(e4_hr), len(active))
                gated_mae = None
                if lam_hr.size > 2 and n_grid > 10:
                    grid = np.arange(n_grid)
                    est = np.interp(grid, lam_hr_t, lam_hr,
                                    left=np.nan, right=np.nan)
                    mask = active[:n_grid] & np.isfinite(est)
                    if mask.sum() > 10:
                        gated_mae = float(np.mean(
                            np.abs(est[mask] - e4_hr[:n_grid][mask])))
                emit("wearable-exam-stress", rid, "hr_mae_active_segments_bpm",
                     None if gated_mae is None else round(gated_mae, 2), "bpm",
                     f"Lamina BVP HR vs E4 HR.csv restricted to seconds with "
                     f"BVP per-second std > 1 (off-wrist/flat stretches "
                     f"excluded); active fraction "
                     f"{float(np.mean(active)):.2f}; consistency check")
                emit("wearable-exam-stress", rid, "bvp_active_fraction",
                     round(float(np.mean(active)), 3), "ratio",
                     "fraction of seconds with BVP per-second std > 1 "
                     "(signal-contact quality indicator; DATASET property)")
                emit("wearable-exam-stress", rid, "ibi_median_lamina_ms",
                     round(float(np.median(lam_ibi)) * 1000, 1), "ms",
                     "distribution comparison (structural)")
                emit("wearable-exam-stress", rid, "ibi_median_e4_ms",
                     round(float(np.median(ibi)) * 1000, 1), "ms",
                     "E4 proprietary reference distribution")
            except Exception as exc:
                emit("wearable-exam-stress", rid, "ibi_comparison_error",
                     None, "", f"{type(exc).__name__}: {exc}")


def main() -> None:
    which = sys.argv[1] if len(sys.argv) > 1 else "all"
    if which in ("all", "wesad"):
        wesad_condition_contrasts()
    if which in ("all", "aging"):
        autonomic_aging_hrv()
    if which in ("all", "exam"):
        exam_stress_ibi()
    OUT.parent.mkdir(parents=True, exist_ok=True)
    # merge with existing rows for other datasets when run partially
    existing = []
    if OUT.exists():
        with open(OUT, newline="") as fh:
            existing = list(csv.DictReader(fh))
        done = {r["dataset"] for r in ROWS}
        existing = [r for r in existing if r["dataset"] not in done]
    fields = ["dataset", "recording_id", "metric", "value", "units", "notes"]
    with open(OUT, "w", newline="") as fh:
        w = csv.DictWriter(fh, fieldnames=fields)
        w.writeheader()
        for r in existing + ROWS:
            w.writerow(r)
    print(f"wrote {OUT} ({len(existing) + len(ROWS)} rows)")


if __name__ == "__main__":
    main()
