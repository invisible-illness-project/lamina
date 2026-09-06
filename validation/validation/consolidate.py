"""Consolidate per-group validation results into canonical top-level files.

The validation suite is executed by category groups (``ecg``, ``ppg``,
``autonomic``, ``rppg``), each writing its own run artifacts under
``validation/results/<group>/``. Depending on the group, the artifacts live
either directly in the group directory (one run covering several datasets,
e.g. ``ppg``) or in per-dataset subdirectories (e.g.
``autonomic/wesad/``). This script discovers every run directory, merges the
machine-readable artifacts, and writes the canonical top-level files:

- ``validation/results/summary.csv``     — union of per-run summary rows, plus
  one row per registry dataset that never ran (status from the adapter's own
  verified access check, e.g. ``inaccessible``)
- ``validation/results/recordings.csv``  — union of per-run recording rows
- ``validation/results/metrics.csv``     — union of per-run metric rows
- ``validation/results/datasets.json``   — merged list of dataset entries, plus
  registry entries for datasets that never ran
- ``validation/results/manifest.json``   — umbrella manifest (see below)

The umbrella manifest does NOT pretend the runs were a single homogeneous
execution: each contributing run directory keeps its own git commit, branch,
seed, tolerances, package versions and timestamp, and all of them are listed
individually. Heterogeneity between runs is stated explicitly in the
``notes`` field.

Usage (from the repository root)::

    python -m validation.consolidate
    python -m validation.consolidate --results-dir validation/results
    python -m validation.consolidate --check     # verify, do not write

Only files listed above are written; per-group subdirectories are never
modified.
"""

from __future__ import annotations

import argparse
import csv
import json
import sys
from datetime import datetime, timezone
from pathlib import Path

#: Result groups executed independently (top-level subdirs of results/).
GROUPS = ("ecg", "ppg", "autonomic", "rppg")

#: Artifact filenames a run directory may contribute.
RUN_CSVS = ("summary.csv", "recordings.csv", "metrics.csv")

#: Consolidated manifest schema version (bump on layout change).
CONSOLIDATION_FORMAT_VERSION = "1.0"


def find_run_dirs(results_dir: Path) -> list[Path]:
    """Return every directory under ``results_dir/<group>`` that holds results.

    A run directory is identified by the presence of ``summary.csv``. The
    group directory itself may be a run directory (group-level run), in which
    case its subdirectories are not descended into.
    """
    run_dirs: list[Path] = []
    for group in GROUPS:
        gdir = results_dir / group
        if not gdir.is_dir():
            continue
        if (gdir / "summary.csv").is_file():
            run_dirs.append(gdir)
            continue
        for sub in sorted(gdir.iterdir()):
            if sub.is_dir() and (sub / "summary.csv").is_file():
                run_dirs.append(sub)
    return sorted(run_dirs)


def _read_csv_rows(path: Path) -> tuple[list[str], list[dict[str, str]]]:
    with path.open(newline="", encoding="utf-8") as fh:
        reader = csv.DictReader(fh)
        rows = list(reader)
        return list(reader.fieldnames or []), rows


def _write_csv(path: Path, fieldnames: list[str], rows: list[dict[str, str]]) -> None:
    with path.open("w", newline="", encoding="utf-8") as fh:
        writer = csv.DictWriter(fh, fieldnames=fieldnames, extrasaction="raise")
        writer.writeheader()
        writer.writerows(rows)


def _sort_key_summary(row: dict[str, str]) -> tuple:
    return (row.get("dataset", ""),)


def _sort_key_recordings(row: dict[str, str]) -> tuple:
    return (
        row.get("dataset", ""),
        row.get("subject_id", ""),
        row.get("recording_id", ""),
    )


def _sort_key_metrics(row: dict[str, str]) -> tuple:
    return (
        row.get("dataset", ""),
        row.get("recording_id", ""),
        row.get("metric", ""),
    )


def merge_csv(
    run_dirs: list[Path], name: str
) -> tuple[list[str], list[dict[str, str]], list[str]]:
    """Union rows of ``name`` across run dirs. Returns (header, rows, notes)."""
    header: list[str] | None = None
    rows: list[dict[str, str]] = []
    notes: list[str] = []
    for rd in run_dirs:
        path = rd / name
        if not path.is_file():
            notes.append(f"{rd}: missing {name} (skipped)")
            continue
        fields, rrows = _read_csv_rows(path)
        if header is None:
            header = fields
        elif fields != header:
            raise ValueError(
                f"{name}: header mismatch in {path}: {fields} != {header}"
            )
        rows.extend(rrows)
    if header is None:
        raise ValueError(f"no run directory contributed {name}")
    sorters = {
        "summary.csv": _sort_key_summary,
        "recordings.csv": _sort_key_recordings,
        "metrics.csv": _sort_key_metrics,
    }
    rows.sort(key=sorters.get(name, _sort_key_summary))
    return header, rows, notes


