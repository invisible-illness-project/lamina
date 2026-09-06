"""Autonomic / wearable dataset adapters (Stage 1: honest stubs; Stage 2 TODO)."""

from __future__ import annotations

from .base import DatasetInfo, StubDatasetAdapter


class WesadAdapter(StubDatasetAdapter):
    key = "wesad"

    def info(self) -> DatasetInfo:
        return DatasetInfo(
            key=self.key,
            name="WESAD (Wearable Stress and Affect Detection)",
            category="autonomic",
            modalities=["ecg", "eda", "ppg", "rsp", "temp", "acc"],
            source_url="https://ubicomp.eti.uni-siegen.de/home/datasets/icmi18/",
            license="MIT (dataset)",
            version="1.0",
            citation="Schmidt P et al. Introducing WESAD, a multimodal dataset for "
                     "wearable stress and affect detection. ICMI (2018).",
            lamina_ops=["ecg-clean", "ecg-peaks", "ppg-clean", "ppg-peaks", "eda-clean",
                        "eda-decompose", "eda-peaks", "rsp-clean", "rsp-cycles", "hrv"],
            notes="15 subjects, chest (RespiBAN 700 Hz) + wrist (Empatica E4) devices; "
                  "~6.5 GB download. EDA at 4 Hz (wrist) is below typical Lamina filter "
                  "assumptions — document preprocessing.",
        )


class AutonomicAgingAdapter(StubDatasetAdapter):
    key = "autonomic-aging"

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
            lamina_ops=["ecg-clean", "ecg-peaks", "hrv"],
            notes="~1100 subjects, short resting ECG @ 1000 Hz with annotations; HRV "
                  "reference values per age group. Accessible via wfdb.",
        )


class WearableExamStressAdapter(StubDatasetAdapter):
    key = "wearable-exam-stress"

    def info(self) -> DatasetInfo:
        return DatasetInfo(
            key=self.key,
            name="Wearable Exam Stress Dataset",
            category="autonomic",
            modalities=["eda", "temp", "acc", "bvp", "ecg"],
            source_url="https://physionet.org/content/wearable-exam-stress/",
            license="ODC-BY 1.0",
            version="1.0.0",
            citation="Amin MR et al. Exam stress measurement using wearable sensors. "
                     "PhysioNet (2021).",
            lamina_ops=["eda-clean", "eda-decompose", "eda-peaks", "ecg-clean",
                        "ecg-peaks", "hrv"],
            notes="10 subjects, Empatica E4 (EDA/BVP/temp/acc) + chest ECG during three "
                  "exam periods. EDA @ 4 Hz — document low-rate handling.",
        )


class BigIdeasAdapter(StubDatasetAdapter):
    key = "big-ideas"

    def info(self) -> DatasetInfo:
        return DatasetInfo(
            key=self.key,
            name="BIG IDEAs Wearable/Glycemic Dataset",
            category="autonomic",
            modalities=["acc", "bvp", "eda", "temp", "ecg"],
            source_url="https://bigideaslab.org/ (PhysioNet: https://physionet.org/content/big-ideas-glycemic-wearable/)",
            license="PhysioNet Credentialed Health Data License",
            version="1.1.2",
            citation="Bent B et al. The BIG IDEAs Glycemic Wearable dataset. PhysioNet (2024).",
            lamina_ops=["ppg-clean", "ppg-peaks", "eda-clean", "eda-peaks", "hrv"],
            notes="Wearable (Empatica E4/CGM) data with glycemic labels; PhysioNet copy "
                  "requires credentialed access.",
        )
