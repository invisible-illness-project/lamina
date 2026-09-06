"""PPG / cardiovascular dataset adapters.

Stage 2 implementations (PPG-group validation agent):

- ``BidmcAdapter``           — BIDMC PPG and Respiration Dataset (implemented).
- ``WristPpgExerciseAdapter``— Wrist PPG During Exercise (implemented).
- ``PulseDbAdapter``         — PulseDB (stub: hosts unreachable/credentialed
  from the validation environment).
- ``MimicIiiWaveformAdapter``/``MimicIiiExtPpgAdapter`` (stubs: PhysioNet
  credentialed access required).

Data acquisition notes
----------------------
BIDMC: physionet.org is unreachable (HTTP 403) from the validation
environment, so records are fetched from the verified public mirror
``https://archive.physionet.org/physiobank/database/bidmc/``.  The mirror
serves the original WFDB layout (``bidmcXX.dat`` / ``.hea`` / ``.breath``),
NOT the ``bidmc_XX_Signals.csv`` / ``_Breaths.csv`` triplet of the newer
physionet.org content layout.  Breath annotations are read with
``wfdb.rdann(..., "breath")``; ``aux_note`` identifies the annotator
(``ann1`` / ``ann2``) and ``sample`` is a 0-based sample index at the signal
sampling rate (125 Hz) — verified empirically to coincide with inspiratory
peaks (maxima of the band-passed RESP signal; median offset 0.0 s on
bidmc01), matching Lamina's ``RespirationCycle.inspiration_index`` semantics
(maximum expansion).  ``ann1`` is used as the runner reference
(``rsp_peak_indices``); ``ann2`` agreement is reported in metrics_extra.

WRIST: downloaded via the ``wfdb`` client (``pn_dir="wrist"``), cached
locally.  The dataset ships chest-ECG beat annotations (``.atr``), used both
as ``ecg_peak_indices`` (Lamina ``ecg-peaks`` validated directly @150 ms)
and as an annotation-derived windowed HR reference (``hr_bpm``), compared
against HR from Lamina ``ppg-peaks`` on the wrist PPG
(``hr_bpm_estimated``).  An annotation-HR vs Lamina-ecg-peaks-HR consistency
cross-check is written to metrics_extra (the ECG-agent separately validates
Lamina ECG detection on MIT-BIH).

Extra per-recording metrics computed by these adapters (cross-channel
consistency, inter-annotator agreement, per-activity HR bias) are appended to
``validation/results/ppg/metrics_extra.csv`` (overridable via the
``LAMINA_PPG_METRICS_EXTRA`` env var) with columns:
``dataset,recording_id,metric,value,units,notes``.
"""

from __future__ import annotations

import csv
import os
import random
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
    StubDatasetAdapter,
)
from ..schema import Signal

# ---------------------------------------------------------------------------
# Shared helpers
# ---------------------------------------------------------------------------

_BIDMC_MIRROR = "https://archive.physionet.org/physiobank/database/bidmc"
#: Deterministic 12-record subset (task specification): bidmc01..bidmc12.
_BIDMC_RECORDS = [f"bidmc{i:02d}" for i in range(1, 13)]
_BIDMC_EXT = (".dat", ".hea", ".breath")

_WRIST_PN_DIR = "wrist"


def _repo_root() -> Path:
    return Path(__file__).resolve().parents[3]


def _metrics_extra_path() -> Path:
    env = os.environ.get("LAMINA_PPG_METRICS_EXTRA")
    if env:
        return Path(env)
    return _repo_root() / "validation" / "results" / "ppg" / "metrics_extra.csv"


def _append_metrics_extra(dataset: str, rows: list[dict]) -> None:
    """Append rows to metrics_extra.csv, replacing prior rows for *dataset*.

    Re-writing rows for the same dataset keeps re-runs idempotent while
    preserving rows of other datasets written in the same results dir.
    """
    path = _metrics_extra_path()
    path.parent.mkdir(parents=True, exist_ok=True)
    fieldnames = ["dataset", "recording_id", "metric", "value", "units", "notes"]
    existing: list[dict] = []
    if path.exists():
        with open(path, newline="") as fh:
            existing = [r for r in csv.DictReader(fh) if r.get("dataset") != dataset]
    with open(path, "w", newline="") as fh:
        writer = csv.DictWriter(fh, fieldnames=fieldnames)
        writer.writeheader()
        writer.writerows(existing)
        writer.writerows(rows)


