"""Autonomic / wearable dataset adapters (Stage 2: implemented).

Datasets
--------
- ``wesad``                — WESAD stress/affect dataset (chest RespiBAN @700 Hz
                             + wrist Empatica E4). Per-subject ``.pkl`` entries are
                             extracted from the remote 2.25 GB sciebo zip via HTTP
                             range requests (parallel chunks) so no full-archive
                             download is needed.
- ``autonomic-aging``      — PhysioNet Autonomic Aging (ECG @1000 Hz, wfdb).
- ``wearable-exam-stress`` — PhysioNet Wearable Exam Stress (Empatica E4 exports:
                             BVP @64 Hz, EDA @4 Hz, HR @1 Hz, IBI).
- ``big-ideas``            — BIG IDEAs glycemic wearable (public S3 mirror of the
                             PhysioNet files; very large CSVs — smallest
                             per-modality files only, strict timebox).

Ground-truth honesty
--------------------
None of these datasets provide beat-level or SCR-level annotations usable as
ground truth, so all peak/SCR/rate outputs are *structural* metrics (labelled
``structural_*`` by the runner). Two *consistency* checks are exposed through
the ``hr_bpm`` / ``hr_bpm_estimated`` reference contract:

- WESAD: chest-ECG HR vs wrist-BVP HR, both computed by Lamina (cross-device
  consistency, NOT independent validation).
- wearable-exam-stress: Empatica E4 ``HR.csv`` (proprietary onboard estimate,
  an imperfect reference) vs Lamina BVP-derived HR.
"""

from __future__ import annotations

import io
import json
import os
import pickle
import struct
import zlib
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
from typing import Iterator
from urllib.request import Request, urlopen

import numpy as np

from ..schema import Signal
from .base import (
    AccessReport,
    AccessStatus,
    DatasetAdapter,
    DatasetInfo,
    Recording,
)

# ---------------------------------------------------------------------------
# shared helpers
# ---------------------------------------------------------------------------

_WESAD_URL = "https://uni-siegen.sciebo.de/s/HGdUkoNlW1Ub0Gx/download"
_PHYSIONET_FILES = "https://physionet.org/files"
_BIGIDEAS_S3 = "https://physionet-open.s3.amazonaws.com"


def _default_cache_dir() -> Path:
    """Dataset cache root (local disk, outside the repo)."""
    return Path(os.environ.get("LAMINA_DATA_DIR", str(Path.home() / "data")))


def _http_get(url: str, timeout: float = 60.0, range_header: str | None = None) -> bytes:
    req = Request(url)
    if range_header:
        req.add_header("Range", range_header)
    with urlopen(req, timeout=timeout) as resp:
        return resp.read()


def _download(url: str, dest: Path, timeout: float = 120.0) -> Path:
    """Simple cached download (skips when the file already exists)."""
    dest.parent.mkdir(parents=True, exist_ok=True)
    if dest.exists() and dest.stat().st_size > 0:
        return dest
    data = _http_get(url, timeout=timeout)
    tmp = dest.with_suffix(dest.suffix + ".tmp")
    tmp.write_bytes(data)
    tmp.replace(dest)
    return dest


def _fetch_range(url: str, start: int, end: int, retries: int = 6,
                 timeout: float = 120.0) -> bytes:
    """Fetch ``bytes=start-end`` with retries; raises on persistent failure."""
    for attempt in range(retries):
        try:
            buf = _http_get(url, timeout=timeout,
                            range_header=f"bytes={start}-{end}")
            if len(buf) == end - start + 1:
                return buf
        except Exception:
            if attempt == retries - 1:
                raise
    raise RuntimeError(f"failed range fetch {start}-{end} from {url}")


def _download_chunked(url: str, dest: Path, threads: int = 16,
                      chunk: int = 2 * 1024 * 1024, min_size: int = 1 << 20,
                      timeout: float = 120.0) -> Path:
    """Parallel range-GET download (servers here throttle per-connection to
    ~60-90 KB/s; many parallel ranges give acceptable aggregate throughput).
    Falls back to a plain GET for small files or servers without ranges."""
    dest.parent.mkdir(parents=True, exist_ok=True)
    if dest.exists() and dest.stat().st_size > 0:
        return dest
    total: int | None = None
    try:
        req = Request(url, method="HEAD")
        with urlopen(req, timeout=30) as resp:
            total = int(resp.headers["Content-Length"])
    except Exception:
        total = None
    if total is None or total < min_size:
        return _download(url, dest, timeout=timeout)
    tmp = dest.with_suffix(dest.suffix + ".tmp")
    nchunks = (total + chunk - 1) // chunk
    parts: list[bytes | None] = [None] * nchunks
    with ThreadPoolExecutor(max_workers=threads) as ex:
        def get(i: int) -> None:
            s = i * chunk
            parts[i] = _fetch_range(url, s, min(s + chunk - 1, total - 1))
        list(ex.map(get, range(nchunks)))
    data = b"".join(p for p in parts if p is not None)
    if len(data) != total:
        raise RuntimeError(f"chunked download size mismatch for {url}")
    tmp.write_bytes(data)
    tmp.replace(dest)
    return dest


