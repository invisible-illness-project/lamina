"""ECG dataset adapters (Stage 2: implemented).

Datasets
--------
- ``mit-bih-arrhythmia`` (PhysioNet ``mitdb``): 48 half-hour 2-lead ECG
  recordings @ 360 Hz with expert beat annotations. Downloaded via ``wfdb``.
- ``mit-bih-noise-stress`` (PhysioNet ``nstdb``): ECG records with calibrated
  added noise at SNR levels 00/06/12/18/24 dB. physionet.org is unreachable
  (HTTP 403) from some networks, so this adapter downloads the required files
  from the static mirror ``archive.physionet.org/physiobank/database/nstdb``
  and reads them locally with ``wfdb``.

References produced per recording (runner contract, SPEC §6):
- ``ecg_peak_indices``: 0-based sample indices of annotated beats (same fs as
  the yielded signal), restricted to the MIT-BIH beat symbol set.
- ``hr_bpm``: reference heart-rate series derived from annotation beats.
- ``hr_bpm_estimated``: heart-rate series derived from Lamina-detected peaks
  (bridge ``ecg-peaks``), computed with the *identical* algorithm/grid so the
  runner's ``rate_metrics`` compares like with like.

HRV comparison (annotation vs Lamina peaks, bridge ``hrv`` op) is not part of
the runner's standard metrics (``Recording.metadata`` is not serialized), so
:func:`write_hrv_extra_csv` implements the documented post-run hook that writes
``<results_dir>/metrics_extra.csv``.
"""

from __future__ import annotations

import csv
import os
import random
import time
import urllib.error
import urllib.request
from pathlib import Path
from typing import Iterator

import numpy as np

from .base import (
    AccessReport,
    AccessStatus,
    DatasetAdapter,
    DatasetInfo,
    Recording,
)
from ..schema import Signal

#: MIT-BIH beat annotation symbols (everything else, e.g. '+', '~', '|', 'x',
#: is a non-beat annotation and is excluded from the peak references).
BEAT_SYMBOLS = frozenset(
    ("N", "L", "R", "A", "a", "J", "S", "V", "F", "e", "j", "E", "/", "f", "Q")
)

#: Preferred channel for both datasets; fall back to the first channel.
PREFERRED_CHANNEL = "MLII"

#: Heart-rate series settings (shared by reference and estimated series).
HR_GRID_STEP_SEC = 5.0        # uniform time grid step
HR_SMOOTH_BEATS = 8           # moving-average window (beats)
RR_MIN_SEC = 0.3              # plausible RR bounds (30-200 bpm); outliers
RR_MAX_SEC = 2.0              # excluded from the HR series

MITDB_RECORDS = [
    "100", "101", "102", "103", "104", "105", "106", "107", "108", "109",
    "111", "112", "113", "114", "115", "116", "117", "118", "119", "121",
    "122", "123", "124", "200", "201", "202", "203", "205", "207", "208",
    "209", "210", "212", "213", "214", "215", "217", "219", "220", "221",
    "222", "223", "228", "230", "231", "232", "233", "234",
]

#: nstdb subset used for robustness stratification (SNR in dB = name suffix).
NSTDB_RECORDS = [
    "118e00", "118e06", "118e12", "118e18", "118e24", "119e00", "119e24",
]
NSTDB_MIRROR = "https://archive.physionet.org/physiobank/database/nstdb"
NSTDB_EXTS = (".hea", ".atr", ".dat")


def _data_root() -> Path:
    """Local dataset cache root (override with LAMINA_VALIDATION_DATA)."""
    return Path(
        os.environ.get(
            "LAMINA_VALIDATION_DATA",
            Path.home() / ".cache" / "lamina-validation",
        )
    )