def _windowed_hr_from_peaks(peaks: np.ndarray, fs: float, n_samples: int,
                            window_sec: float, step_sec: float
                            ) -> tuple[np.ndarray, np.ndarray]:
    """Interval-based HR (bpm) over sliding windows.

    For each pair of consecutive peaks the instantaneous rate 60/IBI is
    assigned to the IBI midpoint; a window's HR is the mean of the
    instantaneous rates whose midpoint falls inside it.  Windows with no
    complete IBI yield NaN.  Returns (window_start_times_sec, hr_bpm).
    """
    peaks = np.asarray(peaks, dtype=float)
    times = peaks / fs
    dur = n_samples / fs
    starts = np.arange(0.0, max(dur - window_sec, 0.0) + 1e-9, step_sec)
    hr = np.full(len(starts), np.nan)
    if len(peaks) >= 2:
        ibi = np.diff(times)
        inst = 60.0 / ibi
        mid = (times[:-1] + times[1:]) / 2.0
        for k, t0 in enumerate(starts):
            m = (mid >= t0) & (mid < t0 + window_sec)
            if m.any():
                hr[k] = float(np.mean(inst[m]))
    return starts, hr


def _match_f1(ref: np.ndarray, det: np.ndarray, fs: float, tol_sec: float) -> dict:
    """Greedy one-to-one matching (mirrors metrics.events semantics)."""
    from ..metrics import peak_detection_metrics

    return peak_detection_metrics(np.asarray(ref), np.asarray(det), fs, tol_sec)


def _interp_nans(x: np.ndarray) -> tuple[np.ndarray, int]:
    """Linearly interpolate non-finite samples (index-typed gaps).

    The wrist-PPG records contain short contiguous NaN gaps (wrist-device
    dropouts, resampled layout).  Returns (cleaned, n_interpolated); edge
    NaNs are filled with the nearest finite value.  Raises ValueError if the
    channel has no finite samples at all.
    """
    x = np.asarray(x, dtype=float)
    bad = ~np.isfinite(x)
    n_bad = int(bad.sum())
    if n_bad == 0:
        return x, 0
    good = np.where(~bad)[0]
    if good.size == 0:
        raise ValueError("channel has no finite samples")
    xi = np.arange(len(x))
    x[bad] = np.interp(xi[bad], good, x[good])
    return x, n_bad


# ---------------------------------------------------------------------------
# BIDMC
# ---------------------------------------------------------------------------