def _hr_series_from_peaks(peaks: np.ndarray, fs: float, t_grid: np.ndarray,
                          min_rr_sec: float = 0.3, max_rr_sec: float = 2.0
                          ) -> np.ndarray:
    """Interpolated instantaneous heart rate (bpm) on ``t_grid`` (seconds).

    RR intervals outside ``[min_rr_sec, max_rr_sec]`` are discarded as
    physiologically implausible before interpolation. Values outside the peak
    range are NaN (dropped later by the rate metrics).
    """
    peaks = np.asarray(peaks, dtype=float)
    if peaks.size < 3:
        return np.full(t_grid.shape, np.nan)
    rr = np.diff(peaks) / fs
    t = peaks[1:] / fs
    ok = (rr >= min_rr_sec) & (rr <= max_rr_sec)
    if ok.sum() < 2:
        return np.full(t_grid.shape, np.nan)
    hr = 60.0 / rr[ok]
    t = t[ok]
    out = np.interp(t_grid, t, hr, left=np.nan, right=np.nan)
    return out


def _lazy_bridge():
    """Module-level LaminaBridge singleton (only built when actually needed)."""
    global _BRIDGE
    if _BRIDGE is None:
        from ..bridge import LaminaBridge
        _BRIDGE = LaminaBridge()
    return _BRIDGE


_BRIDGE = None


def _select_subset(items: list, max_recordings: int | None, seed: int) -> list:
    if max_recordings is None or len(items) <= max_recordings:
        return items
    rng = np.random.RandomState(seed)
    idx = np.sort(rng.choice(len(items), size=max_recordings, replace=False))
    return [items[i] for i in idx]


# ---------------------------------------------------------------------------
# WESAD
# ---------------------------------------------------------------------------

#: Central-directory metadata for the per-subject ``.pkl`` entries inside the
#: remote WESAD zip (parsed from the archive EOCD on 2026-09-06; the sciebo
#: server supports HTTP range requests). ``name -> (local_header_offset,
#: compressed_size, uncompressed_size)``. Used to extract only the needed
#: subjects instead of downloading the full 2.25 GB archive. If the remote
#: archive changes, the offsets are re-parsed from the zip tail at runtime.
_WESAD_PKL_ENTRIES: dict[str, tuple[int, int, int]] = {
    "S2": (1006556876, 86983149, 975117737),
    "S3": (1164239331, 93260258, 1031507132),
    "S4": (1330834095, 91322270, 1034344388),
    "S5": (1494812652, 88469755, 993435543),
    "S6": (1654156750, 101826757, 1123292444),
}

_WESAD_FS_CHEST = 700.0
_WESAD_FS_BVP = 64.0
#: WESAD label semantics (0=undefined/transient, 1=baseline, 2=stress,
#: 3=amusement, 4=meditation).
_WESAD_CONDITIONS = {1: "baseline", 2: "stress", 3: "amusement"}
_WESAD_WINDOW_SEC = 240.0  # 4-minute contiguous window per condition
_WESAD_DEFAULT_SUBJECTS = ["S2", "S3", "S4", "S5", "S6"]


