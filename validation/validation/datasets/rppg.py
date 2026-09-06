"""rPPG dataset adapters (Stage 2).

SCAMPS is fully implemented (synthetic-data validation of the Lamina rPPG
pipeline: ``rppg-algorithm`` + ``ppg-peaks``).  All other rPPG datasets were
probed for access on 2026-09-06 and are honestly marked ``inaccessible`` with
the exact verified reason (see each ``check_access`` detail).
"""

from __future__ import annotations

import json
import os
import random
from pathlib import Path
from typing import Iterator

import numpy as np

from ..schema import Signal
from .base import (
    AccessReport,
    AccessStatus,
    DatasetAdapter,
    DatasetInfo,
    Recording,
    StubDatasetAdapter,
)

# ---------------------------------------------------------------------------
# Inaccessible datasets (verified probes, 2026-09-06; one quick probe each)
# ---------------------------------------------------------------------------


class PureAdapter(StubDatasetAdapter):
    key = "pure"
    pending_status = AccessStatus.INACCESSIBLE
    pending_reason = "needs-registration: download requires e-mail application to TU Ilmenau"

    def info(self) -> DatasetInfo:
        return DatasetInfo(
            key=self.key,
            name="PURE (Pulse Rate Detection Dataset)",
            category="rppg",
            modalities=["rgb_video", "bvp"],
            source_url="https://www.tu-ilmenau.de/neurob/data-sets-code/pulse-rate-detection-dataset-pure",
            license="Research use (custom)",
            version="1.0",
            citation="Stricker R, Müller S, Gross HM. Non-contact video-based pulse rate "
                     "measurement on a mobile service robot. RO-MAN (2014).",
            lamina_ops=["rppg-algorithm", "ppg-clean", "ppg-peaks"],
            notes="10 subjects x 6 sessions, uncompressed PNG image sequences @ 30 fps "
                  "with contact SpO2 reference; ~10 GB.",
        )

    def check_access(self, cache_dir: str | None = None) -> AccessReport:
        return AccessReport(
            status=self.pending_status,
            reason=self.pending_reason,
            detail="verified 2026-09-06: landing page reachable (HTTP 200) but the page "
                   "states the dataset is provided only after an e-mail application to "
                   "nikr-datasets-request@tu-ilmenau.de; no direct download link exists.",
        )


class UbfcRppgAdapter(StubDatasetAdapter):
    key = "ubfc-rppg"
    pending_status = AccessStatus.INACCESSIBLE
    pending_reason = "blocked-host: official Google Sites page unreachable; Kaggle mirror requires authentication"

    def info(self) -> DatasetInfo:
        return DatasetInfo(
            key=self.key,
            name="UBFC-rPPG",
            category="rppg",
            modalities=["rgb_video", "bvp"],
            source_url="https://sites.google.com/view/ybenezeth/ubfcrppg",
            license="Research use (custom)",
            version="2.0",
            citation="Bobbia S et al. Unsupervised skin tissue segmentation for remote "
                     "photoplethysmography. Pattern Recognition Letters 124 (2019).",
            lamina_ops=["rppg-algorithm", "ppg-clean", "ppg-peaks"],
            notes="42 videos @ 30 fps with synchronized CMS50E pulse oximeter reference.",
        )

    def check_access(self, cache_dir: str | None = None) -> AccessReport:
        return AccessReport(
            status=self.pending_status,
            reason=self.pending_reason,
            detail="verified 2026-09-06: curl to https://sites.google.com/view/ybenezeth/ubfcrppg "
                   "fails from this environment (curl exit code, HTTP 000 — host blocked); "
                   "the Kaggle mirror requires authenticated access (needs-registration).",
        )