class BidmcAdapter(DatasetAdapter):
    """BIDMC PPG and Respiration Dataset (53 ICU recordings @ 125 Hz).

    Validated subset: the 12 records ``bidmc01``..``bidmc12`` (deterministic,
    per task specification).  Ground truth: manual breath annotations (two
    annotators) marking inspiratory peaks — used as ``rsp_peak_indices``.
    PPG has no independent peak reference (structural metrics only); the ECG
    channel is exercised structurally and used for a clearly-labelled
    cross-channel HR *consistency* check (metrics_extra), not independent
    validation.
    """

    key = "bidmc"

    def info(self) -> DatasetInfo:
        return DatasetInfo(
            key=self.key,
            name="BIDMC PPG and Respiration Dataset",
            category="ppg",
            modalities=["ppg", "rsp", "ecg"],
            source_url="https://physionet.org/content/bidmc/ (mirror: "
                       "https://archive.physionet.org/physiobank/database/bidmc/)",
            license="ODC-BY 1.0",
            version="1.0.0",
            citation="Pimentel MAF et al. Toward a robust estimation of respiratory rate "
                     "from pulse oximeters. IEEE TBME 64(8):1914-1923 (2017).",
            lamina_ops=["ppg-clean", "ppg-peaks", "rsp-clean", "rsp-cycles",
                        "ecg-peaks", "hrv"],
            notes="53 adult ICU recordings @ 125 Hz, 8 min, with two-annotator breath "
                  "annotations. Validated subset: bidmc01..bidmc12 fetched from the "
                  "archive.physionet.org mirror (WFDB .dat/.hea/.breath layout; "
                  "physionet.org 403 from validation environment). Breath annotations "
                  "are 0-based sample indices at 125 Hz marking inspiratory peaks "
                  "(verified empirically); ann1 used as rsp_peak_indices reference, "
                  "ann2 agreement reported in metrics_extra. No independent PPG peak "
                  "reference exists (PPG structural metrics only). ECG @125 Hz "
                  "exercised for observation (below/at edge of the 5-15 Hz bandpass "
                  "comfort zone). Cross-channel PPG-vs-ECG HR consistency is a "
                  "self-consistency check, not independent validation.",
        )

    # -- acquisition --------------------------------------------------------

    @staticmethod
    def _data_dir(cache_dir: str | None) -> Path:
        env = os.environ.get("LAMINA_BIDMC_DIR")
        if env:
            return Path(env)
        if cache_dir:
            return Path(cache_dir) / "bidmc"
        return Path.home() / ".cache" / "lamina-validation" / "bidmc"

    def _ensure_record(self, data_dir: Path, record: str) -> None:
        data_dir.mkdir(parents=True, exist_ok=True)
        for ext in _BIDMC_EXT:
            path = data_dir / f"{record}{ext}"
            if path.exists() and path.stat().st_size > 0:
                continue
            url = f"{_BIDMC_MIRROR}/{record}{ext}"
            with urllib.request.urlopen(url, timeout=120) as resp:
                path.write_bytes(resp.read())

    def check_access(self, cache_dir: str | None = None) -> AccessReport:
        data_dir = self._data_dir(cache_dir)
        try:
            self._ensure_record(data_dir, _BIDMC_RECORDS[0])
            import wfdb  # local import: keeps module importable without wfdb

            rec = wfdb.rdrecord(str(data_dir / _BIDMC_RECORDS[0]))
            if rec.sig_len <= 0 or "PLETH," not in rec.sig_name:
                return AccessReport(
                    status=AccessStatus.FAILED,
                    reason="BIDMC mirror reachable but record content unexpected",
                    detail=f"sig_name={rec.sig_name} sig_len={rec.sig_len}",
                )
            return AccessReport(
                status=AccessStatus.VALIDATED,
                reason="BIDMC mirror (archive.physionet.org) reachable; WFDB records readable",
                detail=f"data_dir={data_dir}; subset={_BIDMC_RECORDS[0]}..{_BIDMC_RECORDS[-1]}",
            )
        except Exception as exc:  # noqa: BLE001 - honest access report
            return AccessReport(
                status=AccessStatus.INACCESSIBLE,
                reason="BIDMC fetch/read failed from validation environment",
                detail=f"{type(exc).__name__}: {exc}",
            )

    # -- iteration ----------------------------------------------------------

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

        from ..bridge import LaminaBridge

        data_dir = self._data_dir(cache_dir)
        names = list(_BIDMC_RECORDS)
        if subjects:  # subject id == record id for BIDMC (one record per subject)
            names = [n for n in names if n in subjects]
        if recordings:
            names = [n for n in names if n in recordings]
        if smoke:
            names = names[:2]
        if max_recordings is not None and len(names) > max_recordings:
            names = sorted(random.Random(seed).sample(sorted(names), max_recordings))

        extra_rows: list[dict] = []
        bridge: LaminaBridge | None = None
        for name in names:
            self._ensure_record(data_dir, name)
            rec = wfdb.rdrecord(str(data_dir / name))
            fs = float(rec.fs)
            sig_names = [s.rstrip(",") for s in rec.sig_name]
            sig = {s.rstrip(","): rec.p_signal[:, i]
                   for i, s in enumerate(rec.sig_name)}

            signals: dict[str, Signal] = {}
            if "RESP" in sig_names:
                signals["resp"] = Signal(
                    samples=np.asarray(sig["RESP"], dtype=float),
                    sampling_rate=fs, modality="rsp",
                    units="arbitrary (impedance pneumograph)",
                    subject_id=name, recording_id=name, channel="RESP",
                )
            if "PLETH" in sig_names:
                signals["ppg"] = Signal(
                    samples=np.asarray(sig["PLETH"], dtype=float),
                    sampling_rate=fs, modality="ppg",
                    units="NU (raw ADC counts)",
                    subject_id=name, recording_id=name, channel="PLETH",
                )
            if "II" in sig_names:
                signals["ecg"] = Signal(
                    samples=np.asarray(sig["II"], dtype=float),
                    sampling_rate=fs, modality="ecg", units="mV",
                    subject_id=name, recording_id=name, channel="II",
                )

            ann = wfdb.rdann(str(data_dir / name), "breath")
            ann1 = np.array([s for s, a in zip(ann.sample, ann.aux_note)
                             if a == "ann1"], dtype=int)
            ann2 = np.array([s for s, a in zip(ann.sample, ann.aux_note)
                             if a == "ann2"], dtype=int)
            ann1.sort()
            ann2.sort()

            references = {"rsp_peak_indices": ann1}
            metadata = {
                "source": "archive.physionet.org mirror, WFDB layout",
                "breath_annotation_semantics": "0-based sample index @125 Hz of the "
                    "inspiratory peak (max expansion); aux_note identifies annotator",
                "reference_annotator": "ann1",
                "n_breaths_ann1": int(len(ann1)),
                "n_breaths_ann2": int(len(ann2)),
                "channels_present": sig_names,
                "metrics_extra": "metrics_extra.csv: ann1-vs-ann2 agreement, "
                    "Lamina-rsp-vs-ann2 F1, Lamina PPG-HR vs ECG-HR consistency",
            }

            # --- extra metrics (self-computed, clearly labelled) -----------
            rid = name
            if len(ann1) and len(ann2):
                m = _match_f1(ann1, ann2, fs, 0.5)
                extra_rows.append(dict(
                    dataset=self.key, recording_id=rid,
                    metric="interannotator_breath_f1", value=m["f1"],
                    units="fraction",
                    notes="ann2 matched against ann1 @0.5 s tolerance (greedy 1:1)"))
            if bridge is None:
                bridge = LaminaBridge()
            if "resp" in signals and len(ann2):
                try:
                    r = bridge.rsp_cycles(signals["resp"].samples, fs)
                    insp = np.array([c["inspiration_index"] for c in r["cycles"]])
                    m = _match_f1(ann2, insp, fs, 0.5)
                    extra_rows.append(dict(
                        dataset=self.key, recording_id=rid,
                        metric="lamina_rsp_vs_ann2_f1", value=m["f1"],
                        units="fraction",
                        notes="robustness of runner metric to annotator choice "
                              "(runner uses ann1)"))
                except Exception as exc:  # noqa: BLE001
                    extra_rows.append(dict(
                        dataset=self.key, recording_id=rid,
                        metric="lamina_rsp_vs_ann2_f1", value="",
                        units="fraction", notes=f"failed: {type(exc).__name__}: {exc}"))
            if "ppg" in signals and "ecg" in signals:
                try:
                    n = signals["ppg"].n_samples
                    ppg_pk = np.array(
                        bridge.ppg_peaks(signals["ppg"].samples, fs)["peaks"])
                    ecg_pk = np.array(
                        bridge.ecg_peaks(signals["ecg"].samples, fs)["peaks"])
                    t, hr_ppg = _windowed_hr_from_peaks(ppg_pk, fs, n, 60.0, 30.0)
                    _, hr_ecg = _windowed_hr_from_peaks(ecg_pk, fs, n, 60.0, 30.0)
                    ok = np.isfinite(hr_ppg) & np.isfinite(hr_ecg)
                    if ok.sum() >= 2:
                        diff = hr_ppg[ok] - hr_ecg[ok]
                        extra_rows.append(dict(
                            dataset=self.key, recording_id=rid,
                            metric="hr_consistency_ppg_vs_ecg_mae",
                            value=float(np.mean(np.abs(diff))), units="bpm",
                            notes="CONSISTENCY CHECK (not independent validation): "
                                  "Lamina ppg-peaks HR vs Lamina ecg-peaks HR, "
                                  "60 s windows / 30 s step"))
                        extra_rows.append(dict(
                            dataset=self.key, recording_id=rid,
                            metric="hr_consistency_ppg_vs_ecg_bias",
                            value=float(np.mean(diff)), units="bpm",
                            notes="mean(PPG-HR minus ECG-HR); both from Lamina"))
                    extra_rows.append(dict(
                        dataset=self.key, recording_id=rid,
                        metric="ecg_peaks_per_min",
                        value=float(len(ecg_pk) / (n / fs / 60.0)), units="1/min",
                        notes="structural: Lamina ecg-peaks rate on II @125 Hz; "
                              "plausible range ~40-180; 125 Hz is below the 5-15 Hz "
                              "bandpass comfort zone (observation)"))
                except Exception as exc:  # noqa: BLE001
                    extra_rows.append(dict(
                        dataset=self.key, recording_id=rid,
                        metric="hr_consistency_ppg_vs_ecg_mae", value="",
                        units="bpm", notes=f"failed: {type(exc).__name__}: {exc}"))

            yield Recording(
                recording_id=rid,
                subject_id=name,
                signals=signals,
                references=references,
                metadata=metadata,
            )

        if extra_rows:
            _append_metrics_extra(self.key, extra_rows)