def _select_records(
    all_records: list[str],
    subjects: list[str] | None,
    recordings: list[str] | None,
    max_recordings: int | None,
    seed: int,
    smoke: bool,
    smoke_subset: list[str],
) -> list[str]:
    """Deterministic subset selection (adapter contract, base.py)."""
    recs = [r for r in all_records]
    if smoke:
        recs = [r for r in smoke_subset if r in recs]
    if subjects:
        keep = set(subjects)
        recs = [r for r in recs if r in keep]
    if recordings:
        keep = set(recordings)
        recs = [r for r in recs if r in keep]
    if max_recordings is not None and len(recs) > max_recordings:
        recs = sorted(random.Random(seed).sample(recs, max_recordings))
    return recs


def _beat_sample_indices(ann) -> np.ndarray:
    """0-based sample indices of beat annotations (beat symbols only)."""
    mask = np.array([s in BEAT_SYMBOLS for s in ann.symbol], dtype=bool)
    return np.asarray(ann.sample, dtype=np.int64)[mask]


def _rhythm_annotations(ann) -> list[str]:
    """Rhythm-change labels ('+' annotations carry the rhythm in aux_note)."""
    out = []
    for sym, aux in zip(ann.symbol, ann.aux_note or []):
        if sym == "+" and aux:
            label = aux.strip("\x00").strip().lstrip("(")
            if label and label not in out:
                out.append(label)
    return out


def _hr_series_from_peaks(peaks: np.ndarray, fs: float, grid: np.ndarray) -> np.ndarray:
    """Smoothed heart-rate (bpm) sampled on ``grid`` from a beat train.

    Instantaneous HR = 60/RR at the end of each RR interval, implausible RR
    intervals dropped, 8-beat moving average, linear interpolation onto the
    grid; NaN outside the covered time span. The SAME function is applied to
    annotation beats and Lamina beats so both series are comparable.
    """
    out = np.full(grid.shape, np.nan, dtype=float)
    peaks = np.sort(np.asarray(peaks, dtype=np.int64))
    if peaks.size < HR_SMOOTH_BEATS + 1:
        return out
    rr = np.diff(peaks) / fs
    t = peaks[1:] / fs
    ok = (rr >= RR_MIN_SEC) & (rr <= RR_MAX_SEC)
    rr, t = rr[ok], t[ok]
    if rr.size < HR_SMOOTH_BEATS:
        return out
    hr = 60.0 / rr
    kernel = np.ones(HR_SMOOTH_BEATS) / HR_SMOOTH_BEATS
    hr_smooth = np.convolve(hr, kernel, mode="same")
    # Edges of 'same'-mode convolution are attenuated; drop half a window.
    edge = HR_SMOOTH_BEATS // 2
    hr_smooth, t = hr_smooth[edge:-edge], t[edge:-edge]
    if t.size < 2:
        return out
    covered = (grid >= t[0]) & (grid <= t[-1])
    out[covered] = np.interp(grid[covered], t, hr_smooth)
    return out


def _pick_channel(record) -> tuple[np.ndarray, str]:
    """Return (samples, channel_name): MLII if present, else first channel."""
    names = list(record.sig_name)
    idx = names.index(PREFERRED_CHANNEL) if PREFERRED_CHANNEL in names else 0
    return np.asarray(record.p_signal[:, idx], dtype=float), names[idx]


def _lazy_bridge():
    from ..bridge import LaminaBridge

    return LaminaBridge(auto_build=False)


