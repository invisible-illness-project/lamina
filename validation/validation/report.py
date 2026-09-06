"""Human-readable validation report generator (task §13, SPEC §6).

Generates ``docs/validation/public-dataset-validation.md``. When run results
exist (``results/datasets.json``) the inventory table reflects them; otherwise
it falls back to the registry with ``not_attempted`` placeholders so the
skeleton is always generatable.
"""

from __future__ import annotations

import json
from datetime import datetime, timezone
from pathlib import Path

from .registry import list_datasets

_TEMPLATE_SECTIONS = [
    "## Methodology",
    "## Results",
    "## Robustness",
    "## Failures",
    "## Known/Potential Bugs",
    "## Limitations",
    "## Reproducibility",
]


def _inventory_rows(results_dir: Path) -> list[dict]:
    datasets_json = results_dir / "datasets.json"
    if datasets_json.exists():
        try:
            return json.loads(datasets_json.read_text())
        except Exception:
            pass
    # Fallback: registry-derived placeholder rows.
    rows = []
    for adapter in list_datasets():
        info = adapter.info()
        rows.append({
            "dataset": info.key,
            "access_status": "not_attempted",
            "modalities": info.modalities,
            "subjects_evaluated": 0,
            "recordings_evaluated": 0,
            "lamina_ops": info.lamina_ops,
        })
    return rows


def _inventory_table(rows: list[dict]) -> str:
    lines = [
        "| Dataset | Status | Signals | Subjects | Recordings | Validation Areas |",
        "| ------- | ------ | ------- | -------: | ---------: | ---------------- |",
    ]
    for r in rows:
        signals = ", ".join(r.get("modalities") or []) or "—"
        ops = r.get("lamina_ops") or []
        areas = ", ".join(ops) if ops else "—"
        lines.append(
            f"| {r.get('dataset', '?')} | {r.get('access_status', '?')} | {signals} "
            f"| {r.get('subjects_evaluated', 0)} | {r.get('recordings_evaluated', 0)} "
            f"| {areas} |"
        )
    return "\n".join(lines)


def generate(results_dir: str | Path, out_path: str | Path) -> Path:
    """Generate the validation report skeleton; returns the output path."""
    results_dir = Path(results_dir)
    out_path = Path(out_path)
    rows = _inventory_rows(results_dir)
    statuses: dict[str, int] = {}
    for r in rows:
        statuses[r.get("access_status", "unknown")] = (
            statuses.get(r.get("access_status", "unknown"), 0) + 1
        )
    status_lines = "\n".join(f"- {k}: {v}" for k, v in sorted(statuses.items()))

    manifest_note = ""
    if (results_dir / "manifest.json").exists():
        try:
            m = json.loads((results_dir / "manifest.json").read_text())
            manifest_note = (
                f"Lamina commit `{m.get('lamina_git_commit', 'unknown')}`, "
                f"framework {m.get('framework_version', 'unknown')}, "
                f"seed {m.get('seed', '?')}."
            )
        except Exception:
            pass

    text = f"""# Lamina Public-Dataset Validation Report

_Generated {datetime.now(timezone.utc).isoformat(timespec="seconds")} by
`python -m validation report`._

## Executive Summary

This report records the empirical validation status of the Lamina crate
against public physiological datasets. What was evaluated and what was not is
summarized by the dataset inventory below. {manifest_note}

Dataset status counts:

{status_lines}

## Dataset Inventory

{_inventory_table(rows)}

## Methodology

- Implementation under test: the Lamina Rust crate, exercised exclusively
  through the `lamina_bridge` JSON bridge (see `validation/SPEC.md` §3).
- Reference signals: dataset-provided annotations (beat annotations, contact
  PPG, respiratory labels) as documented per dataset adapter.
- Event matching: greedy one-to-one nearest-within-tolerance matching
  (`validation/metrics/events.py`); default peak tolerance 150 ms.
- Metrics: TP/FP/FN/precision/recall/F1 + timing error for events; MAE/RMSE/
  bias/correlation for rates; absolute/relative error for HRV (see
  `validation/metrics/`).
- Exclusions and preprocessing assumptions: documented per dataset in the
  adapter metadata and in Limitations below.

## Results

_To be populated by validation runs (Stage 3). Sections: ECG, PPG,
respiration, autonomic/EDA, signal quality, rPPG._

## Robustness

_To be populated: stratification by noise, motion, signal quality, recording
condition._

## Failures

_To be populated honestly from `results/summary.csv` and `results/logs/`._

## Known/Potential Bugs

See [`docs/validation/BUGS.md`](./BUGS.md). Bugs are documented, never fixed,
in this task.

## Limitations

- Datasets recorded as `inaccessible` / `not_attempted` above have no
  empirical validation coverage.
- Where no defensible ground truth exists, only structural/execution
  validation is reported and labelled as such.

## Reproducibility

```bash
python -m validation list
python -m validation check-access
python -m validation run --dataset all --seed 0
python -m validation report
```

See `validation/results/manifest.json` for exact code/data/parameter versions.
"""
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(text)
    return out_path