class CohfaceAdapter(StubDatasetAdapter):
    key = "cohface"
    pending_status = AccessStatus.INACCESSIBLE
    pending_reason = "needs-registration: Zenodo record files are restricted (EULA, academic signatory)"

    def info(self) -> DatasetInfo:
        return DatasetInfo(
            key=self.key,
            name="COHFACE",
            category="rppg",
            modalities=["rgb_video", "bvp"],
            source_url="https://www.idiap.ch/en/dataset/cohface",
            license="Research use (Idiap)",
            version="1.0",
            citation="Heusch G, Anjos A, Marcel S. A reproducible study on remote heart "
                     "rate measurement. arXiv:1709.00962 (2017).",
            lamina_ops=["rppg-algorithm", "ppg-clean", "ppg-peaks"],
            notes="160 videos (40 subjects), compressed H.264 @ 20 Hz with synchronized "
                  "pulse-oximetry; includes lighting/motion conditions.",
        )

    def check_access(self, cache_dir: str | None = None) -> AccessReport:
        return AccessReport(
            status=self.pending_status,
            reason=self.pending_reason,
            detail="verified 2026-09-06: zenodo.org is reachable from this environment, but "
                   "the COHFACE record (https://zenodo.org/api/records/4081054) reports "
                   "access_right='restricted' with an empty public file list; access requires "
                   "an EULA request signed by a permanent-position academic signatory.",
        )


class UbfcPhysAdapter(StubDatasetAdapter):
    key = "ubfc-phys"
    pending_status = AccessStatus.INACCESSIBLE
    pending_reason = "download size exceeds environment timebox"

    def info(self) -> DatasetInfo:
        return DatasetInfo(
            key=self.key,
            name="UBFC-Phys",
            category="rppg",
            modalities=["rgb_video", "bvp", "eda"],
            source_url="https://sites.google.com/view/ybenezeth/ubfc-phys",
            license="Research use (custom)",
            version="1.0",
            citation="Meziat Sabour R et al. UBFC-Phys: A multimodal database for "
                     "psychophysiological studies of social stress. IEEE TAFFC (2021).",
            lamina_ops=["rppg-algorithm", "ppg-clean", "ppg-peaks", "eda-clean",
                        "eda-decompose", "eda-peaks"],
            notes="56 subjects, stress/no-stress tasks; contact BVP + EDA (Empatica E4) "
                  "reference.",
        )

    def check_access(self, cache_dir: str | None = None) -> AccessReport:
        return AccessReport(
            status=self.pending_status,
            reason=self.pending_reason,
            detail="verified 2026-09-06: https://search-data.ubfc.fr/dl_data.php?file={140..145,220} "
                   "serves 303 redirects to signed storage-data.ubfc.fr URLs (range GET works). "
                   "Archive sizes via Content-Range: s1_to_s10.7z=152.9 GB, s11_to_s20.7z=160.0 GB, "
                   "s21_to_s30.7z=146.5 GB, s31_to_s40.7z=150.4 GB, s41_to_s50.7z=151.7 GB, "
                   "s51_to_s56.7z=91.7 GB (smallest); file=145 is only the documentation PDF "
                   "(4.1 MB). The smallest subject archive (91.7 GB) far exceeds the 1.5 GB "
                   "environment download budget, so no data was acquired.",
        )


class MmpdAdapter(StubDatasetAdapter):
    key = "mmpd"
    pending_status = AccessStatus.INACCESSIBLE
    pending_reason = "needs-registration: requires signed release agreement by faculty e-mail"

    def info(self) -> DatasetInfo:
        return DatasetInfo(
            key=self.key,
            name="MMPD (Multi-domain Mobile Video Physiology Dataset)",
            category="rppg",
            modalities=["rgb_video", "bvp"],
            source_url="https://github.com/McJackTang/MMPD_rPPG_dataset",
            license="Research use (custom)",
            version="1.0",
            citation="Tang J et al. MMPD: Multi-domain Mobile Video Physiology Dataset. "
                     "arXiv:2305.00759 (2023).",
            lamina_ops=["rppg-algorithm", "ppg-clean", "ppg-peaks"],
            notes="660 mobile-phone videos with PPG reference, skin-tone/lighting/motion "
                  "stratification; several-hundred-GB full release, subset available.",
        )

    def check_access(self, cache_dir: str | None = None) -> AccessReport:
        return AccessReport(
            status=self.pending_status,
            reason=self.pending_reason,
            detail="verified 2026-09-06: GitHub landing page reachable (HTTP 200) but the "
                   "repository distributes no data directly; access requires a signed release "
                   "agreement submitted from a faculty e-mail address.",
        )