class WesadAdapter(DatasetAdapter):
    """WESAD adapter: condition-segmented windows for known-groups checks.

    One recording per (subject, condition in {baseline, stress, amusement}):
    the longest contiguous run of the condition label is located and the
    middle ``_WESAD_WINDOW_SEC`` seconds are extracted (4 min; full ~100 min
    chest recordings @700 Hz are impractical for a validation suite).
    """

    key = "wesad"

    def info(self) -> DatasetInfo:
        return DatasetInfo(
            key=self.key,
            name="WESAD (Wearable Stress and Affect Detection)",
            category="autonomic",
            modalities=["ecg", "eda", "bvp", "rsp"],
            source_url="https://ubicomp.eti.uni-siegen.de/home/datasets/icmi18/",
            license="MIT (dataset)",
            version="1.0",
            citation="Schmidt P et al. Introducing WESAD, a multimodal dataset for "
                     "wearable stress and affect detection. ICMI (2018).",
            lamina_ops=["ecg-peaks", "ppg-peaks", "eda-peaks", "rsp-cycles", "hrv"],
            notes="15 subjects (S2..S17; S1/S12 excluded by the dataset authors). "
                  "Chest RespiBAN @700 Hz (ECG/EDA/RESP) + wrist Empatica E4 "
                  "(BVP @64 Hz). No beat/SCR ground truth -> structural metrics "
                  "plus known-groups consistency checks (baseline vs stress). "
                  "HR reference pair is a Lamina-vs-Lamina cross-device "
                  "consistency check, not independent validation. "
                  "fs=700 Hz exceeds Lamina's typical design rates; anomalies "
                  "are logged as bug candidates.",
        )

    # -- access ---------------------------------------------------------

    def _cache(self, cache_dir: str | None) -> Path:
        return Path(cache_dir) if cache_dir else _default_cache_dir() / "wesad"

    def check_access(self, cache_dir: str | None = None) -> AccessReport:
        cache = self._cache(cache_dir)
        present = [s for s in _WESAD_DEFAULT_SUBJECTS
                   if (cache / f"{s}.pkl").exists()]
        if len(present) == len(_WESAD_DEFAULT_SUBJECTS):
            return AccessReport(
                status=AccessStatus.VALIDATED,
                reason="per-subject pickles cached locally",
                detail=f"cache={cache}; subjects={present}",
            )
        try:
            # HEAD probe only: a GET (even ranged) follows the sciebo 303
            # redirect and can end up streaming the whole 2.25 GB archive.
            req = Request(_WESAD_URL, method="HEAD")
            with urlopen(req, timeout=20) as resp:
                size = int(resp.headers.get("Content-Length", "0"))
            if size > 2_000_000_000:
                return AccessReport(
                    status=AccessStatus.VALIDATED,
                    reason="remote zip reachable via range GET; per-subject "
                           "entries are extracted on demand (slow server, "
                           "~4 min/subject with parallel ranges)",
                    detail=f"missing cached subjects: "
                           f"{[s for s in _WESAD_DEFAULT_SUBJECTS if s not in present]}",
                )
        except Exception as exc:
            return AccessReport(
                status=AccessStatus.INACCESSIBLE,
                reason=f"sciebo range probe failed: {exc}",
                detail=f"partial cache: {present}",
            )
        return AccessReport(status=AccessStatus.INACCESSIBLE,
                            reason="empty range probe response")

    # -- remote zip entry extraction -------------------------------------

    def _entry_offsets(self, subject: str) -> tuple[int, int, int]:
        """(local_header_offset, csize, usize); falls back to re-parsing the
        remote zip tail if the embedded table lacks the subject."""
        if subject in _WESAD_PKL_ENTRIES:
            return _WESAD_PKL_ENTRIES[subject]
        return self._parse_remote_central_directory()[subject]

    @staticmethod
    def _parse_remote_central_directory() -> dict[str, tuple[int, int, int]]:
        req = Request(_WESAD_URL, method="HEAD")
        with urlopen(req, timeout=60) as resp:
            total = int(resp.headers["Content-Length"])
        tail = _fetch_range(_WESAD_URL, total - 262144, total - 1)
        eocd_at = tail.rfind(b"PK\x05\x06")
        if eocd_at < 0:
            raise RuntimeError("WESAD zip EOCD not found in tail fetch")
        cd_size, cd_off = struct.unpack_from("<LH", tail, eocd_at + 12)
        tail_start = total - 262144
        if cd_off >= tail_start:
            cd = tail[cd_off - tail_start: cd_off - tail_start + cd_size]
        else:
            cd = _fetch_range(_WESAD_URL, cd_off, cd_off + cd_size - 1)
        out: dict[str, tuple[int, int, int]] = {}
        pos = 0
        while pos < len(cd) - 46:
            if cd[pos:pos + 4] != b"PK\x01\x02":
                break
            f = struct.unpack("<4s6H3L5H2L", cd[pos:pos + 46])
            _, _, _, _, _, _, _, csize, usize, fnlen, extralen, commentlen, \
                _, _, lho = f
            name = cd[pos + 46:pos + 46 + fnlen].decode("utf-8", "replace")
            base = name.rstrip("/").split("/")[-1]
            if base.endswith(".pkl"):
                out[base[:-4]] = (lho, csize, usize)
            pos += 46 + fnlen + extralen + commentlen
        return out

    def _ensure_subject_pkl(self, subject: str, cache: Path,
                            threads: int = 24, chunk: int = 4 * 1024 * 1024) -> Path:
        dest = cache / f"{subject}.pkl"
        lh_off, csize, usize = self._entry_offsets(subject)
        if dest.exists() and dest.stat().st_size == usize:
            return dest
        cache.mkdir(parents=True, exist_ok=True)
        lh = _fetch_range(_WESAD_URL, lh_off, lh_off + 29)
        sig, _, _, _, _, _, _, _, _, fnlen, extralen = struct.unpack("<4s5H3L2H", lh)
        if sig != b"PK\x03\x04":
            raise RuntimeError(f"bad zip local header for {subject}")
        data_off = lh_off + 30 + fnlen + extralen
        data_end = data_off + csize - 1
        nchunks = (csize + chunk - 1) // chunk
        parts: list[bytes | None] = [None] * nchunks
        with ThreadPoolExecutor(max_workers=threads) as ex:
            def get(i: int) -> None:
                s = data_off + i * chunk
                parts[i] = _fetch_range(_WESAD_URL, s, min(s + chunk - 1, data_end))
            list(ex.map(get, range(nchunks)))
        # Stream-inflate to disk: the uncompressed pickle is ~1 GB and the
        # runtime memory budget is 4 GB, so never materialize it in RAM.
        d = zlib.decompressobj(-15)
        tmp = dest.with_suffix(".pkl.tmp")
        written = 0
        with open(tmp, "wb") as fh:
            for p in parts:
                if p is None:
                    raise RuntimeError(f"{subject}: missing chunk")
                buf = d.decompress(p)
                fh.write(buf)
                written += len(buf)
            buf = d.flush()
            fh.write(buf)
            written += len(buf)
        if written != usize:
            tmp.unlink(missing_ok=True)
            raise RuntimeError(f"{subject}: uncompressed size mismatch")
        tmp.replace(dest)
        return dest

    # -- recording iteration ----------------------------------------------

    def iter_recordings(
        self,
        subjects: list[str] | None = None,
        recordings: list[str] | None = None,
        max_recordings: int | None = None,
        seed: int = 0,
        smoke: bool = False,
        cache_dir: str | None = None,
    ) -> Iterator[Recording]:
        cache = self._cache(cache_dir)
        subs = list(subjects) if subjects else list(_WESAD_DEFAULT_SUBJECTS)
        if smoke:
            subs = subs[:1]
        units = [(s, c) for s in subs for c in _WESAD_CONDITIONS]
        if recordings:
            wanted = set(recordings)
            units = [u for u in units if f"{u[0]}_{_WESAD_CONDITIONS[u[1]]}" in wanted]
        units = _select_subset(units, max_recordings, seed)
        if smoke:
            units = units[:1]
        window = 60.0 if smoke else _WESAD_WINDOW_SEC
        for subject, cond in units:
            pkl_path = self._ensure_subject_pkl(subject, cache)
            with open(pkl_path, "rb") as fh:
                data = pickle.load(fh, encoding="latin1")
            try:
                rec = self._build_recording(subject, cond, data, window)
            finally:
                del data
            if rec is not None:
                yield rec

    def _build_recording(self, subject: str, cond: int, data: dict,
                         window_sec: float) -> Recording | None:
        label = np.asarray(data["label"]).ravel()
        chest = data["signal"]["chest"]
        wrist = data["signal"]["wrist"]
        run = _longest_run(label, cond)
        if run is None or (run[1] - run[0]) < window_sec * _WESAD_FS_CHEST * 0.5:
            return None
        # middle of the longest contiguous run
        mid = (run[0] + run[1]) // 2
        half = int(window_sec * _WESAD_FS_CHEST) // 2
        i0 = max(run[0], mid - half)
        i1 = min(run[1], i0 + int(window_sec * _WESAD_FS_CHEST))
        t0, t1 = i0 / _WESAD_FS_CHEST, i1 / _WESAD_FS_CHEST
        cname = _WESAD_CONDITIONS[cond]
        rid = f"{subject}_{cname}"

        def chest_sig(key: str, modality: str, units: str) -> Signal:
            return Signal(
                samples=np.asarray(chest[key], dtype=float).ravel()[i0:i1],
                sampling_rate=_WESAD_FS_CHEST, modality=modality, units=units,
                subject_id=subject, recording_id=rid, channel=key,
            )

        j0, j1 = int(t0 * _WESAD_FS_BVP), int(t1 * _WESAD_FS_BVP)
        bvp = np.asarray(wrist["BVP"], dtype=float).ravel()[j0:j1]
        signals = {
            "chest_ecg": chest_sig("ECG", "ecg", "mV"),
            "chest_eda": chest_sig("EDA", "eda", "uS"),
            "chest_rsp": chest_sig("Resp", "rsp", "a.u."),
            "wrist_bvp": Signal(samples=bvp, sampling_rate=_WESAD_FS_BVP,
                                modality="bvp", units="a.u.", subject_id=subject,
                                recording_id=rid, channel="BVP"),
        }
        references: dict = {}
        try:
            references.update(self._cross_device_hr(signals, window_sec))
        except Exception as exc:  # never let the consistency check break the run
            references["hr_consistency_error"] = f"{type(exc).__name__}: {exc}"
        return Recording(
            recording_id=rid, subject_id=subject, signals=signals,
            references=references,
            metadata={
                "condition": cname, "label_code": cond,
                "window_start_sec": t0, "window_end_sec": t1,
                "hr_reference_note": (
                    "hr_bpm = Lamina HR from chest ECG peaks; hr_bpm_estimated = "
                    "Lamina HR from wrist BVP peaks; CROSS-DEVICE CONSISTENCY "
                    "check (both estimated by the implementation under test), "
                    "NOT independent ground-truth validation."),
            },
        )

    @staticmethod
    def _cross_device_hr(signals: dict[str, Signal], window_sec: float) -> dict:
        """Lamina-vs-Lamina HR series (chest ECG vs wrist BVP), 1 Hz grid."""
        bridge = _lazy_bridge()
        ecg = signals["chest_ecg"]
        bvp = signals["wrist_bvp"]
        ecg_peaks = np.asarray(
            bridge.ecg_peaks(ecg.samples, ecg.sampling_rate)["peaks"], dtype=float)
        bvp_peaks = np.asarray(
            bridge.ppg_peaks(bvp.samples, bvp.sampling_rate)["peaks"], dtype=float)
        grid = np.arange(10.0, window_sec - 10.0, 1.0)
        hr_ecg = _hr_series_from_peaks(ecg_peaks, ecg.sampling_rate, grid)
        hr_bvp = _hr_series_from_peaks(bvp_peaks, bvp.sampling_rate, grid)
        ok = np.isfinite(hr_ecg) & np.isfinite(hr_bvp)
        return {"hr_bpm": hr_ecg[ok], "hr_bpm_estimated": hr_bvp[ok]}


