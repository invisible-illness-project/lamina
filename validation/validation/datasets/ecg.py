"""ECG dataset adapters (Stage 1: honest stubs; Stage 2 TODO: implement)."""

from __future__ import annotations

from .base import DatasetInfo, StubDatasetAdapter


class MitBihArrhythmiaAdapter(StubDatasetAdapter):
    key = "mit-bih-arrhythmia"

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
                  "Accessible via wfdb without credentials.",
        )


class MitBihNoiseStressAdapter(StubDatasetAdapter):
    key = "mit-bih-noise-stress"

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
                  "@ 360 Hz; robustness stratification target. Accessible via wfdb.",
        )