class IbvpAdapter(StubDatasetAdapter):
    key = "ibvp"
    pending_status = AccessStatus.INACCESSIBLE
    pending_reason = "needs-registration: requires EULA signed by an academic supervisor"

    def info(self) -> DatasetInfo:
        return DatasetInfo(
            key=self.key,
            name="iBVP (iPhone-based video PPG dataset)",
            category="rppg",
            modalities=["rgb_video", "bvp"],
            source_url="https://github.com/physiotherapy/iBVP-dataset (see paper)",
            license="Research use (custom)",
            version="1.0",
            citation="Joshi K et al. iBVP Dataset: RGB video and iPPG signal dataset. "
                     "NPJ Digital Medicine (2024).",
            lamina_ops=["rppg-algorithm", "ppg-clean", "ppg-peaks"],
            notes="Front-facing iPhone videos with synchronized ear-clip PPG; activity "
                  "conditions (sitting/walking).",
        )


# ---------------------------------------------------------------------------
# SCAMPS (synthetic rPPG corpus) — fully implemented adapter
# ---------------------------------------------------------------------------

#: Default data locations (first match wins): --cache-dir, $SCAMPS_DATA_DIR,
#: then $HOME/data/scamps.
_SCAMPS_URLS = {
    "waveforms": "https://facesyntheticspubwedata.z6.web.core.windows.net/"
                 "neurips-2022/scamps_waveforms_csv.tar.gz",
    "videos_example": "https://facesyntheticspubwedata.z6.web.core.windows.net/"
                      "neurips-2022/scamps_videos_example.tar.gz",
}