def _longest_run(labels: np.ndarray, value: int) -> tuple[int, int] | None:
    """(start, end) sample span of the longest contiguous run of ``value``."""
    mask = labels == value
    if not mask.any():
        return None
    edges = np.diff(mask.astype(np.int8))
    starts = np.concatenate(([0] if mask[0] else [], np.where(edges == 1)[0] + 1))
    ends = np.concatenate((np.where(edges == -1)[0] + 1, [len(mask)] if mask[-1] else []))
    lengths = ends - starts
    k = int(np.argmax(lengths))
    return int(starts[k]), int(ends[k])


# ---------------------------------------------------------------------------
# Autonomic Aging (PhysioNet)
# ---------------------------------------------------------------------------

_AA_PROJECT = "autonomic-aging-cardiovascular"
_AA_VERSION = "1.0.0"
#: PhysioNet-documented age-group codes (see project page):
#: 1=18-19, 2=20-24, ..., 5=35-39, 6=40-44, ..., 11=65-69, 12=70-74, ..., 15=85-92.
_AA_STRATA = {
    "young_18_39": (1, 2, 3, 4, 5),
    "middle_40_69": (6, 7, 8, 9, 10, 11),
    "old_70_plus": (12, 13, 14, 15),
}
_AA_PER_STRATUM = 4
_AA_MAX_SEC = 600.0  # cap: first 10 minutes per record