class _WfdbEcgAdapter(DatasetAdapter):
    """Shared machinery for wfdb-read ECG datasets with beat annotations."""

    key = ""
    db_name = ""
    all_records: list[str] = []
    smoke_records: list[str] = []

    # -- to implement ------------------------------------------------------
    def _ensure_files(self, data_dir: Path) -> tuple[int, str]:
        """Download/locate files; return (n_records_available, detail)."""
        raise NotImplementedError

    # -- contract ------------------------------------------------------------
    def check_access(self, cache_dir: str | None = None) -> AccessReport:
        data_dir = Path(cache_dir) if cache_dir else _data_root() / self.db_name
        try:
            n, detail = self._ensure_files(data_dir)
        except Exception as exc:  # noqa: BLE001 - honest access report
            return AccessReport(
                status=AccessStatus.INACCESSIBLE,
                reason=f"data acquisition failed: {type(exc).__name__}: {exc}",
                detail=f"cache dir: {data_dir}",
            )
        if n <= 0:
            return AccessReport(
                status=AccessStatus.INACCESSIBLE,
                reason="no records available after acquisition attempt",
                detail=detail,
            )
        return AccessReport(
            status=AccessStatus.VALIDATED,
            reason=f"{n}/{len(self.all_records)} records available locally",
            detail=detail,
        )

    def iter_recordings(
        self,
        subjects: list[str] | None = None,
        recordings: list[str] | None = None,
        max_recordings: int | None = None,
        seed: int = 0,
        smoke: bool = False,
        cache_dir: str | None = None,
    ) -> Iterator[Recording]:
        import wfdb

        data_dir = Path(cache_dir) if cache_dir else _data_root() / self.db_name
        recs = _select_records(
            self.all_records, subjects, recordings, max_recordings, seed,
            smoke, self.smoke_records,
        )
        bridge = None  # lazy: only needed for hr_bpm_estimated
        for rec_name in recs:
            rec_path = str(data_dir / rec_name)
            record = wfdb.rdrecord(rec_path)
            ann = wfdb.rdann(rec_path, "atr")
            fs = float(record.fs)
            samples, channel = _pick_channel(record)
            beats = _beat_sample_indices(ann)
            grid = np.arange(0.0, len(samples) / fs, HR_GRID_STEP_SEC)

            if bridge is None:
                bridge = _lazy_bridge()
            lamina_peaks = np.asarray(
                bridge.ecg_peaks(samples, fs)["peaks"], dtype=np.int64
            )

            signal = Signal(
                samples=samples,
                sampling_rate=fs,
                modality="ecg",
                units=record.units[list(record.sig_name).index(channel)]
                if getattr(record, "units", None)
                else None,
                subject_id=rec_name,
                recording_id=rec_name,
                channel=channel,
                annotations={"peak_indices": beats},
                metadata={"source": self.db_name},
            )
            yield Recording(
                recording_id=rec_name,
                subject_id=rec_name,
                signals={channel: signal},
                references={
                    "ecg_peak_indices": beats,
                    "hr_bpm": _hr_series_from_peaks(beats, fs, grid),
                    "hr_bpm_estimated": _hr_series_from_peaks(lamina_peaks, fs, grid),
                },
                metadata={
                    "channel": channel,
                    "fs": fs,
                    "n_beat_annotations": int(beats.size),
                    "n_lamina_peaks": int(lamina_peaks.size),
                    "rhythm_annotations": _rhythm_annotations(ann),
                    "comments": list(getattr(record, "comments", []) or []),
                    **self._extra_metadata(rec_name),
                },
            )

    def _extra_metadata(self, rec_name: str) -> dict:
        return {}


class MitBihArrhythmiaAdapter(_WfdbEcgAdapter):
    key = "mit-bih-arrhythmia"
    db_name = "mitdb"
    all_records = MITDB_RECORDS
    smoke_records = ["100", "101"]

    def info(self) -> DatasetInfo:
        return DatasetInfo(
            key=self.key,
            name="MIT-BIH Arrhythmia Database",
            category="ecg",
            modalities=["ecg"],
            source_url="https://physionet.org/content/mitdb/",
            license="ODC-BY 1.0",
            version="1.0.0",
            citation="Moody GB, Mark RG. The impact of the MIT-BIH Arrhythmia Database. "
                     "IEEE Eng in Med and Biol 20(3):45-50 (2001).",
            lamina_ops=["ecg-clean", "ecg-peaks", "hrv"],
            notes="48 half-hour 2-lead ECG @ 360 Hz with expert beat annotations. "
                  "Accessible via wfdb without credentials. Beat references use the "
                  "MIT-BIH beat symbol set only; channel MLII (fallback: first).",
        )

    def _ensure_files(self, data_dir: Path) -> tuple[int, str]:
        import wfdb

        data_dir.mkdir(parents=True, exist_ok=True)
        have = [
            r for r in self.all_records
            if all((data_dir / f"{r}{e}").exists() for e in (".hea", ".atr", ".dat"))
        ]
        if len(have) < len(self.all_records):
            wfdb.dl_database(
                self.db_name,
                dl_dir=str(data_dir),
                records=[r for r in self.all_records if r not in have],
                annotators=["atr"],
            )
        have = [
            r for r in self.all_records
            if all((data_dir / f"{r}{e}").exists() for e in (".hea", ".atr", ".dat"))
        ]
        return len(have), f"wfdb dl_database('mitdb') cache at {data_dir}"


