#!/usr/bin/env python3
"""Regenerate the validation report figures from the results CSVs.

Run from the repository root:

    python3 docs/validation/figures/generate_figures.py

Data sources (read-only):
    validation/results/**/metrics.csv
    validation/results/**/metrics_extra.csv
    validation/results/summary.csv (consolidated; falls back to per-group)

Outputs (written next to this script):
    fig_nstdb_snr.png          SNR robustness curve (F1 / recall vs SNR)
    fig_mitdb_f1.png           per-record F1 bar chart, 48 mitdb records
    fig_wrist_hr_activity.png  wrist-ppg-exercise HR error by activity
    fig_mitdb_timing_hist.png  mitdb per-record peak timing-error histogram
    fig_wesad_conditions.png   WESAD condition contrast (HR / SCR / RR)
    fig_scamps_hr_mae.png      SCAMPS per-algorithm HR MAE
    fig_dataset_status.png     dataset coverage/status summary table

The figures are referenced from docs/validation/public-dataset-validation.md.
"""

from __future__ import annotations

import csv
from pathlib import Path

import matplotlib

matplotlib.use("Agg")

import matplotlib.pyplot as plt
import numpy as np
import pandas as pd

HERE = Path(__file__).resolve().parent
REPO_ROOT = HERE.parents[2]
RESULTS = REPO_ROOT / "validation" / "results"

STATUS_COLORS = {
    "validated": "#2ca02c",
    "partially_validated": "#ff7f0e",
    "failed": "#d62728",
    "inaccessible": "#7f7f7f",
}


def _metrics(path: Path) -> pd.DataFrame:
    df = pd.read_csv(path, dtype={"subject_id": str, "recording_id": str})
    df["value"] = pd.to_numeric(df["value"], errors="coerce")
    return df


def _pivot(df: pd.DataFrame) -> pd.DataFrame:
    return df.pivot_table(
        index="recording_id", columns="metric", values="value", aggfunc="first"
    )


# ---------------------------------------------------------------- (a) nstdb
def fig_nstdb_snr() -> None:
    df = _metrics(RESULTS / "ecg" / "mit-bih-noise-stress" / "metrics.csv")
    piv = _pivot(df)
    # recording ids like 118e06 -> base 118 at 6 dB
    rows = []
    for rid, row in piv.iterrows():
        base, _, snr = str(rid).partition("e")
        rows.append(
            dict(
                base=base,
                snr=int(snr),
                f1=row["ecg_MLII_f1"],
                recall=row["ecg_MLII_recall"],
            )
        )
    d = pd.DataFrame(rows).sort_values("snr")
    fig, ax = plt.subplots(figsize=(6.4, 4.0))
    for base, marker in [("118", "o"), ("119", "s")]:
        sub = d[d.base == base]
        ax.plot(sub.snr, sub.f1, marker + "-", label=f"record {base} F1")
        ax.plot(sub.snr, sub.recall, marker + "--", alpha=0.6,
                label=f"record {base} recall")
    ax.set_xlabel("added noise SNR (dB)")
    ax.set_ylabel("score @ 150 ms tolerance")
    ax.set_ylim(0.7, 1.02)
    ax.set_title("MIT-BIH Noise Stress Test: beat detection vs SNR")
    ax.grid(alpha=0.3)
    ax.legend(fontsize=8, loc="lower right")
    fig.tight_layout()
    fig.savefig(HERE / "fig_nstdb_snr.png", dpi=150)
    plt.close(fig)


# --------------------------------------------------------------- (b) mitdb
def _mitdb_primary_channel(piv: pd.DataFrame) -> pd.DataFrame:
    """Return per-record frame with the evaluated channel's metrics.

    The adapter evaluates MLII where present, else the first channel (V5 for
    records 102/104).
    """
    chan = np.where(piv["ecg_MLII_f1"].notna(), "MLII", "V5")
    out = pd.DataFrame(index=piv.index)
    out["chan"] = chan
    for base in ("f1", "precision", "recall", "mean_abs_timing_error_sec"):
        out[base] = [
            piv.loc[rid, f"ecg_{c}_{base}"] for rid, c in zip(piv.index, chan)
        ]
    return out