class AutonomicAgingAdapter(DatasetAdapter):
    """Autonomic Aging adapter: 12 records stratified across 3 age strata.

    Deterministic subset (seed 42): 4 records per age stratum
    (18-39 / 40-69 / 70+), each capped at the first 10 minutes.
    No beat annotations are provided -> structural ECG metrics only; HRV
    (bridge ``hrv`` op) and the age trend are computed as extra analyses.
    """

    key = "autonomic-aging"
    _seed = 42

    def info(self) -> DatasetInfo:
        return DatasetInfo(
            key=self.key,
            name="Autonomic Aging: A dataset to quantify changes of cardiovascular "
                 "autonomic function during healthy aging",
            category="autonomic",
            modalities=["ecg"],
            source_url="https://physionet.org/content/autonomic-aging-cardiovascular/",
            license="ODC-BY 1.0",
            version="1.0.0",
            citation="Schumann A, Bär KJ. Autonomic Aging. PhysioNet (2022).",
            lamina_ops=["ecg-peaks", "hrv"],
            notes="1121 records, ECG1/ECG2/NIBP @1000 Hz. Age-group codes per "
                  "PhysioNet documentation (1=18-19y .. 15=85-92y). No beat "
                  "annotations -> structural metrics; RMSSD-vs-age-group trend "
                  "is a population-level consistency check, not ground truth.",
        )

    def _cache(self, cache_dir: str | None) -> Path:
        return Path(cache_dir) if cache_dir else _default_cache_dir() / "autonomic-aging"

    def _subject_info(self, cache: Path) -> "list[dict]":
        import csv as _csv
        path = _download(
            f"{_PHYSIONET_FILES}/{_AA_PROJECT}/{_AA_VERSION}/subject-info.csv",
            cache / "subject-info.csv")
        rows = []
        with open(path, newline="") as fh:
            for row in _csv.DictReader(fh):
                try:
                    ag = int(row["Age_group"])
                except (TypeError, ValueError):
                    continue  # NaN age group
                rows.append({
                    "id": row["ID"], "age_group": ag,
                    "sex": row.get("Sex", ""), "bmi": row.get("BMI", ""),
                    "length_min": row.get("Length", ""),
                })
        return rows

    def _stratum(self, age_group: int) -> str | None:
        for name, codes in _AA_STRATA.items():
            if age_group in codes:
                return name
        return None

    def selected_records(self, cache: Path) -> list[dict]:
        """Deterministic stratified subset: 4 per stratum, seed 42."""
        rows = [r for r in self._subject_info(cache)
                if self._stratum(r["age_group"]) is not None]
        rng = np.random.RandomState(self._seed)
        out = []
        for stratum in _AA_STRATA:
            pool = sorted(r["id"] for r in rows if self._stratum(r["age_group"]) == stratum)
            pick = sorted(rng.choice(len(pool), size=min(_AA_PER_STRATUM, len(pool)),
                                     replace=False).tolist())
            for i in pick:
                r = next(r for r in rows if r["id"] == pool[i])
                out.append({**r, "stratum": stratum})
        return sorted(out, key=lambda r: r["id"])

    def check_access(self, cache_dir: str | None = None) -> AccessReport:
        cache = self._cache(cache_dir)
        try:
            rows = self._subject_info(cache)
            n_sel = len(self.selected_records(cache))
            if rows and n_sel == 3 * _AA_PER_STRATUM:
                return AccessReport(
                    status=AccessStatus.VALIDATED,
                    reason="subject-info.csv reachable; records downloadable via "
                           "direct HTTPS (.hea/.dat)",
                    detail=f"{len(rows)} subjects with age group; "
                           f"selected {n_sel} records",
                )
            return AccessReport(status=AccessStatus.INACCESSIBLE,
                                reason="subject-info.csv parsed but empty/incomplete")
        except Exception as exc:
            return AccessReport(status=AccessStatus.INACCESSIBLE,
                                reason=f"PhysioNet fetch failed: {exc}")

    def _ensure_record(self, rec_id: str, cache: Path) -> Path:
        base = cache / "records" / rec_id
        for ext in (".hea", ".dat"):
            _download_chunked(
                f"{_PHYSIONET_FILES}/{_AA_PROJECT}/{_AA_VERSION}/{rec_id}{ext}",
                Path(str(base) + ext))
        return base

    def iter_recordings(
        self,
        subjects: list[str] | None = None,
        recordings: list[str] | None = None,
        max_recordings: int | None = None,
        seed: int = 0,
        smoke: bool = False,
        cache_dir: str | None = None,
    ) -> Iterator[Recording]:
        import wfdb  # deferred: heavy dependency, only needed at run time

        cache = self._cache(cache_dir)
        rows = self.selected_records(cache)
        if subjects:
            wanted = set(subjects)
            rows = [r for r in rows if r["id"] in wanted]
        if recordings:
            wanted_r = set(recordings)
            rows = [r for r in rows if r["id"] in wanted_r]
        rows = _select_subset(rows, max_recordings, seed)
        if smoke:
            rows = rows[:1]
        for r in rows:
            base = self._ensure_record(r["id"], cache)
            header = wfdb.rdheader(str(base))
            n = min(header.sig_len, int(_AA_MAX_SEC * header.fs))
            rec = wfdb.rdrecord(str(base), sampfrom=0, sampto=n)
            # Channel names vary by recording device: device 1 records expose
            # ECG1/ECG2/NIBP, device 0 records expose ECG/NIBP.
            ecg_chans = [i for i, nm in enumerate(rec.sig_name)
                         if nm.upper().startswith("ECG")]
            if not ecg_chans:
                raise RuntimeError(f"record {r['id']}: no ECG channel in "
                                   f"{rec.sig_name}")
            signals = {}
            for k, idx in enumerate(ecg_chans):
                ch = rec.sig_name[idx]
                signals[f"ecg{k + 1}"] = Signal(
                    samples=rec.p_signal[:, idx].astype(float),
                    sampling_rate=float(rec.fs), modality="ecg",
                    units=rec.units[idx], subject_id=r["id"],
                    recording_id=r["id"], channel=ch)
            yield Recording(
                recording_id=r["id"], subject_id=r["id"], signals=signals,
                references={},  # no beat annotations -> structural metrics only
                metadata={
                    "age_group_code": r["age_group"], "age_stratum": r["stratum"],
                    "sex": r["sex"], "bmi": r["bmi"],
                    "duration_capped_sec": min(_AA_MAX_SEC, header.sig_len / header.fs),
                    "note": "structural ECG metrics; RMSSD age trend computed in "
                            "extra analyses (population-trend consistency check).",
                },
            )