class MitBihNoiseStressAdapter(_WfdbEcgAdapter):
    key = "mit-bih-noise-stress"
    db_name = "nstdb"
    all_records = NSTDB_RECORDS
    smoke_records = ["118e06", "119e24"]

    def info(self) -> DatasetInfo:
        return DatasetInfo(
            key=self.key,
            name="MIT-BIH Noise Stress Test Database",
            category="ecg",
            modalities=["ecg"],
            source_url="https://physionet.org/content/nstdb/",
            license="ODC-BY 1.0",
            version="1.0.0",
            citation="Moody GB, Muldrow WE, Mark RG. A noise stress test for arrhythmia "
                     "detectors. Computers in Cardiology 11:381-384 (1984).",
            lamina_ops=["ecg-clean", "ecg-peaks"],
            notes="12 half-hour ECG recordings with calibrated added noise (bw, ma, em) "
                  "@ 360 Hz; robustness stratification target. Downloaded from the "
                  f"static mirror {NSTDB_MIRROR} (physionet.org unreachable, HTTP 403, "
                  "from the validation host). Subset: "
                  + ", ".join(NSTDB_RECORDS)
                  + " (record base 118/119; suffix = SNR dB).",
        )

    def _extra_metadata(self, rec_name: str) -> dict:
        base, _, suffix = rec_name.partition("e")
        return {
            "snr_db": int(suffix) if suffix.isdigit() else None,
            "base_record": base,
        }

    def _ensure_files(self, data_dir: Path) -> tuple[int, str]:
        data_dir.mkdir(parents=True, exist_ok=True)
        n_ok, errors = 0, []
        for rec in self.all_records:
            try:
                for ext in NSTDB_EXTS:
                    dest = data_dir / f"{rec}{ext}"
                    if dest.exists() and dest.stat().st_size > 0:
                        continue
                    _download(f"{NSTDB_MIRROR}/{rec}{ext}", dest)
                n_ok += 1
            except Exception as exc:  # noqa: BLE001 - per-record isolation
                errors.append(f"{rec}: {type(exc).__name__}: {exc}")
        detail = f"mirror {NSTDB_MIRROR} -> {data_dir}"
        if errors:
            detail += "; failed: " + "; ".join(errors)
        return n_ok, detail


def _download(url: str, dest: Path, retries: int = 4, timeout: float = 300.0) -> None:
    """Download ``url`` to ``dest`` with retry/backoff (mirror throttles .dat)."""
    last: Exception | None = None
    for attempt in range(retries):
        try:
            req = urllib.request.Request(url, headers={"User-Agent": "lamina-validation/1.0"})
            with urllib.request.urlopen(req, timeout=timeout) as resp:
                data = resp.read()
            if not data:
                raise OSError("empty response")
            dest.write_bytes(data)
            return
        except (urllib.error.URLError, OSError, TimeoutError) as exc:
            last = exc
            time.sleep(min(2.0 * (attempt + 1), 10.0))
    raise OSError(f"download failed after {retries} attempts: {url}: {last}")


# ---------------------------------------------------------------------------
# Post-run HRV hook (runner does not serialize Recording.metadata, so the
# annotation-vs-Lamina HRV comparison is written to metrics_extra.csv here).
# ---------------------------------------------------------------------------