class ScampsAdapter(DatasetAdapter):
    """SCAMPS example-set adapter (synthetic-data validation).

    Uses the 10 example ``.mat`` videos (``scamps_videos_example.tar.gz``), each
    containing raw RGB frames (``Xsub``: 240x240 face crop, 600 frames @ 30 fps)
    and the per-frame ground-truth PPG (``d_ppg``) that was used to synthesize
    the video — so video/reference sync is exact by construction.

    Validation-side orchestration (documented assumptions):

    - **ROI**: the dataset-provided per-frame ``skin_mask`` (threshold > 0.5) on
      the ``Xsub`` face crop; per-frame channel means over masked pixels, with
      ``valid_pixel_counts`` = mask area. This deliberately bypasses face
      detection so the measurement isolates Lamina's rPPG algorithms.
    - **Timestamps**: uniform 30 fps (``t_ppg`` grid in the waveform CSVs and
      the SCAMPS documentation); frame i -> t = i/30.
    - **Reference peaks**: detected validation-side on ``d_ppg`` with scipy
      (order-3 Butterworth 0.75–2.5 Hz bandpass + ``find_peaks``,
      min distance 0.4 s) — independent of the implementation under test.
    - **HR series**: 6 s windows, 1 s step; HR = mean(60/IBI) over IBIs whose
      midpoint falls inside the window; windows with < 2 peaks yield NaN and
      are dropped by the rate metrics.
    - **Algorithms**: each (video, algorithm) pair — green / chrom / pos — is
      yielded as one recording so per-algorithm metrics are computed by the
      standard runner.

    This is *synthetic*-data validation of the rPPG pipeline; it does not
    replace real-video validation (all real-video rPPG datasets were
    inaccessible from this environment — see the stub adapters above).
    """

    key = "scamps"

    #: rPPG algorithms exercised per video.
    ALGORITHMS = ("green", "chrom", "pos")
    #: Nominal frame rate of SCAMPS videos (documented; matches t_ppg grid).
    FPS = 30.0
    #: Sliding-window config for the rppg-algorithm bridge op.
    RPPG_WINDOW_SEC = 3.0
    RPPG_STEP_SEC = 0.5
    #: HR-estimation windows.
    HR_WINDOW_SEC = 6.0
    HR_STEP_SEC = 1.0
    #: Reference-peak detector (validation-side, scipy).
    REF_BANDPASS_HZ = (0.75, 2.5)
    REF_MIN_PEAK_DIST_SEC = 0.4
    #: Waveform-vs-ground-truth alignment tolerance for peak-index mapping.
    ALIGN_TOL_SEC = 1e-3

    def __init__(self) -> None:
        self._bridge = None
        self._extras: list[dict] = []

    # -- metadata -----------------------------------------------------------

    def info(self) -> DatasetInfo:
        return DatasetInfo(
            key=self.key,
            name="SCAMPS (synthetic rPPG corpus)",
            category="rppg",
            modalities=["rgb_video", "bvp"],
            source_url="https://github.com/danielmcduff/scamps",
            license="MIT",
            version="1.0",
            citation="McDuff D et al. SCAMPS: Synthetics for camera measurement of "
                     "physiological signals. NeurIPS (2022).",
            lamina_ops=["rppg-algorithm", "ppg-clean", "ppg-peaks"],
            notes="2800 synthetic avatar videos with perfectly synchronized ground-truth "
                  "PPG waveforms. This adapter validates on the 10-video public example "
                  "set (scamps_videos_example.tar.gz): synthetic-data validation of the "
                  "rPPG pipeline, not real-video validation. ROI = dataset-provided "
                  "skin_mask on the Xsub face crop (validation-side heuristic).",
        )

    # -- access --------------------------------------------------------------

    @staticmethod
    def _data_root(cache_dir: str | None) -> Path:
        if cache_dir:
            return Path(cache_dir)
        env = os.environ.get("SCAMPS_DATA_DIR")
        if env:
            return Path(env)
        return Path.home() / "data" / "scamps"

    def _video_dir(self, cache_dir: str | None) -> Path:
        return self._data_root(cache_dir) / "scamps_videos_example"

    def _videos(self, cache_dir: str | None) -> list[Path]:
        vdir = self._video_dir(cache_dir)
        return sorted(vdir.glob("P*.mat")) if vdir.is_dir() else []

    def check_access(self, cache_dir: str | None = None) -> AccessReport:
        videos = self._videos(cache_dir)
        if videos:
            return AccessReport(
                status=AccessStatus.VALIDATED,
                reason="example video set present in local cache",
                detail=f"{len(videos)} .mat videos under {self._video_dir(cache_dir)} "
                       f"(downloaded from {_SCAMPS_URLS['videos_example']}).",
            )
        return AccessReport(
            status=AccessStatus.NOT_ATTEMPTED,
            reason="SCAMPS example data not found in local cache",
            detail=f"expected .mat files under {self._video_dir(cache_dir)}; download "
                   f"{_SCAMPS_URLS['videos_example']} (ground truth is embedded per-frame "
                   f"as d_ppg; waveform CSVs: {_SCAMPS_URLS['waveforms']}).",
        )

    # -- recordings ------------------------------------------------------------

    def iter_recordings(
        self,
        subjects: list[str] | None = None,
        recordings: list[str] | None = None,
        max_recordings: int | None = None,
        seed: int = 0,
        smoke: bool = False,
        cache_dir: str | None = None,
    ) -> Iterator[Recording]:
        videos = self._videos(cache_dir)
        if not videos:
            raise FileNotFoundError(
                f"no SCAMPS .mat videos under {self._video_dir(cache_dir)}"
            )

        # Build the full (video, algorithm) recording plan.
        plan = [(v, a) for v in videos for a in self.ALGORITHMS]
        if subjects is not None:
            plan = [(v, a) for v, a in plan if v.stem in subjects]
        if recordings is not None:
            wanted = set(recordings)
            plan = [(v, a) for v, a in plan
                    if v.stem in wanted or f"{v.stem}-{a}" in wanted]
        if smoke:
            plan = plan[: len(self.ALGORITHMS)]  # first video, all algorithms
        elif max_recordings is not None and len(plan) > max_recordings:
            rng = random.Random(seed)
            plan = sorted(rng.sample(plan, max_recordings))

        self._extras = []
        # Group by video so each .mat is decoded exactly once.
        by_video: dict[Path, list[str]] = {}
        for v, a in plan:
            by_video.setdefault(v, []).append(a)

        for video_path, algos in by_video.items():
            data = _load_video(video_path, self._data_root(cache_dir))
            yield from self._recordings_for_video(video_path.stem, data, sorted(algos))

        # Persist extras next to the data cache (promoted into the results
        # directory by the run driver as metrics_extra.csv).
        if self._extras:
            out = self._data_root(cache_dir) / "rppg_extras_scamps.jsonl"
            try:
                with open(out, "w") as fh:
                    for row in self._extras:
                        fh.write(json.dumps(row) + "\n")
            except OSError:
                pass

    # -- per-video orchestration ----------------------------------------------

    def _bridge_client(self):
        if self._bridge is None:
            from ..bridge import LaminaBridge

            self._bridge = LaminaBridge()
        return self._bridge

    def _recordings_for_video(
        self, vid: str, data: dict, algorithms: list[str]
    ) -> Iterator[Recording]:
        bridge = self._bridge_client()
        ts = data["timestamps_sec"]
        fs = self.FPS
        gt_peaks = data["gt_peak_indices"]          # frame indices into d_ppg
        gt_peak_times = gt_peaks / fs
        ref_hr, win_centers = _windowed_hr(
            gt_peak_times, data["duration_sec"], self.HR_WINDOW_SEC, self.HR_STEP_SEC
        )

        for algo in algorithms:
            res = bridge.rppg_algorithm(
                ts,
                data["red"],
                data["green"],
                data["blue"],
                valid_pixel_counts=data["valid_pixel_counts"],
                config={
                    "algorithm": algo,
                    "window_sec": self.RPPG_WINDOW_SEC,
                    "step_sec": self.RPPG_STEP_SEC,
                },
            )
            wf = np.asarray(res["waveform"], dtype=float)
            wts = np.asarray(res["timestamps_sec"], dtype=float)
            wf_fs = float(res["mean_sampling_rate_hz"])

            # Verify the waveform timeline aligns with the frame grid before
            # exposing ground-truth peak indices (sync is exact for SCAMPS
            # synthetics, but check rather than assume).
            aligned = (
                len(wf) == len(ts)
                and len(wts) == len(ts)
                and np.max(np.abs(wts - ts)) < self.ALIGN_TOL_SEC
            )

            # Lamina peak detection on the rPPG waveform -> estimated HR.
            pk = bridge.ppg_peaks(wf, wf_fs)
            est_peaks = np.asarray(pk["peaks"], dtype=float)
            est_peak_times = est_peaks / wf_fs
            est_hr, _ = _windowed_hr(
                est_peak_times, data["duration_sec"],
                self.HR_WINDOW_SEC, self.HR_STEP_SEC,
            )

            # Waveform-correlation extras (bandpass-aligned, common grid).
            # Raw sign is reported as produced by Lamina; the sign-corrected
            # variant (standard rPPG benchmark practice, sign is physically
            # ambiguous for intensity-based algorithms) uses max(+r, -r).
            corr = _bandpass_correlation(wf, data["d_ppg"][: len(wf)], wf_fs)
            self._extras.append({
                "dataset": self.key,
                "subject_id": vid,
                "recording_id": f"{vid}-{algo}",
                "metric": "waveform_pearson_r_bandpassed",
                "value": corr,
                "value_sign_corrected": (None if corr is None else abs(corr)),
                "algorithm": algo,
                "gt_mean_hr_bpm": _mean_hr(gt_peak_times),
                "est_mean_hr_bpm": _mean_hr(est_peak_times),
                "n_gt_peaks": int(len(gt_peaks)),
                "n_est_peaks": int(pk["count"]),
                "timeline_aligned": bool(aligned),
                "mean_abs_pitch_rad": data["mean_abs_pitch"],
                "mean_abs_roll_rad": data["mean_abs_roll"],
                "mean_abs_yaw_rad": data["mean_abs_yaw"],
            })

            references = {
                "hr_bpm": ref_hr,
                "hr_bpm_estimated": est_hr,
                "hr_window_centers_sec": win_centers,
            }
            if aligned:
                # Ground-truth PPG peaks live on the same 30 Hz grid as the
                # rPPG waveform (synthetic data, zero sync error). Passed as a
                # plain list: runner uses `or`-chaining on this key.
                references["ppg_peak_indices"] = [int(i) for i in gt_peaks]

            yield Recording(
                recording_id=f"{vid}-{algo}",
                subject_id=vid,
                signals={
                    "rppg_waveform": Signal(
                        samples=wf,
                        sampling_rate=wf_fs,
                        modality="ppg",
                        timestamps=wts,
                        units="a.u.",
                        subject_id=vid,
                        recording_id=f"{vid}-{algo}",
                        annotations={},
                        metadata={
                            "origin": "lamina rppg-algorithm on skin-mask ROI means",
                            "algorithm": algo,
                        },
                    )
                },
                references=references,
                metadata={
                    "algorithm": algo,
                    "synthetic": True,
                    "roi": "dataset skin_mask (>0.5) on Xsub 240x240 face crop",
                    "fps": fs,
                    "n_frames": int(len(ts)),
                    "timeline_aligned": bool(aligned),
                    "mean_abs_pitch_rad": data["mean_abs_pitch"],
                    "mean_abs_roll_rad": data["mean_abs_roll"],
                    "mean_abs_yaw_rad": data["mean_abs_yaw"],
                },
            )


