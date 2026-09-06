"""Dataset registry (SPEC §9).

18 validation-target datasets. Stage 1 ships stub adapters with real metadata
and honest ``not_attempted``/``inaccessible`` access statuses; Stage 2 adapter
agents replace the stub classes without touching this registry's structure.
"""

from __future__ import annotations

from .datasets import DatasetAdapter
from .datasets.autonomic import (
    AutonomicAgingAdapter,
    BigIdeasAdapter,
    WearableExamStressAdapter,
    WesadAdapter,
)
from .datasets.ecg import MitBihArrhythmiaAdapter, MitBihNoiseStressAdapter
from .datasets.ppg import (
    BidmcAdapter,
    MimicIiiExtPpgAdapter,
    MimicIiiWaveformAdapter,
    PulseDbAdapter,
    WristPpgExerciseAdapter,
)
from .datasets.rppg import (
    CohfaceAdapter,
    IbvpAdapter,
    MmpdAdapter,
    PureAdapter,
    ScampsAdapter,
    UbfcPhysAdapter,
    UbfcRppgAdapter,
)

#: Registry key -> adapter class. Insertion order is the canonical list order.
DATASET_CLASSES: dict[str, type[DatasetAdapter]] = {
    # ECG
    "mit-bih-arrhythmia": MitBihArrhythmiaAdapter,
    "mit-bih-noise-stress": MitBihNoiseStressAdapter,
    # PPG / cardiovascular
    "bidmc": BidmcAdapter,
    "wrist-ppg-exercise": WristPpgExerciseAdapter,
    "pulsedb": PulseDbAdapter,
    "mimic-iii-waveform": MimicIiiWaveformAdapter,
    "mimic-iii-ext-ppg": MimicIiiExtPpgAdapter,
    # Autonomic / wearable
    "wesad": WesadAdapter,
    "autonomic-aging": AutonomicAgingAdapter,
    "wearable-exam-stress": WearableExamStressAdapter,
    "big-ideas": BigIdeasAdapter,
    # rPPG
    "pure": PureAdapter,
    "ubfc-rppg": UbfcRppgAdapter,
    "cohface": CohfaceAdapter,
    "ubfc-phys": UbfcPhysAdapter,
    "mmpd": MmpdAdapter,
    "ibvp": IbvpAdapter,
    "scamps": ScampsAdapter,
}


def list_datasets(category: str | None = None) -> list[DatasetAdapter]:
    """Instantiate all registered adapters (optionally filtered by category)."""
    adapters = [cls() for cls in DATASET_CLASSES.values()]
    if category is not None:
        adapters = [a for a in adapters if a.info().category == category]
    return adapters


def get_dataset(key: str) -> DatasetAdapter:
    """Instantiate one adapter by registry key."""
    if key not in DATASET_CLASSES:
        raise KeyError(
            f"unknown dataset {key!r}; known: {sorted(DATASET_CLASSES)}"
        )
    return DATASET_CLASSES[key]()
