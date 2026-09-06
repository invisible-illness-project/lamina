"""Validation runner + machine-readable result writers (SPEC §6).

Failure-handling contract (task §12): one failing dataset/recording never
aborts the suite. Every recording is processed inside try/except; failures are
recorded with a traceback into ``results/logs/<dataset>.log`` and counted.
"""

from __future__ import annotations

import csv
import json
import logging
import traceback
from dataclasses import dataclass, field
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

from .bridge import LaminaBridge, LaminaBridgeError
from .datasets.base import AccessReport, AccessStatus, DatasetAdapter, Recording
from .metrics import peak_detection_metrics, rate_metrics

#: Default event-matching tolerances (seconds) per comparison type.
DEFAULT_TOLERANCES: dict[str, float] = {
    "peak_match_sec": 0.150,   # ECG/PPG beat matching (task verification: 150 ms)
    "rsp_match_sec": 0.500,    # respiratory event matching
    "scr_match_sec": 1.000,    # EDA SCR onset matching
}

#: Access statuses that mean "do not iterate recordings".
_SKIP_STATUSES = {
    AccessStatus.INACCESSIBLE,
    AccessStatus.UNSUPPORTED_FORMAT,
    AccessStatus.NOT_ATTEMPTED,
}


@dataclass
class RunOptions:
    subjects: list[str] | None = None
    recordings: list[str] | None = None
    max_recordings: int | None = None
    seed: int = 0
    smoke: bool = False
    cache_dir: str | None = None
    tolerances: dict[str, float] = field(default_factory=lambda: dict(DEFAULT_TOLERANCES))


@dataclass
class RecordingResult:
    dataset: str
    subject_id: str
    recording_id: str
    status: str                       # "ok" | "failed"
    modalities: list[str] = field(default_factory=list)
    metrics: dict[str, float | int | None] = field(default_factory=dict)
    error: str | None = None


@dataclass
class DatasetRunResult:
    dataset: str
    status: str
    reason: str = ""
    access: AccessReport | None = None
    recordings: list[RecordingResult] = field(default_factory=list)
    started_at: str = ""
    finished_at: str = ""
    error: str | None = None

    @property
    def n_ok(self) -> int:
        return sum(1 for r in self.recordings if r.status == "ok")

    @property
    def n_failed(self) -> int:
        return sum(1 for r in self.recordings if r.status == "failed")


# ---------------------------------------------------------------------------
# Per-recording processing (modality-dispatched bridge calls)
# ---------------------------------------------------------------------------

def process_recording(
    recording: Recording,
    bridge: LaminaBridge,
    tolerances: dict[str, float] | None = None,
) -> dict[str, float | int | None]:
    """Run Lamina (via bridge) on a recording and compute defensible metrics.

    Metrics are only computed when a corresponding reference exists; otherwise
    structural/execution metrics (counts, rates) are reported and labelled by
    their ``structural_`` prefix.
    """
    tol = dict(DEFAULT_TOLERANCES)
    if tolerances:
        tol.update(tolerances)
    out: dict[str, float | int | None] = {}
    refs = recording.references

    for ch_name, sig in recording.signals.items():
        mod = sig.modality
        fs = sig.sampling_rate
        prefix = f"{mod}_{ch_name}".replace(" ", "_")
        if mod == "ecg":
            r = bridge.ecg_peaks(sig.samples, fs)
            out[f"{prefix}_n_peaks"] = r["count"]
            ref = refs.get("ecg_peak_indices")
            if ref is not None:
                for k, v in peak_detection_metrics(
                    ref, r["peaks"], fs, tol["peak_match_sec"]
                ).items():
                    out[f"{prefix}_{k}"] = v
            else:
                out[f"structural_{prefix}_n_peaks"] = r.pop("count")
        elif mod in ("ppg", "bvp"):
            r = bridge.ppg_peaks(sig.samples, fs)
            out[f"{prefix}_n_peaks"] = r["count"]
            ref = refs.get("ppg_peak_indices") or refs.get("bvp_peak_indices")
            if ref is not None:
                for k, v in peak_detection_metrics(
                    ref, r["peaks"], fs, tol["peak_match_sec"]
                ).items():
                    out[f"{prefix}_{k}"] = v
            else:
                out[f"structural_{prefix}_n_peaks"] = r.pop("count")
        elif mod == "rsp":
            r = bridge.rsp_cycles(sig.samples, fs)
            out[f"{prefix}_n_cycles"] = r["count"]
            rates = [c["respiratory_rate_bpm"] for c in r["cycles"]]
            if rates:
                out[f"structural_{prefix}_mean_rate_bpm"] = (
                    sum(rates) / len(rates)
                )
            ref = refs.get("rsp_peak_indices")
            if ref is not None:
                insp = [c["inspiration_index"] for c in r["cycles"]]
                for k, v in peak_detection_metrics(
                    ref, insp, fs, tol["rsp_match_sec"]
                ).items():
                    out[f"{prefix}_{k}"] = v
        elif mod == "eda":
            r = bridge.eda_peaks(sig.samples, fs)
            out[f"structural_{prefix}_n_scr"] = r["count"]
            ref = refs.get("scr_onset_indices")
            if ref is not None:
                onsets = [e["onset_index"] for e in r["events"]]
                for k, v in peak_detection_metrics(
                    ref, onsets, fs, tol["scr_match_sec"]
                ).items():
                    out[f"{prefix}_{k}"] = v

    # Recording-level rate references (e.g. dataset-provided HR time series).
    if "hr_bpm" in refs and "hr_bpm_estimated" in refs:
        for k, v in rate_metrics(refs["hr_bpm"], refs["hr_bpm_estimated"]).items():
            out[f"hr_{k}"] = v
    return out