# ---------------------------------------------------------------------------
# SCAMPS helpers (validation-side)
# ---------------------------------------------------------------------------


def _load_video(path: Path, cache_root: Path) -> dict:
    """Load a SCAMPS .mat example video -> per-frame ROI RGB means + ground truth.

    HDF5 (MATLAB v7.3) layout (dims reversed vs MATLAB):
      Xsub       (3, 240, 240, 600)  -> [channel, col, row, frame], RGB in [0, 1]
      skin_mask  (240, 240, 599)     -> [col, row, frame]
      d_ppg      (600, 1)            ground-truth PPG per frame
    Results are cached as .npz under <cache_root>/.trace_cache/.
    """
    import h5py
    from scipy.signal import butter, filtfilt, find_peaks

    cache_dir = cache_root / ".trace_cache"
    cache_dir.mkdir(parents=True, exist_ok=True)
    cache_file = cache_dir / f"{path.stem}.npz"
    if cache_file.exists():
        z = np.load(cache_file)
        # 0-d arrays (scalars) must come back as Python floats for JSON safety.
        return {k: (float(v) if v.ndim == 0 else v) for k, v in
                ((k, z[k]) for k in z.files)}

    with h5py.File(path, "r") as f:
        xsub = f["Xsub"][:]                       # (3, col, row, frame)
        mask = f["skin_mask"][:] > 0.5            # (col, row, frame)
        d_ppg = f["d_ppg"][:].ravel()
        pitch = np.abs(f["d_pitch"][:].ravel()).mean()
        roll = np.abs(f["d_roll"][:].ravel()).mean()
        yaw = np.abs(f["d_yaw"][:].ravel()).mean()

    nt = min(xsub.shape[3], mask.shape[2], d_ppg.shape[0])
    red = np.empty(nt)
    grn = np.empty(nt)
    blu = np.empty(nt)
    vpc = np.empty(nt, dtype=np.int64)
    for t in range(nt):
        m = mask[:, :, t]
        red[t] = xsub[0, :, :, t][m].mean()
        grn[t] = xsub[1, :, :, t][m].mean()
        blu[t] = xsub[2, :, :, t][m].mean()
        vpc[t] = int(m.sum())

    ts = np.arange(nt) / ScampsAdapter.FPS
    d_ppg = d_ppg[:nt]

    # Reference peaks: validation-side detector on ground-truth PPG.
    nyq = ScampsAdapter.FPS / 2.0
    lo, hi = ScampsAdapter.REF_BANDPASS_HZ
    b, a = butter(3, [lo / nyq, hi / nyq], btype="band")
    ppg_f = filtfilt(b, a, d_ppg)
    gt_peaks, _ = find_peaks(
        ppg_f, distance=int(ScampsAdapter.REF_MIN_PEAK_DIST_SEC * ScampsAdapter.FPS)
    )

    out = {
        "timestamps_sec": ts,
        "red": red,
        "green": grn,
        "blue": blu,
        "valid_pixel_counts": vpc,
        "d_ppg": d_ppg,
        "gt_peak_indices": gt_peaks.astype(np.int64),
        "duration_sec": float(ts[-1]),
        "mean_abs_pitch": float(pitch),
        "mean_abs_roll": float(roll),
        "mean_abs_yaw": float(yaw),
    }
    np.savez(cache_file, **out)
    return out