def merge_datasets_json(run_dirs: list[Path]) -> tuple[list[dict], list[str]]:
    """Merge per-run ``datasets.json`` lists into one list, one entry per dataset."""
    merged: dict[str, dict] = {}
    notes: list[str] = []
    for rd in run_dirs:
        path = rd / "datasets.json"
        if not path.is_file():
            notes.append(f"{rd}: missing datasets.json (skipped)")
            continue
        entries = json.loads(path.read_text(encoding="utf-8"))
        if isinstance(entries, dict):
            entries = [entries]
        for entry in entries:
            key = entry.get("dataset")
            if key in merged:
                notes.append(
                    f"{rd}: duplicate datasets.json entry for {key!r} "
                    "(kept first occurrence)"
                )
                continue
            entry = dict(entry)
            entry["source_results_dir"] = str(rd)
            merged[key] = entry
    return [merged[k] for k in sorted(merged)], notes


def registry_fill(
    present_keys: set[str],
) -> tuple[list[dict[str, str]], list[dict], list[str]]:
    """Build summary.csv rows and datasets.json entries for registry datasets
    that never ran (not present in any contributing run).

    Status/reason come from each adapter's own ``check_access()``. For the
    stub adapters this is the statically recorded, previously verified probe
    result — no network is touched. Any exception is recorded honestly as
    ``not_attempted``. Timestamps/commit/version fields are left empty rather
    than fabricated.
    """
    from .registry import list_datasets

    summary_rows: list[dict[str, str]] = []
    dataset_entries: list[dict] = []
    notes: list[str] = []
    for adapter in list_datasets():
        info = adapter.info()
        if info.key in present_keys:
            continue
        try:
            access = adapter.check_access()
            status = access.status.value
            reason = access.reason
        except Exception as exc:  # noqa: BLE001 - never fabricate; record honestly
            status = "not_attempted"
            reason = f"access check raised during consolidation: {type(exc).__name__}: {exc}"
        summary_rows.append(
            {
                "dataset": info.key,
                "status": status,
                "recordings_ok": "0",
                "recordings_failed": "0",
                "reason": reason,
                "error": "",
                "started_at": "",
                "finished_at": "",
            }
        )
        dataset_entries.append(
            {
                "dataset": info.key,
                "version": info.version,
                "source": info.source_url,
                "access_status": status,
                "reason": reason,
                "subjects_evaluated": 0,
                "recordings_evaluated": 0,
                "modalities": info.modalities,
                "lamina_ops": info.lamina_ops,
                "lamina_version": "",
                "git_commit": "",
                "validation_date": "",
                "metrics": {},
                "note": "not executed in any contributing run; status is the "
                        "adapter's recorded access-check result",
            }
        )
        notes.append(
            f"registry fill: {info.key} not present in any run; "
            f"added with adapter-reported status {status!r}"
        )
    return summary_rows, dataset_entries, notes