# ---------------------------------------------------------------------------
# Wrist PPG During Exercise
# ---------------------------------------------------------------------------

class WristPpgExerciseAdapter(DatasetAdapter):
    """Wrist PPG During Exercise (Jarchi & Casson 2017).

    19 records (subjects s1..s9 minus s7; activities walk/run/
    low/high_resistance_bike) @ 256 Hz with wrist PPG, low-noise 3-axis ACC
    and chest ECG.  The dataset ships chest-ECG beat annotations (``.atr``,
    ``N`` beats) which serve a dual role:

    - ``ecg_peak_indices`` reference → runner validates Lamina ``ecg-peaks``
      on the chest ECG directly (150 ms tolerance);
    - windowed HR reference (``hr_bpm``) derived from the annotated beats —
      compared against ``hr_bpm_estimated`` from Lamina ``ppg-peaks`` on the
      wrist PPG (8 s windows / 2 s step, the standard protocol for this
      dataset).

    Motion stratification (walk vs run vs bike resistance) is reported via
    per-activity HR bias in metrics_extra.  A consistency cross-check of
    annotation-derived HR vs Lamina-ecg-peaks-derived HR is also written to
    metrics_extra.

    Caveat: the record header declares all 15 channels at 256 Hz and
    includes a ``sample_times_for_all_signals_apart_from_ecg`` channel —
    non-ECG channels were resampled/aligned to 256 Hz by the dataset authors
    from the wrist device's native timing.  Channels are treated as uniform
    256 Hz per the header (documented limitation).
    """

    key = "wrist-ppg-exercise"

    #: HR estimation windows (seconds): 8 s windows, 2 s step — the standard
    #: protocol for this dataset (Jarchi & Casson 2017).
    HR_WINDOW_SEC = 8.0
    HR_STEP_SEC = 2.0

    def info(self) -> DatasetInfo:
        return DatasetInfo(
            key=self.key,
            name="Wrist PPG During Exercise",
            category="ppg",
            modalities=["ppg", "ecg", "acc"],
            source_url="https://physionet.org/content/wrist-ppg-during-exercise/",
            license="ODC-BY 1.0",
            version="1.0.0",
            citation="Jarchi D, Casson AJ. Estimation of heart rate from wrist PPG "
                     "during exercise. EMBC (2017).",
            lamina_ops=["ppg-clean", "ppg-peaks", "ecg-peaks", "hrv"],
            notes="19 records (subjects s1-s9 except s7) @ 256 Hz; wrist PPG + chest "
                  "ECG + low-noise 3-axis accelerometry (records also contain gyro and "
                  "magnetometer channels, not exercised); activities walk/run/"
                  "low/high resistance bike. Fetched via wfdb (pn_dir='wrist'). "
                  "Dataset ships chest-ECG beat annotations (.atr): used both as "
                  "ecg_peak_indices (Lamina ecg-peaks validation @150 ms) and as the "
                  "windowed HR reference (8 s windows / 2 s step) against Lamina "
                  "ppg-peaks HR. Per-activity HR bias in metrics_extra. Non-ECG "
                  "channels resampled to 256 Hz by the dataset authors (header "
                  "declares 256 Hz; sample_times channel documents native timing).",
        )

    # -- acquisition --------------------------------------------------------

    @staticmethod
    def _data_dir(cache_dir: str | None) -> Path:
        env = os.environ.get("LAMINA_WRIST_DIR")
        if env:
            return Path(env)
        if cache_dir:
            return Path(cache_dir) / "wrist"
        return Path.home() / ".cache" / "lamina-validation" / "wrist"

    @staticmethod
    def _record_names() -> list[str]:
        import wfdb

        return sorted(wfdb.get_record_list(_WRIST_PN_DIR))

    def _ensure_record(self, data_dir: Path, name: str) -> None:
        import wfdb

        data_dir.mkdir(parents=True, exist_ok=True)
        if (data_dir / f"{name}.dat").exists() and (data_dir / f"{name}.hea").exists():
            return
        wfdb.dl_database(_WRIST_PN_DIR, dl_dir=str(data_dir), records=[name])

    def check_access(self, cache_dir: str | None = None) -> AccessReport:
        try:
            names = self._record_names()
            if not names:
                return AccessReport(status=AccessStatus.INACCESSIBLE,
                                    reason="wfdb returned an empty record list for 'wrist'")
            data_dir = self._data_dir(cache_dir)
            self._ensure_record(data_dir, names[0])
            import wfdb

            rec = wfdb.rdrecord(str(data_dir / names[0]))
            return AccessReport(
                status=AccessStatus.VALIDATED,
                reason="wfdb pn_dir='wrist' reachable; records cached locally",
                detail=f"{len(names)} records; data_dir={data_dir}; "
                       f"channels={rec.sig_name}",
            )
        except Exception as exc:  # noqa: BLE001
            return AccessReport(
                status=AccessStatus.INACCESSIBLE,
                reason="wrist-ppg-exercise fetch failed from validation environment",
                detail=f"{type(exc).__name__}: {exc}",
            )

    # -- iteration ----------------------------------------------------------

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

        from ..bridge import LaminaBridge

        data_dir = self._data_dir(cache_dir)
        names = self._record_names()
        if subjects:
            names = [n for n in names if n.split("_")[0] in subjects]
        if recordings:
            names = [n for n in names if n in recordings]
        if smoke:
            names = names[:2]
        if max_recordings is not None and len(names) > max_recordings:
            names = sorted(random.Random(seed).sample(sorted(names), max_recordings))

        extra_rows: list[dict] = []
        bridge: LaminaBridge | None = None
        for name in names:
            self._ensure_record(data_dir, name)
            rec = wfdb.rdrecord(str(data_dir / name))
            fs = float(rec.fs)
            sig = {}
            nan_counts: dict[str, int] = {}
            for i, s in enumerate(rec.sig_name):
                cleaned, n_bad = _interp_nans(rec.p_signal[:, i])
                sig[s] = cleaned
                if n_bad:
                    nan_counts[s] = n_bad
            subject = name.split("_")[0]
            activity = "_".join(name.split("_")[1:])

            signals: dict[str, Signal] = {}
            if "wrist_ppg" in sig:
                signals["ppg"] = Signal(
                    samples=np.asarray(sig["wrist_ppg"], dtype=float),
                    sampling_rate=fs, modality="ppg", units="mV (see header gain)",
                    subject_id=subject, recording_id=name, channel="wrist_ppg",
                )
            if "chest_ecg" in sig:
                signals["ecg"] = Signal(
                    samples=np.asarray(sig["chest_ecg"], dtype=float),
                    sampling_rate=fs, modality="ecg", units="mV",
                    subject_id=subject, recording_id=name, channel="chest_ecg",
                )
            for ax in ("x", "y", "z"):
                ch = f"wrist_low_noise_accelerometer_{ax}"
                if ch in sig:
                    signals[f"acc_{ax}"] = Signal(
                        samples=np.asarray(sig[ch], dtype=float),
                        sampling_rate=fs, modality="acc", units="ms^-2",
                        subject_id=subject, recording_id=name, channel=ch,
                    )

            # Chest-ECG beat annotations shipped with the dataset (.atr).
            ann = wfdb.rdann(str(data_dir / name), "atr")
            beats = np.sort(np.asarray(ann.sample, dtype=int))

            if bridge is None:
                bridge = LaminaBridge()
            references: dict = {"ecg_peak_indices": beats}
            hr_meta: dict = {
                "ecg_annotation_semantics": ".atr 'N' beat annotations on chest_ecg, "
                    "0-based sample indices @256 Hz (dataset-provided)",
                "n_ecg_annotations": int(len(beats)),
            }
            if "ppg" in signals and len(beats) >= 2:
                n = signals["ppg"].n_samples
                ppg_pk = np.array(bridge.ppg_peaks(signals["ppg"].samples, fs)["peaks"])
                # HR reference from ANNOTATED beats; estimate from Lamina ppg-peaks.
                t, hr_ref = _windowed_hr_from_peaks(
                    beats, fs, n, self.HR_WINDOW_SEC, self.HR_STEP_SEC)
                _, hr_est = _windowed_hr_from_peaks(
                    ppg_pk, fs, n, self.HR_WINDOW_SEC, self.HR_STEP_SEC)
                ok = np.isfinite(hr_ref) & np.isfinite(hr_est)
                references["hr_bpm"] = hr_ref[ok]
                references["hr_bpm_estimated"] = hr_est[ok]
                hr_meta.update({
                    "hr_window_sec": self.HR_WINDOW_SEC,
                    "hr_step_sec": self.HR_STEP_SEC,
                    "hr_n_windows": int(ok.sum()),
                    "hr_reference_derivation": "windowed HR from dataset chest-ECG "
                        "beat annotations (independent of Lamina)",
                    "hr_estimate_derivation": "Lamina ppg-peaks on wrist_ppg",
                })
                if ok.sum() >= 2:
                    diff = hr_est[ok] - hr_ref[ok]
                    extra_rows.append(dict(
                        dataset=self.key, recording_id=name,
                        metric="hr_bias_bpm", value=float(np.mean(diff)),
                        units="bpm",
                        notes=f"Bland-Altman-style bias (Lamina PPG-HR minus "
                              f"annotation-HR); activity={activity}"))
                    extra_rows.append(dict(
                        dataset=self.key, recording_id=name,
                        metric="hr_loa_halfwidth_bpm",
                        value=float(1.96 * np.std(diff)), units="bpm",
                        notes=f"1.96*std of differences; activity={activity}"))
                    extra_rows.append(dict(
                        dataset=self.key, recording_id=name,
                        metric="hr_windows_covered_fraction",
                        value=float(ok.mean()), units="fraction",
                        notes=f"windows with >=1 complete IBI on both channels; "
                              f"activity={activity}"))
            # Consistency cross-check: annotation-HR vs Lamina-ecg-peaks-HR
            # (quantifies the ECG-detector contribution separately).
            if "ecg" in signals and len(beats) >= 2:
                try:
                    n = signals["ecg"].n_samples
                    ecg_pk = np.array(
                        bridge.ecg_peaks(signals["ecg"].samples, fs)["peaks"])
                    _, hr_ann = _windowed_hr_from_peaks(
                        beats, fs, n, self.HR_WINDOW_SEC, self.HR_STEP_SEC)
                    _, hr_lecg = _windowed_hr_from_peaks(
                        ecg_pk, fs, n, self.HR_WINDOW_SEC, self.HR_STEP_SEC)
                    ok2 = np.isfinite(hr_ann) & np.isfinite(hr_lecg)
                    if ok2.sum() >= 2:
                        d2 = hr_lecg[ok2] - hr_ann[ok2]
                        extra_rows.append(dict(
                            dataset=self.key, recording_id=name,
                            metric="hr_consistency_lamina_ecg_vs_annotation_mae",
                            value=float(np.mean(np.abs(d2))), units="bpm",
                            notes=f"CONSISTENCY CHECK: Lamina ecg-peaks HR vs "
                                  f"annotation HR; activity={activity}"))
                except Exception as exc:  # noqa: BLE001
                    extra_rows.append(dict(
                        dataset=self.key, recording_id=name,
                        metric="hr_consistency_lamina_ecg_vs_annotation_mae",
                        value="", units="bpm",
                        notes=f"failed: {type(exc).__name__}: {exc}"))

            yield Recording(
                recording_id=name,
                subject_id=subject,
                signals=signals,
                references=references,
                metadata={
                    "activity": activity,
                    "source": f"wfdb pn_dir='{_WRIST_PN_DIR}', local cache {data_dir}",
                    "nan_interpolated_samples": nan_counts or "none",
                    "nan_handling": "linear interpolation over wrist-device dropout "
                        "gaps (documented; bridge rejects non-finite input)",
                    **hr_meta,
                    "metrics_extra": "metrics_extra.csv: per-recording HR bias, "
                        "limits-of-agreement half-width, annotation-vs-Lamina-ECG "
                        "consistency; all rows carry activity labels",
                },
            )

        if extra_rows:
            _append_metrics_extra(self.key, extra_rows)