def fig_mitdb_f1() -> None:
    df = _metrics(RESULTS / "ecg" / "mit-bih-arrhythmia" / "metrics.csv")
    rec = _mitdb_primary_channel(_pivot(df)).sort_index()
    fig, ax = plt.subplots(figsize=(11, 4.2))
    colors = ["#1f77b4" if c == "MLII" else "#9467bd" for c in rec.chan]
    ax.bar(rec.index.astype(str), rec.f1, color=colors)
    ax.axhline(rec.f1.mean(), color="k", ls=":", lw=1,
               label=f"mean F1 = {rec.f1.mean():.3f}")
    ax.set_ylim(0.6, 1.02)
    ax.set_ylabel("beat-detection F1 @ 150 ms")
    ax.set_xlabel("mitdb record (evaluated channel: MLII; V5 for 102/104)")
    ax.tick_params(axis="x", labelrotation=90, labelsize=6.5)
    ax.set_title("MIT-BIH Arrhythmia: per-record beat-detection F1 (48/48 records)")
    weakest = rec.f1.nsmallest(2)
    for rid, f1 in weakest.items():
        ax.annotate(
            f"{rid}: {f1:.3f}",
            xy=(list(rec.index).index(rid), f1),
            xytext=(0, -28),
            textcoords="offset points",
            ha="center",
            fontsize=8,
            color="#d62728",
            arrowprops=dict(arrowstyle="->", color="#d62728", lw=0.8),
        )
    ax.legend(loc="lower right", fontsize=8)
    fig.tight_layout()
    fig.savefig(HERE / "fig_mitdb_f1.png", dpi=150)
    plt.close(fig)


# --------------------------------------------------------------- (c) wrist
def fig_wrist_hr_activity() -> None:
    df = _metrics(RESULTS / "ppg" / "metrics.csv")
    piv = _pivot(df)
    wrist = piv.loc[[i for i in piv.index if "_" in str(i)]].copy()
    wrist["activity"] = [
        str(i).split("_", 1)[1].replace("_", " ") for i in wrist.index
    ]
    order = ["walk", "run", "low resistance bike", "high resistance bike"]
    data = [wrist.loc[wrist.activity == a, "hr_mae"].dropna().values for a in order]
    fig, ax = plt.subplots(figsize=(6.8, 4.2))
    bp = ax.boxplot(data, tick_labels=[f"{a}\n(n={len(d)})" for a, d in zip(order, data)],
                    showmeans=True, patch_artist=True)
    for patch in bp["boxes"]:
        patch.set_facecolor("#9ecae1")
    for i, d in enumerate(data, start=1):
        ax.scatter(np.full(len(d), i) + np.linspace(-0.08, 0.08, len(d)), d,
                   color="#08519c", zorder=3, s=18)
    means = [d.mean() for d in data]
    for i, m in enumerate(means, start=1):
        ax.annotate(f"mean {m:.1f}", xy=(i, m), xytext=(8, 4),
                    textcoords="offset points", fontsize=8, color="#08306b")
    ax.set_ylabel("HR MAE vs chest-ECG annotation HR (bpm)")
    ax.set_title("Wrist PPG During Exercise: HR-from-PPG error by activity")
    ax.grid(axis="y", alpha=0.3)
    fig.tight_layout()
    fig.savefig(HERE / "fig_wrist_hr_activity.png", dpi=150)
    plt.close(fig)