# ---------------------------------------------------------------------------
# Wearable Exam Stress (PhysioNet, Empatica E4 exports)
# ---------------------------------------------------------------------------

_WES_PROJECT = "wearable-exam-stress"
_WES_VERSION = "1.0.0"
_WES_STUDENTS = ["S1", "S2", "S3", "S4", "S5"]
_WES_SESSIONS = ["midterm_1", "midterm_2", "Final"]


def _parse_e4_csv(path: Path) -> tuple[float, float, np.ndarray]:
    """Parse an Empatica E4 export CSV.

    Row 1 = unix epoch start time (seconds), row 2 = sampling rate (Hz),
    remaining rows = sample values (first column used). Returns
    ``(epoch_sec, fs, samples)``.
    """
    with open(path) as fh:
        epoch = float(fh.readline().split(",")[0].strip())
        fs = float(fh.readline().split(",")[0].strip())
        data = np.loadtxt(fh, delimiter=",", ndmin=1)
    if data.ndim == 2:
        data = data[:, 0]
    return epoch, fs, np.asarray(data, dtype=float)


def _parse_e4_ibi(path: Path) -> tuple[float, np.ndarray, np.ndarray]:
    """Parse IBI.csv: row 1 = ``epoch, IBI``; then ``t_sec, ibi_sec`` rows."""
    with open(path) as fh:
        epoch = float(fh.readline().split(",")[0].strip())
        rows = [ln.split(",") for ln in fh if ln.strip()]
    t = np.array([float(r[0]) for r in rows])
    ibi = np.array([float(r[1]) for r in rows])
    return epoch, t, ibi