# ---------------------------------------------------------------------------
# PulseDB — inaccessible from the validation environment (stub retained)
# ---------------------------------------------------------------------------

class PulseDbAdapter(StubDatasetAdapter):
    key = "pulsedb"
    pending_status = AccessStatus.INACCESSIBLE
    pending_reason = ("hosts unreachable/credentialed from validation environment: "
                      "pulsedb.org connection fails, Box/Drive mirrors blocked, "
                      "Kaggle download requires authentication")

    def info(self) -> DatasetInfo:
        return DatasetInfo(
            key=self.key,
            name="PulseDB",
            category="ppg",
            modalities=["ppg", "ecg", "abp"],
            source_url="https://pulsedb.org/ (PhysioNet: https://physionet.org/content/pulsedb/; "
                       "Kaggle: https://www.kaggle.com/datasets/weiaiwang/pulsedb)",
            license="ODbL 1.0",
            version="1.0",
            citation="Wang W et al. PulseDB: A large, cleaned dataset based on MIMIC-III "
                     "and VitalDB for benchmarking cuff-less blood pressure estimation "
                     "methods. Frontiers (2022).",
            lamina_ops=["ppg-clean", "ppg-peaks"],
            notes="~5.2M 10-second segments (PPG/ECG/ABP @125 Hz) from MIMIC-III and "
                  "VitalDB, distributed as Matlab .mat files under ODbL 1.0. "
                  "INACCESSIBLE from this validation environment: official site "
                  "unreachable, Box/Drive mirrors blocked, Kaggle requires auth. "
                  "Stub retained with real metadata (Stage 2 TODO if a mirror "
                  "becomes reachable).",
        )


