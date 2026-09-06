"""Dataset registry (SPEC §9): the 18 validation-target datasets."""

from __future__ import annotations

from .autonomic import (
    AutonomicAgingAdapter,
    BigIdeasAdapter,
    WearableExamStressAdapter,
    WesadAdapter,
)
from .base import AccessReport, AccessStatus, DatasetAdapter, DatasetInfo, Recording
from .ecg import MitBihArrhythmiaAdapter, MitBihNoiseStressAdapter
from .ppg import (
    BidmcAdapter,
    MimicIiiExtPpgAdapter,
    MimicIiiWaveformAdapter,
    PulseDbAdapter,
    WristPpgExerciseAdapter,
)
from .rppg import (
    CohfaceAdapter,
    IbvpAdapter,
    MmpdAdapter,
    PureAdapter,
    ScampsAdapter,
    UbfcPhysAdapter,
    UbfcRppgAdapter,
)

__all__ = [
    "AccessReport",
    "AccessStatus",
    "DatasetAdapter",
    "DatasetInfo",
    "Recording",
]