# ------------------------------------------------------ (d) mitdb timing
def fig_mitdb_timing_hist() -> None:
    df = _metrics(RESULTS / "ecg" / "mit-bih-arrhythmia" / "metrics.csv")
    rec = _mitdb_primary_channel(_pivot(df))
    ms = rec.mean_abs_timing_error_sec * 1000.0
    fig, ax = plt.subplots(figsize=(6.4, 4.0))
    ax.hist(ms, bins=np.arange(0, max(ms) + 2, 2), color="#1f77b4",
            edgecolor="white")
    ax.axvline(ms.mean(), color="#d62728", ls="--", lw=1,
               label=f"mean = {ms.mean():.1f} ms")
    ax.axvline(ms.median(), color="#2ca02c", ls=":", lw=1.5,
               label=f"median = {ms.median():.1f} ms")
    ax.set_xlabel("per-record mean absolute R-peak timing error (ms)")
    ax.set_ylabel("records")
    ax.set_title("MIT-BIH Arrhythmia: peak timing accuracy (360 Hz, 150 ms tolerance)")
    ax.legend(fontsize=9)
    ax.grid(alpha=0.3)
    fig.tight_layout()
    fig.savefig(HERE / "fig_mitdb_timing_hist.png", dpi=150)
    plt.close(fig)


# --------------------------------------------------------------- (e) wesad
def fig_wesad_conditions() -> None:
    ex = pd.read_csv(RESULTS / "autonomic" / "metrics_extra.csv")
    ex = ex[(ex.dataset == "wesad") & ex.metric.str.startswith("condition")]
    cond = ex.recording_id.str.extract(r"_(baseline|stress|amusement)$")[0]
    ex = ex.assign(condition=cond.values)
    piv = ex.pivot_table(
        index="recording_id", columns="metric", values="value", aggfunc="first"
    )
    piv["condition"] = ex.drop_duplicates("recording_id").set_index("recording_id")[
        "condition"
    ]
    # respiratory rate from the wesad metrics.csv (structural, no ground truth)
    wm = _metrics(RESULTS / "autonomic" / "wesad" / "metrics.csv")
    wpiv = _pivot(wm)
    piv["rr"] = wpiv["structural_rsp_chest_rsp_mean_rate_bpm"]

    conds = ["baseline", "amusement", "stress"]
    panels = [
        ("condition_mean_hr_bpm", "heart rate (bpm)"),
        ("condition_scr_rate_per_min", "SCR rate (1/min)"),
        ("rr", "respiratory rate (brpm)"),
    ]
    fig, axes = plt.subplots(1, 3, figsize=(10.5, 3.8), sharey=False)
    colors = {"baseline": "#2ca02c", "amusement": "#1f77b4", "stress": "#d62728"}
    for ax, (metric, ylabel) in zip(axes, panels):
        meds = [piv.loc[piv.condition == c, metric].median() for c in conds]
        ax.bar(conds, meds, color=[colors[c] for c in conds])
        for i, m in enumerate(meds):
            ax.text(i, m, f"{m:.1f}", ha="center", va="bottom", fontsize=9)
        ax.set_ylabel(ylabel)
        ax.set_ylim(0, max(meds) * 1.25)
        ax.tick_params(axis="x", rotation=20)
    fig.suptitle("WESAD known-groups contrast (medians over subjects S2-S6; "
                 "structural — no ground truth)")
    fig.tight_layout(rect=(0, 0, 1, 0.93))
    fig.savefig(HERE / "fig_wesad_conditions.png", dpi=150)
    plt.close(fig)