class WearableExamStressAdapter(DatasetAdapter):
    """Wearable Exam Stress adapter: S1..S5 x {midterm_1, midterm_2, Final}.

    Signals: BVP @64 Hz + EDA @4 Hz (Empatica E4 wrist). References:
    ``hr_bpm`` = E4 ``HR.csv`` (proprietary onboard estimate — an imperfect
    reference, documented as such); ``hr_bpm_estimated`` = Lamina BVP-derived
    HR interpolated onto the HR.csv 1 Hz grid (aligned by absolute time using
    the per-file epoch header rows).
    """

    key = "wearable-exam-stress"

    def info(self) -> DatasetInfo:
        return DatasetInfo(
            key=self.key,
            name="Wearable Exam Stress Dataset",
            category="autonomic",
            modalities=["bvp", "eda"],
            source_url="https://physionet.org/content/wearable-exam-stress/",
            license="ODC-BY 1.0",
            version="1.0.0",
            citation="Amin MR et al. Exam stress measurement using wearable "
                     "sensors. PhysioNet (2021).",
            lamina_ops=["ppg-peaks", "eda-clean", "eda-decompose", "eda-peaks"],
            notes="10 students (S1..S10), 3 exam sessions each, Empatica E4 "
                  "exports. E4 CSV row1 = epoch, row2 = fs (parsed explicitly). "
                  "HR.csv is the proprietary onboard estimate used as reference "
                  "(imperfect). EDA @4 Hz is below typical SCR-analysis rates; "
                  "Lamina behavior is recorded honestly.",
        )

    def _cache(self, cache_dir: str | None) -> Path:
        return (Path(cache_dir) if cache_dir
                else _default_cache_dir() / "wearable-exam-stress")

    def _session_dir(self, student: str, session: str, cache: Path) -> Path:
        dest = cache / student / session
        for fname in ("BVP.csv", "EDA.csv", "HR.csv", "IBI.csv"):
            _download_chunked(f"{_PHYSIONET_FILES}/{_WES_PROJECT}/{_WES_VERSION}/"
                              f"data/{student}/{session}/{fname}", dest / fname)
        return dest

    def check_access(self, cache_dir: str | None = None) -> AccessReport:
        cache = self._cache(cache_dir)
        try:
            dest = self._session_dir("S1", "Final", cache)
            epoch, fs, bvp = _parse_e4_csv(dest / "BVP.csv")
            if bvp.size > 100 and abs(fs - 64.0) < 1e-6:
                return AccessReport(
                    status=AccessStatus.VALIDATED,
                    reason="S1/Final E4 CSVs downloaded and parsed "
                           f"(BVP fs={fs}, epoch={epoch})",
                    detail=f"cache={cache}",
                )
            return AccessReport(status=AccessStatus.FAILED,
                                reason=f"unexpected E4 parse: fs={fs}, n={bvp.size}")
        except Exception as exc:
            return AccessReport(status=AccessStatus.INACCESSIBLE,
                                reason=f"PhysioNet fetch/parse failed: {exc}")

    def iter_recordings(
        self,
        subjects: list[str] | None = None,
        recordings: list[str] | None = None,
        max_recordings: int | None = None,
        seed: int = 0,
        smoke: bool = False,
        cache_dir: str | None = None,
    ) -> Iterator[Recording]:
        cache = self._cache(cache_dir)
        students = list(subjects) if subjects else list(_WES_STUDENTS)
        if smoke:
            students = students[:1]
        units = [(s, sess) for s in students for sess in _WES_SESSIONS]
        if recordings:
            wanted = set(recordings)
            units = [u for u in units if f"{u[0]}_{u[1]}" in wanted]
        units = _select_subset(units, max_recordings, seed)
        if smoke:
            units = units[:1]
        for student, session in units:
            rid = f"{student}_{session}"
            dest = self._session_dir(student, session, cache)
            bvp_epoch, bvp_fs, bvp = _parse_e4_csv(dest / "BVP.csv")
            eda_epoch, eda_fs, eda = _parse_e4_csv(dest / "EDA.csv")
            hr_epoch, hr_fs, hr = _parse_e4_csv(dest / "HR.csv")
            if smoke:
                bvp = bvp[: int(120 * bvp_fs)]
                eda = eda[: int(120 * eda_fs)]
                hr = hr[:120]
            signals = {
                "wrist_bvp": Signal(
                    samples=bvp, sampling_rate=bvp_fs, modality="bvp",
                    units="a.u.", subject_id=student, recording_id=rid,
                    channel="BVP",
                    metadata={"epoch_sec": bvp_epoch}),
            }
            references: dict = {}
            try:
                references.update(
                    self._hr_vs_e4(bvp, bvp_fs, bvp_epoch, hr, hr_fs, hr_epoch))
            except Exception as exc:
                references["hr_consistency_error"] = f"{type(exc).__name__}: {exc}"
            yield Recording(
                recording_id=rid, subject_id=student, signals=signals,
                references=references,
                metadata={
                    "session": session,
                    "hr_reference_note": (
                        "hr_bpm = Empatica E4 HR.csv proprietary onboard estimate "
                        "(imperfect reference, documented as such); "
                        "hr_bpm_estimated = Lamina BVP-peak HR interpolated onto "
                        "the HR.csv grid; agreement metrics are consistency "
                        "evidence, not ground-truth accuracy."),
                },
            )
            # EDA @4 Hz is yielded as a SEPARATE recording: Lamina's eda
            # pipeline has a hardcoded 5 Hz lowpass cutoff (fails for
            # fs <= 10 Hz — see bug-candidates.md BUG-A03), and one failing
            # modality would otherwise fail the whole recording in the runner,
            # hiding the BVP/HR results. The failure is therefore recorded
            # explicitly and honestly as a per-recording failure.
            yield Recording(
                recording_id=f"{rid}_eda", subject_id=student,
                signals={
                    "wrist_eda": Signal(
                        samples=eda, sampling_rate=eda_fs, modality="eda",
                        units="uS", subject_id=student,
                        recording_id=f"{rid}_eda", channel="EDA",
                        metadata={"epoch_sec": eda_epoch}),
                },
                references={},
                metadata={
                    "session": session,
                    "note": "structural EDA @4 Hz; expected to fail in Lamina "
                            "eda-clean (hardcoded 5 Hz cutoff > Nyquist at "
                            "fs=4 Hz) — failure is the documented finding.",
                },
            )

    @staticmethod
    def _hr_vs_e4(bvp: np.ndarray, bvp_fs: float, bvp_epoch: float,
                  hr: np.ndarray, hr_fs: float, hr_epoch: float) -> dict:
        bridge = _lazy_bridge()
        peaks = np.asarray(bridge.ppg_peaks(bvp, bvp_fs)["peaks"], dtype=float)
        # Absolute-time grid of the E4 HR.csv rows.
        t_hr = hr_epoch + np.arange(hr.size) / hr_fs
        # Lamina HR at those times (BVP timeline starts at bvp_epoch).
        t_rel = t_hr - bvp_epoch
        hr_est = _hr_series_from_peaks(peaks, bvp_fs, t_rel)
        ok = (np.isfinite(hr_est) & np.isfinite(hr)
              & (t_rel >= 0) & (t_rel <= bvp.size / bvp_fs))
        return {"hr_bpm": np.asarray(hr, dtype=float)[ok],
                "hr_bpm_estimated": hr_est[ok]}


# ---------------------------------------------------------------------------
# BIG IDEAs glycemic wearable (PhysioNet open S3 mirror)
# ---------------------------------------------------------------------------

_BI_PREFIX = "big-ideas-glycemic-wearable/1.0.0"
_BI_MAX_BYTES = 200 * 1024 * 1024  # strict download budget
_BI_EDA_WINDOW_SEC = 2 * 3600.0    # 2 h window @4 Hz = 28.8k samples
_BI_FS_EDA = 4.0


