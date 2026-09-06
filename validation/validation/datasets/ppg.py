"""PPG / cardiovascular dataset adapters (Stage 1: honest stubs; Stage 2 TODO)."""

from __future__ import annotations

from .base import AccessStatus, DatasetInfo, StubDatasetAdapter


class BidmcAdapter(StubDatasetAdapter):
    key = "bidmc"

    def info(self) -> DatasetInfo:
        return DatasetInfo(
            key=self.key,
            name="BIDMC PPG and Respiration Dataset",
            category="ppg",
            modalities=["ppg", "rsp", "ecg"],
            source_url="https://physionet.org/content/bidmc/",
            license="ODC-BY 1.0",
            version="1.0.0",
            citation="Pimentel MAF et al. Toward a robust estimation of respiratory rate "
                     "from pulse oximeters. IEEE TBME 64(8):1914-1923 (2017).",
            lamina_ops=["ppg-clean", "ppg-peaks", "rsp-clean", "rsp-cycles", "hrv"],
            notes="53 adult ICU recordings @ 125 Hz, 8 min, with breath annotations. "
                  "Accessible via wfdb without credentials.",
        )


class WristPpgExerciseAdapter(StubDatasetAdapter):
    key = "wrist-ppg-exercise"

    def info(self) -> DatasetInfo:
        return DatasetInfo(
            key=self.key,
            name="Wrist PPG During Exercise",
            category="ppg",
            modalities=["ppg", "ecg", "acc"],
            source_url="https://physionet.org/content/wrist-ppg-during-exercise/",
            license="ODC-BY 1.0",
            version="1.0.0",
            citation="Jarchi D, Casson AJ. Estimation of heart rate from wrist PPG during "
                     "exercise. EMBC (2017).",
            lamina_ops=["ppg-clean", "ppg-peaks", "hrv"],
            notes="8 subjects, wrist PPG @ 256 Hz with chest ECG reference; motion "
                  "conditions. Accessible via wfdb without credentials.",
        )


class PulseDbAdapter(StubDatasetAdapter):
    key = "pulsedb"

    def info(self) -> DatasetInfo:
        return DatasetInfo(
            key=self.key,
            name="PulseDB",
            category="ppg",
            modalities=["ppg", "ecg", "abp"],
            source_url="https://pulsedb.org/ (PhysioNet: https://physionet.org/content/pulsedb/)",
            license="ODC-BY 1.0",
            version="1.0.0",
            citation="Wang W et al. PulseDB: A large, cleaned dataset based on MIMIC-III "
                     "and VitalDB for benchmarking cuff-less blood pressure estimation "
                     "methods. Frontiers (2022).",
            lamina_ops=["ppg-clean", "ppg-peaks"],
            notes="Very large (TB-scale Matlab files); smoke-mode requires the small "
                  "'PulseDB AAMI' subset. Download size may make full runs impractical.",
        )


class MimicIiiWaveformAdapter(StubDatasetAdapter):
    key = "mimic-iii-waveform"
    pending_status = AccessStatus.INACCESSIBLE
    pending_reason = "requires PhysioNet credentialed access (CITI training + signed DUA)"

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
            notes="Credentialed access required; record as inaccessible unless "
                  "credentials are available in the environment.",
        )


class MimicIiiExtPpgAdapter(StubDatasetAdapter):
    key = "mimic-iii-ext-ppg"
    pending_status = AccessStatus.INACCESSIBLE
    pending_reason = "requires PhysioNet credentialed access to the parent MIMIC-III Waveform Database"

    def info(self) -> DatasetInfo:
        return DatasetInfo(
            key=self.key,
            name="MIMIC-III-Ext-PPG",
            category="ppg",
            modalities=["ppg", "abp"],
            source_url="https://physionet.org/content/mimic3wdb/",
            license="PhysioNet Credentialed Health Data License",
            version="1.0",
            citation="Kotzen K et al. PPG and BP waveform features derived from MIMIC-III. "
                     "(2022).",
            lamina_ops=["ppg-clean", "ppg-peaks"],
            notes="Derived PPG/BP dataset from MIMIC-III; same credentialed-access "
                  "barrier as the parent waveform database.",
        )
