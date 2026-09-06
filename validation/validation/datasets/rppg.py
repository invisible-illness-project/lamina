"""rPPG dataset adapters (Stage 1: honest stubs; Stage 2 TODO)."""

from __future__ import annotations

from .base import DatasetInfo, StubDatasetAdapter


class PureAdapter(StubDatasetAdapter):
    key = "pure"

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


class UbfcRppgAdapter(StubDatasetAdapter):
    key = "ubfc-rppg"

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


class CohfaceAdapter(StubDatasetAdapter):
    key = "cohface"

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


class UbfcPhysAdapter(StubDatasetAdapter):
    key = "ubfc-phys"

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


class MmpdAdapter(StubDatasetAdapter):
    key = "mmpd"

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


class IbvpAdapter(StubDatasetAdapter):
    key = "ibvp"

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


class ScampsAdapter(StubDatasetAdapter):
    key = "scamps"

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
                  "PPG waveforms; useful as a controlled synthetic reference.",
        )