# ---------------------------------------------------------------------------
# Dataset-level run
# ---------------------------------------------------------------------------

def run_dataset(
    adapter: DatasetAdapter,
    bridge: LaminaBridge | None = None,
    options: RunOptions | None = None,
    results_dir: str | Path | None = None,
) -> DatasetRunResult:
    """Run validation for one dataset with per-recording exception isolation."""
    options = options or RunOptions()
    started = datetime.now(timezone.utc).isoformat(timespec="seconds")
    info = adapter.info()
    log = _dataset_logger(results_dir, info.key)

    result = DatasetRunResult(dataset=info.key, status=AccessStatus.FAILED.value,
                              started_at=started)
    try:
        access = adapter.check_access(cache_dir=options.cache_dir)
    except Exception:
        log.error("check_access raised:\n%s", traceback.format_exc())
        result.status = AccessStatus.FAILED.value
        result.error = "check_access raised an exception (see logs)"
        result.finished_at = datetime.now(timezone.utc).isoformat(timespec="seconds")
        return result
    result.access = access

    if access.status in _SKIP_STATUSES:
        result.status = access.status.value
        result.reason = access.reason
        result.finished_at = datetime.now(timezone.utc).isoformat(timespec="seconds")
        return result

    try:
        iterator = adapter.iter_recordings(
            subjects=options.subjects,
            recordings=options.recordings,
            max_recordings=options.max_recordings,
            seed=options.seed,
            smoke=options.smoke,
            cache_dir=options.cache_dir,
        )
        for rec in iterator:
            rr = RecordingResult(
                dataset=info.key,
                subject_id=rec.subject_id,
                recording_id=rec.recording_id,
                status="ok",
                modalities=sorted({s.modality for s in rec.signals.values()}),
            )
            try:
                if bridge is None:
                    bridge = LaminaBridge()
                rr.metrics = process_recording(rec, bridge, options.tolerances)
            except Exception as exc:  # noqa: BLE001 - per-recording isolation
                rr.status = "failed"
                rr.error = f"{type(exc).__name__}: {exc}"
                log.error("recording %s/%s failed:\n%s",
                          rec.subject_id, rec.recording_id, traceback.format_exc())
            result.recordings.append(rr)
    except Exception:
        log.error("iter_recordings failed:\n%s", traceback.format_exc())
        result.status = AccessStatus.FAILED.value
        result.error = "iter_recordings raised an exception (see logs)"
        result.finished_at = datetime.now(timezone.utc).isoformat(timespec="seconds")
        return result

    if result.recordings and result.n_failed == 0:
        result.status = AccessStatus.VALIDATED.value
    elif result.n_ok > 0:
        result.status = AccessStatus.PARTIALLY_VALIDATED.value
    elif result.recordings:
        result.status = AccessStatus.FAILED.value
    else:
        # Access reported OK but nothing was yielded.
        result.status = AccessStatus.FAILED.value
        result.error = "access ok but no recordings yielded"
    result.finished_at = datetime.now(timezone.utc).isoformat(timespec="seconds")
    return result


def run_all(
    adapters: list[DatasetAdapter],
    bridge: LaminaBridge | None = None,
    options: RunOptions | None = None,
    results_dir: str | Path | None = None,
) -> list[DatasetRunResult]:
    """Attempt every dataset; skip inaccessible; continue after failures."""
    results = []
    for adapter in adapters:
        results.append(run_dataset(adapter, bridge=bridge, options=options,
                                   results_dir=results_dir))
    return results


# ---------------------------------------------------------------------------
# Writers
# ---------------------------------------------------------------------------

def write_recordings_csv(path: str | Path, results: list[DatasetRunResult]) -> None:
    rows = []
    for d in results:
        for r in d.recordings:
            rows.append({
                "dataset": r.dataset, "subject_id": r.subject_id,
                "recording_id": r.recording_id, "status": r.status,
                "modalities": ";".join(r.modalities), "error": r.error or "",
            })
    _write_csv(path, ["dataset", "subject_id", "recording_id", "status",
                      "modalities", "error"], rows)