# ---------------------------------------------------------------------------
# MIMIC-III waveform datasets — credentialed access required (stubs retained)
# ---------------------------------------------------------------------------

class MimicIiiWaveformAdapter(StubDatasetAdapter):
    key = "mimic-iii-waveform"
    pending_status = AccessStatus.INACCESSIBLE
    pending_reason = ("requires PhysioNet credentialed access (CITI training + signed "
                      "DUA); HTTP 403 verified from validation environment")

    def info(self) -> DatasetInfo:
        return DatasetInfo(
            key=self.key,
            name="MIMIC-III Waveform Database",
            category="ppg",
            modalities=["ppg", "ecg", "abp", "rsp"],
            source_url="https://physionet.org/content/mimic3wdb/",
            license="PhysioNet Credentialed Health Data License",
            version="1.0",
            citation="Johnson AEW et al. MIMIC-III, a freely accessible critical care "
                     "database. Scientific Data 3:160035 (2016).",
            lamina_ops=["ppg-clean", "ppg-peaks", "hrv"],
            notes="Credentialed access required (CITI training + signed DUA); "
                  "credential application is out of scope for the validation "
                  "environment (403 verified). Recorded as inaccessible.",
        )


class MimicIiiExtPpgAdapter(StubDatasetAdapter):
    key = "mimic-iii-ext-ppg"
    pending_status = AccessStatus.INACCESSIBLE
    pending_reason = ("requires PhysioNet credentialed access to the parent MIMIC-III "
                      "Waveform Database; HTTP 403 verified from validation environment")

    def info(self) -> DatasetInfo:
        return DatasetInfo(
            key=self.key,
            name="MIMIC-III-Ext-PPG",
            category="ppg",
            modalities=["ppg", "abp"],
            source_url="https://physionet.org/content/mimic3wdb/ (derived records)",
            license="PhysioNet Credentialed Health Data License",
            version="1.0",
            citation="Kotzen K et al. PPG and BP waveform features derived from MIMIC-III. "
                     "(2022).",
            lamina_ops=["ppg-clean", "ppg-peaks"],
            notes="Derived PPG/BP feature dataset from MIMIC-III waveforms; same "
                  "credentialed-access barrier as the parent database (403 verified). "
                  "Recorded as inaccessible.",
        )