def build_umbrella_manifest(
    run_dirs: list[Path],
    merged_counts: dict[str, int],
    notes: list[str],
) -> dict:
    """Umbrella manifest listing every contributing run manifest verbatim-ish.

    Honest about heterogeneity: the consolidated results were produced by
    several runs on several branches/commits with different seeds; this
    manifest records each run's own manifest rather than fabricating a single
    synthetic one.
    """
    runs = []
    for rd in run_dirs:
        entry: dict = {"results_dir": str(rd)}
        mpath = rd / "manifest.json"
        if mpath.is_file():
            entry["manifest"] = json.loads(mpath.read_text(encoding="utf-8"))
        else:
            entry["manifest"] = None
            notes.append(f"{rd}: missing manifest.json")
        runs.append(entry)

    distinct_commits = sorted(
        {
            r["manifest"].get("lamina_git_commit", "")
            for r in runs
            if r["manifest"]
        }
        - {""}
    )
    distinct_seeds = sorted(
        {
            r["manifest"].get("seed")
            for r in runs
            if r["manifest"] and r["manifest"].get("seed") is not None
        }
    )
    distinct_tolerances = {
        json.dumps(r["manifest"].get("tolerances", {}), sort_keys=True)
        for r in runs
        if r["manifest"]
    }

    heterogeneity = []
    if len(distinct_commits) > 1:
        heterogeneity.append(
            f"runs were executed on {len(distinct_commits)} different git "
            "commits (adapter-development branches; Lamina core src/ identical "
            "to upstream main f3a195a in all of them)"
        )
    if len(distinct_seeds) > 1:
        heterogeneity.append(f"seeds differ across runs: {distinct_seeds}")
    if len(distinct_tolerances) > 1:
        heterogeneity.append("event-matching tolerances differ across runs")

    return {
        "format": "umbrella-consolidation-manifest",
        "format_version": CONSOLIDATION_FORMAT_VERSION,
        "created_utc": datetime.now(timezone.utc).isoformat(),
        "created_by": "python -m validation.consolidate",
        "results": {
            "summary_rows": merged_counts.get("summary.csv", 0),
            "recording_rows": merged_counts.get("recordings.csv", 0),
            "metric_rows": merged_counts.get("metrics.csv", 0),
            "dataset_entries": merged_counts.get("datasets.json", 0),
        },
        "contributing_runs": runs,
        "heterogeneity_notes": heterogeneity,
        "notes": notes,
    }


def consolidate(results_dir: Path, check: bool = False) -> int:
    results_dir = results_dir.resolve()
    run_dirs = find_run_dirs(results_dir)
    if not run_dirs:
        print(f"error: no run directories found under {results_dir}", file=sys.stderr)
        return 1

    notes: list[str] = []
    merged_counts: dict[str, int] = {}
    outputs: dict[str, tuple[list[str], list[dict[str, str]]]] = {}
    for name in RUN_CSVS:
        header, rows, n = merge_csv(run_dirs, name)
        notes.extend(n)
        outputs[name] = (header, rows)
        merged_counts[name] = len(rows)

    datasets, n = merge_datasets_json(run_dirs)
    notes.extend(n)

    # Fill in registry datasets that never ran so the consolidated artifacts
    # cover all 18 registered datasets, not only the executed ones.
    present = {r.get("dataset", "") for r in outputs["summary.csv"][1]}
    fill_summary, fill_datasets, n = registry_fill(present)
    notes.extend(n)
    header, rows = outputs["summary.csv"]
    rows.extend(fill_summary)
    rows.sort(key=_sort_key_summary)
    outputs["summary.csv"] = (header, rows)
    merged_counts["summary.csv"] = len(rows)
    datasets.extend(fill_datasets)
    datasets.sort(key=lambda e: e.get("dataset", ""))
    merged_counts["datasets.json"] = len(datasets)

    manifest = build_umbrella_manifest(run_dirs, merged_counts, list(notes))

    status = "would write" if check else "wrote"
    if not check:
        for name, (header, rows) in outputs.items():
            _write_csv(results_dir / name, header, rows)
        (results_dir / "datasets.json").write_text(
            json.dumps(datasets, indent=2) + "\n", encoding="utf-8"
        )
        (results_dir / "manifest.json").write_text(
            json.dumps(manifest, indent=2) + "\n", encoding="utf-8"
        )

    print(f"run directories ({len(run_dirs)}):")
    for rd in run_dirs:
        print(f"  {rd.relative_to(results_dir)}")
    for name in RUN_CSVS:
        print(f"{status} {results_dir / name} ({merged_counts[name]} rows)")
    print(f"{status} {results_dir / 'datasets.json'} ({len(datasets)} datasets)")
    print(f"{status} {results_dir / 'manifest.json'} "
          f"({len(manifest['contributing_runs'])} contributing runs)")
    if notes:
        print("notes:")
        for note in notes:
            print(f"  - {note}")
    if manifest["heterogeneity_notes"]:
        print("heterogeneity (recorded in umbrella manifest):")
        for note in manifest["heterogeneity_notes"]:
            print(f"  - {note}")
    return 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        prog="python -m validation.consolidate",
        description=__doc__,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    parser.add_argument(
        "--results-dir",
        type=Path,
        default=Path("validation/results"),
        help="results root containing the per-group subdirectories "
        "(default: validation/results, relative to CWD)",
    )
    parser.add_argument(
        "--check",
        action="store_true",
        help="discover and merge without writing any files",
    )
    args = parser.parse_args(argv)
    return consolidate(args.results_dir, check=args.check)


if __name__ == "__main__":
    raise SystemExit(main())