def write_metrics_csv(path: str | Path, results: list[DatasetRunResult]) -> None:
    rows = []
    for d in results:
        for r in d.recordings:
            for name, value in sorted(r.metrics.items()):
                rows.append({
                    "dataset": r.dataset, "subject_id": r.subject_id,
                    "recording_id": r.recording_id, "metric": name,
                    "value": "" if value is None else value,
                })
    _write_csv(path, ["dataset", "subject_id", "recording_id", "metric", "value"], rows)


def write_summary_csv(path: str | Path, results: list[DatasetRunResult]) -> None:
    rows = [{
        "dataset": d.dataset, "status": d.status,
        "recordings_ok": d.n_ok, "recordings_failed": d.n_failed,
        "reason": d.reason, "error": d.error or "",
        "started_at": d.started_at, "finished_at": d.finished_at,
    } for d in results]
    _write_csv(path, ["dataset", "status", "recordings_ok", "recordings_failed",
                      "reason", "error", "started_at", "finished_at"], rows)


def write_datasets_json(path: str | Path, results: list[DatasetRunResult],
                        adapters: dict[str, DatasetAdapter] | None = None) -> None:
    payload = []
    for d in results:
        info = adapters[d.dataset].info() if adapters and d.dataset in adapters else None
        payload.append({
            "dataset": d.dataset,
            "version": info.version if info else "unknown",
            "source": info.source_url if info else "",
            "access_status": d.status,
            "reason": d.reason,
            "subjects_evaluated": len({r.subject_id for r in d.recordings if r.status == "ok"}),
            "recordings_evaluated": d.n_ok,
            "modalities": info.modalities if info else [],
            "lamina_ops": info.lamina_ops if info else [],
            "lamina_version": _lamina_version(results_dir_hint=path),
            "git_commit": _git_commit(),
            "validation_date": d.finished_at or d.started_at,
            "metrics": _summarize_metrics(d),
            "limitations": [],
            "failures": [r.error for r in d.recordings if r.error],
        })
    Path(path).parent.mkdir(parents=True, exist_ok=True)
    Path(path).write_text(json.dumps(payload, indent=2))


def write_all(results_dir: str | Path, results: list[DatasetRunResult],
              adapters: dict[str, DatasetAdapter] | None = None) -> None:
    """Write the full standard result set (SPEC §6)."""
    rd = Path(results_dir)
    rd.mkdir(parents=True, exist_ok=True)
    write_recordings_csv(rd / "recordings.csv", results)
    write_metrics_csv(rd / "metrics.csv", results)
    write_summary_csv(rd / "summary.csv", results)
    write_datasets_json(rd / "datasets.json", results, adapters)


# ---------------------------------------------------------------------------
# internals
# ---------------------------------------------------------------------------

def _write_csv(path: str | Path, fieldnames: list[str], rows: list[dict]) -> None:
    Path(path).parent.mkdir(parents=True, exist_ok=True)
    with open(path, "w", newline="") as fh:
        writer = csv.DictWriter(fh, fieldnames=fieldnames)
        writer.writeheader()
        writer.writerows(rows)


def _dataset_logger(results_dir: str | Path | None, key: str) -> logging.Logger:
    logger = logging.getLogger(f"validation.runner.{key}")
    logger.setLevel(logging.INFO)
    logger.propagate = False
    if results_dir is not None and not any(
        isinstance(h, logging.FileHandler) for h in logger.handlers
    ):
        log_dir = Path(results_dir) / "logs"
        log_dir.mkdir(parents=True, exist_ok=True)
        handler = logging.FileHandler(log_dir / f"{key}.log")
        handler.setFormatter(logging.Formatter("%(asctime)s %(levelname)s %(message)s"))
        logger.addHandler(handler)
    return logger


def _summarize_metrics(d: DatasetRunResult) -> dict[str, float]:
    """Mean of each numeric metric across ok recordings (dataset-level rollup)."""
    acc: dict[str, list[float]] = {}
    for r in d.recordings:
        if r.status != "ok":
            continue
        for k, v in r.metrics.items():
            if isinstance(v, (int, float)) and v is not None:
                acc.setdefault(k, []).append(float(v))
    return {k: sum(vs) / len(vs) for k, vs in sorted(acc.items()) if vs}


def _git_commit() -> str:
    import subprocess
    try:
        return subprocess.run(
            ["git", "rev-parse", "HEAD"], capture_output=True, text=True, check=True
        ).stdout.strip()
    except Exception:
        return "unknown"


def _lamina_version(results_dir_hint=None) -> str:
    try:
        return LaminaBridge(auto_build=False).version()["lamina_version"]
    except Exception:
        return "unknown"