def _windowed_hr(
    peak_times: np.ndarray, duration_sec: float, window_sec: float, step_sec: float
) -> tuple[np.ndarray, np.ndarray]:
    """Windowed HR (bpm): mean(60/IBI) over IBIs whose midpoint is in-window."""
    peak_times = np.asarray(peak_times, dtype=float)
    centers = np.arange(window_sec / 2.0, duration_sec - window_sec / 2.0 + 1e-9,
                        step_sec)
    hr = np.full(centers.shape, np.nan)
    if len(peak_times) >= 2:
        ibi = np.diff(peak_times)
        mid = (peak_times[:-1] + peak_times[1:]) / 2.0
        inst = 60.0 / ibi
        for i, c in enumerate(centers):
            sel = (mid >= c - window_sec / 2.0) & (mid < c + window_sec / 2.0)
            if sel.any():
                hr[i] = inst[sel].mean()
    return hr, centers


def _mean_hr(peak_times: np.ndarray) -> float | None:
    peak_times = np.asarray(peak_times, dtype=float)
    if len(peak_times) < 2:
        return None
    return float(60.0 / np.mean(np.diff(peak_times)))


def _bandpass_correlation(a: np.ndarray, b: np.ndarray, fs: float) -> float | None:
    """Pearson r between two waveforms after identical 0.75–2.5 Hz bandpass."""
    from scipy.signal import butter, filtfilt

    n = min(len(a), len(b))
    if n < int(fs * 4):  # need a few seconds for a stable estimate
        return None
    a = np.asarray(a, dtype=float)[:n]
    b = np.asarray(b, dtype=float)[:n]
    nyq = fs / 2.0
    lo, hi = ScampsAdapter.REF_BANDPASS_HZ
    bp, ap = butter(3, [lo / nyq, min(hi, nyq * 0.99) / nyq], btype="band")
    try:
        af = filtfilt(bp, ap, a)
        bf = filtfilt(bp, ap, b)
    except ValueError:
        return None
    if np.std(af) == 0 or np.std(bf) == 0:
        return None
    return float(np.corrcoef(af, bf)[0, 1])