class BigIdeasAdapter(DatasetAdapter):
    """BIG IDEAs adapter (strict timebox; structural only).

    The public S3 mirror hosts per-subject Empatica-style CSVs, but the BVP
    files are all > 900 MB and ACC > 580 MB — outside the 200 MB budget.
    Strategy: list keys, pick the smallest EDA files (60-65 MB each), window
    to the first 2 h, and run structural EDA validation only. Glucose (Dexcom)
    is out of Lamina scope; Demographics.csv is actually XLSX and is skipped.
    """

    key = "big-ideas"

    def info(self) -> DatasetInfo:
        return DatasetInfo(
            key=self.key,
            name="BIG IDEAs Wearable/Glycemic Dataset",
            category="autonomic",
            modalities=["eda"],
            source_url="https://physionet.org/content/big-ideas-glycemic-wearable/",
            license="PhysioNet Credentialed Health Data License (public S3 mirror "
                    "of the per-subject CSVs used here)",
            version="1.0.0",
            citation="Bent B et al. The BIG IDEAs Glycemic Wearable dataset. "
                     "PhysioNet (2024).",
            lamina_ops=["eda-clean", "eda-decompose", "eda-peaks"],
            notes="BVP/ACC files exceed the 200 MB download budget (smallest "
                  "BVP ~919 MB) -> BVP modality skipped (partially_validated). "
                  "EDA only, first-2h window, structural metrics. Glucose out "
                  "of Lamina scope.",
        )

    def _cache(self, cache_dir: str | None) -> Path:
        return Path(cache_dir) if cache_dir else _default_cache_dir() / "big-ideas"

    @staticmethod
    def _list_keys() -> list[tuple[str, int]]:
        import re
        keys: list[tuple[str, int]] = []
        token: str | None = None
        while True:
            url = (f"{_BIGIDEAS_S3}/?list-type=2&prefix={_BI_PREFIX}/"
                   f"&max-keys=1000")
            if token:
                url += f"&continuation-token={token}"
            xml = _http_get(url, timeout=60).decode("utf-8", "replace")
            keys.extend((k, int(s)) for k, s in re.findall(
                r"<Key>([^<]+)</Key>.*?<Size>(\d+)</Size>", xml))
            trunc = re.search(r"<IsTruncated>(\w+)</IsTruncated>", xml)
            tok = re.search(r"<NextContinuationToken>([^<]+)</NextContinuationToken>",
                            xml)
            if not (trunc and trunc.group(1) == "true" and tok):
                break
            token = tok.group(1)
        return keys

    def _plan(self) -> list[tuple[str, int]]:
        """Smallest EDA CSVs within the byte budget."""
        keys = [(k, s) for k, s in self._list_keys()
                if k.endswith(".csv") and "/EDA_" in k]
        keys.sort(key=lambda x: x[1])
        plan, total = [], 0
        for k, s in keys:
            if total + s > _BI_MAX_BYTES:
                break
            plan.append((k, s))
            total += s
        return plan[:2]  # at most 2 subjects, deterministic (smallest first)

    def check_access(self, cache_dir: str | None = None) -> AccessReport:
        try:
            plan = self._plan()
        except Exception as exc:
            return AccessReport(status=AccessStatus.INACCESSIBLE,
                                reason=f"S3 listing failed: {exc}")
        if not plan:
            return AccessReport(
                status=AccessStatus.PARTIALLY_VALIDATED,
                reason="S3 listing ok but no EDA CSV within the 200 MB budget")
        return AccessReport(
            status=AccessStatus.PARTIALLY_VALIDATED,
            reason="structural EDA-only validation (BVP/ACC exceed the 200 MB "
                   "download budget)",
            detail="planned: " + ", ".join(f"{k} ({s/1e6:.0f}MB)" for k, s in plan))

    def iter_recordings(
        self,
        subjects: list[str] | None = None,
        recordings: list[str] | None = None,
        max_recordings: int | None = None,
        seed: int = 0,
        smoke: bool = False,
        cache_dir: str | None = None,
    ) -> Iterator[Recording]:
        cache = self._cache(cache_dir)
        plan = self._plan()
        if subjects:
            plan = [p for p in plan
                    if p[0].split("/")[-2].lstrip("0") in {s.lstrip("0") for s in subjects}
                    or p[0].split("/")[-2] in set(subjects)]
        plan = _select_subset(plan, max_recordings, seed)
        if smoke:
            plan = plan[:1]
        for key, _size in plan:
            subject = key.split("/")[-2]
            fname = key.split("/")[-1]
            dest = _download_chunked(f"{_BIGIDEAS_S3}/{key}",
                                     cache / subject / fname)
            # CSV: header "datetime, eda"; timestamped rows @4 Hz.
            import pandas as pd
            n_max = int(_BI_EDA_WINDOW_SEC * _BI_FS_EDA)
            df = pd.read_csv(dest, skipinitialspace=True, nrows=n_max)
            valcol = [c for c in df.columns if c != "datetime"][0]
            samples = pd.to_numeric(df[valcol], errors="coerce").to_numpy(float)
            samples = samples[np.isfinite(samples)]
            rid = f"{subject}_eda"
            yield Recording(
                recording_id=rid, subject_id=subject,
                signals={
                    "wrist_eda": Signal(
                        samples=samples, sampling_rate=_BI_FS_EDA, modality="eda",
                        units="uS", subject_id=subject, recording_id=rid,
                        channel="EDA"),
                },
                references={},
                metadata={
                    "source_key": key,
                    "window_sec": samples.size / _BI_FS_EDA,
                    "note": "structural EDA metrics only; first-2h window of the "
                            "full multi-day file; no ground truth.",
                },
            )