# -------------------------------------------------------------- (f) scamps
def fig_scamps_hr_mae() -> None:
    df = _metrics(RESULTS / "rppg" / "metrics.csv")
    piv = _pivot(df)
    piv["algo"] = [str(i).rsplit("-", 1)[-1] for i in piv.index]
    order = ["green", "chrom", "pos"]
    means = [piv.loc[piv.algo == a, "hr_mae"].mean() for a in order]
    mins = [piv.loc[piv.algo == a, "hr_mae"].min() for a in order]
    maxs = [piv.loc[piv.algo == a, "hr_mae"].max() for a in order]
    fig, ax = plt.subplots(figsize=(6.0, 4.0))
    bars = ax.bar(order, means, yerr=[np.array(means) - np.array(mins),
                                      np.array(maxs) - np.array(means)],
                  capsize=4, color=["#2ca02c", "#1f77b4", "#9467bd"])
    for b, m in zip(bars, means):
        ax.text(b.get_x() + b.get_width() / 2, m + 0.4, f"{m:.2f}",
                ha="center", fontsize=10)
    ax.set_ylabel("HR MAE vs embedded ground truth (bpm)")
    ax.set_title("SCAMPS synthetic rPPG: HR MAE by algorithm (10 videos; whiskers = min/max)")
    ax.grid(axis="y", alpha=0.3)
    fig.tight_layout()
    fig.savefig(HERE / "fig_scamps_hr_mae.png", dpi=150)
    plt.close(fig)


# ------------------------------------------------- (g) dataset status table
def fig_dataset_status() -> None:
    summary_path = RESULTS / "summary.csv"
    if summary_path.is_file():
        rows = list(csv.DictReader(summary_path.open()))
    else:  # fall back to per-group summaries
        rows = []
        for p in sorted(RESULTS.glob("**/summary.csv")):
            if p.parent == RESULTS:
                continue
            rows.extend(csv.DictReader(p.open()))
    attempted = {r["dataset"]: r["status"] for r in rows}
    inaccessible = {
        "pulsedb": "hosts unreachable",
        "mimic-iii-waveform": "credentialed access",
        "mimic-iii-ext-ppg": "credentialed access",
        "ubfc-phys": "91.7 GB min. archive",
        "pure": "e-mail application",
        "ubfc-rppg": "host blocked / Kaggle auth",
        "cohface": "restricted + IP-blocked",
        "mmpd": "signed agreement",
        "ibvp": "signed EULA",
    }
    entries = [(k, v, "") for k, v in sorted(attempted.items())]
    entries += [(k, "inaccessible", v) for k, v in sorted(inaccessible.items())]

    fig, ax = plt.subplots(figsize=(8.6, 0.42 * len(entries) + 1.2))
    ax.axis("off")
    cell_text = [[k, v.replace("_", " "), note] for k, v, note in entries]
    table = ax.table(
        cellText=cell_text,
        colLabels=["dataset", "validation status", "access note"],
        colWidths=[0.34, 0.28, 0.38],
        bbox=[0.0, 0.0, 1.0, 1.0],
        cellLoc="left",
    )
    table.auto_set_font_size(False)
    table.set_fontsize(9)
    for (r, c), cell in table.get_celld().items():
        cell.set_edgecolor("#cccccc")
        if r == 0:
            cell.set_facecolor("#eeeeee")
            cell.set_text_props(weight="bold")
        elif c == 1:
            status = entries[r - 1][1]
            cell.set_facecolor(STATUS_COLORS.get(status, "#ffffff") + "55")
    counts = {}
    for _, v, _ in entries:
        counts[v] = counts.get(v, 0) + 1
    title = "Dataset coverage: " + " · ".join(
        f"{n} {k.replace('_', ' ')}" for k, n in sorted(counts.items())
    )
    ax.set_title(title, fontsize=11, pad=14)
    fig.tight_layout()
    fig.savefig(HERE / "fig_dataset_status.png", dpi=150)
    plt.close(fig)


def main() -> None:
    for fn in (
        fig_nstdb_snr,
        fig_mitdb_f1,
        fig_wrist_hr_activity,
        fig_mitdb_timing_hist,
        fig_wesad_conditions,
        fig_scamps_hr_mae,
        fig_dataset_status,
    ):
        fn()
        print(f"wrote {fn.__name__}.png".replace("fig_", "figures/fig_", 1))


if __name__ == "__main__":
    main()