def write_hrv_extra_csv(
    dataset_key: str,
    results_dir: str | Path,
    seed: int = 0,
    smoke: bool = False,
    max_recordings: int | None = None,
    cache_dir: str | None = None,
) -> Path:
    """Compute per-recording HRV (bridge ``hrv`` op) from annotation beats and
    from Lamina-detected beats and append rows to ``metrics_extra.csv``.

    Rows: dataset, subject_id, recording_id, metric, value. Metrics:
    ``hrv_rmssd_ms_annotation``, ``hrv_rmssd_ms_lamina``,
    ``hrv_mean_nn_ms_annotation``, ``hrv_mean_nn_ms_lamina``,
    ``hrv_rmssd_abs_error_ms``, ``hrv_mean_nn_abs_error_ms``.
    Run AFTER ``python -m validation run --dataset <key>``; uses the identical
    deterministic recording selection.
    """
    from ..metrics.hrv import hrv_metrics
    from ..registry import get_dataset

    adapter = get_dataset(dataset_key)
    bridge = _lazy_bridge()
    rows: list[dict] = []
    rmssd_ref, rmssd_est, nn_ref, nn_est = [], [], [], []
    for rec in adapter.iter_recordings(
        seed=seed, smoke=smoke, max_recordings=max_recordings, cache_dir=cache_dir
    ):
        sig = next(iter(rec.signals.values()))
        fs = sig.sampling_rate
        n = len(sig.samples)
        ref_peaks = rec.references["ecg_peak_indices"]
        lamina_peaks = bridge.ecg_peaks(sig.samples, fs)["peaks"]
        h_ref = bridge.hrv(ref_peaks, n, fs)
        h_est = bridge.hrv(lamina_peaks, n, fs)
        vals = {
            "hrv_rmssd_ms_annotation": h_ref["rmssd_ms"],
            "hrv_rmssd_ms_lamina": h_est["rmssd_ms"],
            "hrv_mean_nn_ms_annotation": h_ref["mean_nn_ms"],
            "hrv_mean_nn_ms_lamina": h_est["mean_nn_ms"],
        }
        if h_ref["rmssd_ms"] is not None and h_est["rmssd_ms"] is not None:
            rmssd_ref.append(h_ref["rmssd_ms"])
            rmssd_est.append(h_est["rmssd_ms"])
            vals["hrv_rmssd_abs_error_ms"] = abs(
                h_est["rmssd_ms"] - h_ref["rmssd_ms"]
            )
        if h_ref["mean_nn_ms"] is not None and h_est["mean_nn_ms"] is not None:
            nn_ref.append(h_ref["mean_nn_ms"])
            nn_est.append(h_est["mean_nn_ms"])
            vals["hrv_mean_nn_abs_error_ms"] = abs(
                h_est["mean_nn_ms"] - h_ref["mean_nn_ms"]
            )
        for name, value in vals.items():
            rows.append({
                "dataset": adapter.key,
                "subject_id": rec.subject_id,
                "recording_id": rec.recording_id,
                "metric": name,
                "value": "" if value is None else value,
            })
    for label, m in (("hrv_rmssd_ms", hrv_metrics(rmssd_ref, rmssd_est)),
                     ("hrv_mean_nn_ms", hrv_metrics(nn_ref, nn_est))):
        for stat, value in m.items():
            rows.append({
                "dataset": adapter.key,
                "subject_id": "__dataset__",
                "recording_id": "__dataset__",
                "metric": f"{label}_comparison_{stat}",
                "value": "" if value is None else value,
            })
    out = Path(results_dir) / "metrics_extra.csv"
    out.parent.mkdir(parents=True, exist_ok=True)
    fieldnames = ["dataset", "subject_id", "recording_id", "metric", "value"]
    write_header = not out.exists()
    with open(out, "a", newline="") as fh:
        writer = csv.DictWriter(fh, fieldnames=fieldnames)
        if write_header:
            writer.writeheader()
        writer.writerows(rows)
    return out
