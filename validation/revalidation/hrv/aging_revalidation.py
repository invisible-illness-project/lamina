#!/usr/bin/env python3
"""Wave F revalidation, task §4: autonomic-aging RMSSD re-run + corrected path.

Methodology
-----------
OLD (baseline, BUG-018): bridge op `hrv` on raw Lamina ECG1 detected peaks
-> RMSSD over ALL detected RR intervals, no ectopy/artifact handling.

NEW (this wave): same detected peaks -> bridge op `hrv-correct` with
policy=reject_invalid -> classify_intervals -> clean_rr_intervals ->
hrv_rmssd over the corrected N-N series. This path did not exist at baseline
(task §5 methodology-change allowance; documented in waveF-notes.md).

Per record we emit rmssd_baseline (from baseline metrics_extra.csv),
rmssd_current_raw (h rv op, same-as-baseline methodology, regression check),
rmssd_corrected (hrv-correct reject_invalid), plus quality-label counts.
"""
from __future__ import annotations

import csv
import json
import sys
from collections import Counter
from pathlib import Path

import numpy as np

REPO = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(REPO))
from validation.bridge import LaminaBridge  # noqa: E402
from validation.datasets.autonomic import AutonomicAgingAdapter  # noqa: E402

OUT = Path(__file__).resolve().parent
BASELINE_EXTRA = Path("/mnt/agents/output/reval-baseline/autonomic_metrics_extra.csv")
ORDER = ["young_18_39", "middle_40_69", "old_70_plus"]


def load_baseline() -> dict[str, float]:
    out = {}
    with open(BASELINE_EXTRA) as fh:
        for row in csv.DictReader(fh):
            if row["dataset"] == "autonomic-aging" and row["metric"] == "rmssd_ms" \
                    and row["recording_id"] != "ALL":
                out[row["recording_id"]] = float(row["value"])
    return out


def main() -> None:
    bridge = LaminaBridge(repo_root=REPO, auto_build=False)
    adapter = AutonomicAgingAdapter()
    cache = Path.home() / "data" / "autonomic-aging"
    baseline = load_baseline()

    rows = []
    groups_raw: dict[str, list[float]] = {}
    groups_corr: dict[str, list[float]] = {}
    for rec in adapter.iter_recordings(cache_dir=str(cache)):
        sig = rec.signals["ecg1"]
        rid = rec.recording_id
        stratum = rec.metadata["age_stratum"]
        peaks = bridge.ecg_peaks(sig.samples, sig.sampling_rate)["peaks"]
        raw = bridge.hrv(peaks, sig.n_samples, sig.sampling_rate)
        corr = bridge.hrv_correct(peaks=peaks, signal_length=sig.n_samples,
                                  fs=sig.sampling_rate, policy="reject_invalid")
        qc = Counter(corr["interval_quality"])
        base = baseline.get(rid)
        note = (f"n_peaks={len(peaks)} n_rr={corr['n_input_intervals']} "
                f"n_nn={corr['n_nn']} "
                f"ectopic={qc.get('ectopic_rr', 0)} "
                f"artifact={qc.get('artifact_rr', 0)} "
                f"missing={qc.get('missing', 0)}")
        delta_raw = (None if base is None or raw["rmssd_ms"] is None
                     else raw["rmssd_ms"] - base)
        rows.append({
            "record": rid, "stratum": stratum,
            "rmssd_baseline": base,
            "rmssd_current_raw": raw["rmssd_ms"],
            "rmssd_corrected": corr["rmssd_ms"],
            "delta": delta_raw,
            "notes": note,
        })
        if raw["rmssd_ms"] is not None:
            groups_raw.setdefault(stratum, []).append(raw["rmssd_ms"])
        if corr["rmssd_ms"] is not None:
            groups_corr.setdefault(stratum, []).append(corr["rmssd_ms"])
        print(rid, stratum, "base", base, "raw", raw["rmssd_ms"],
              "corr", corr["rmssd_ms"], note, flush=True)

    med_raw = [float(np.median(groups_raw[s])) for s in ORDER]
    med_corr = [float(np.median(groups_corr[s])) for s in ORDER]
    mono_raw = med_raw[0] >= med_raw[1] >= med_raw[2]
    mono_corr = med_corr[0] >= med_corr[1] >= med_corr[2]
    rows.append({
        "record": "ALL", "stratum": "medians",
        "rmssd_baseline": "52.26/24.92/142.84",
        "rmssd_current_raw": "/".join(f"{m:.2f}" for m in med_raw),
        "rmssd_corrected": "/".join(f"{m:.2f}" for m in med_corr),
        "delta": "",
        "notes": f"median young/middle/old; monotone_decreasing raw={mono_raw} "
                 f"corrected={mono_corr}",
    })

    with open(OUT / "aging_results.csv", "w", newline="") as fh:
        w = csv.DictWriter(fh, fieldnames=list(rows[0].keys()))
        w.writeheader()
        w.writerows(rows)
    print("\nmedians raw:", med_raw, "monotone:", mono_raw)
    print("medians corrected:", med_corr, "monotone:", mono_corr)
    print(f"wrote {OUT / 'aging_results.csv'}")


if __name__ == "__main__":
    main()
