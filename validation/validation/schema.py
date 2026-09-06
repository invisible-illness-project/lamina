"""Canonical signal representation for the validation framework (SPEC §2).

Dataset adapters convert their native formats into :class:`Signal` so that
validation logic (metrics, runners, bridge calls) never depends on a specific
dataset format.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from fractions import Fraction

import numpy as np

#: Supported modalities (SPEC §2).
MODALITIES = (
    "ecg",
    "ppg",
    "bvp",
    "eda",
    "rsp",
    "acc",
    "gyro",
    "temp",
    "abp",
    "rgb_video",
    "thermal_video",
)


class SignalValidationError(ValueError):
    """Raised when a Signal fails validation."""


@dataclass
class Signal:
    """Canonical 1-D physiological signal.

    Attributes
    ----------
    samples:
        1-D sample values (converted to float64).
    sampling_rate:
        Sampling rate in Hz (must be positive and finite).
    modality:
        One of :data:`MODALITIES`.
    timestamps:
        Optional per-sample timestamps in seconds. ``None`` means a uniform
        grid starting at t=0 with step ``1 / sampling_rate``.
    units, subject_id, recording_id, channel:
        Optional provenance metadata.
    annotations:
        Dataset-provided reference data, e.g. ``{"peak_indices": np.ndarray}``.
    metadata:
        Free-form extra metadata (never used by validation logic directly).
    """

    samples: np.ndarray
    sampling_rate: float
    modality: str
    timestamps: np.ndarray | None = None
    units: str | None = None
    subject_id: str | None = None
    recording_id: str | None = None
    channel: str | None = None
    annotations: dict = field(default_factory=dict)
    metadata: dict = field(default_factory=dict)

    def __post_init__(self) -> None:
        self.samples = np.asarray(self.samples, dtype=np.float64)
        if self.timestamps is not None:
            self.timestamps = np.asarray(self.timestamps, dtype=np.float64)

    # -- helpers --------------------------------------------------------

    @property
    def n_samples(self) -> int:
        return int(self.samples.shape[0])

    @property
    def duration_sec(self) -> float:
        return self.n_samples / self.sampling_rate

    def get_timestamps(self) -> np.ndarray:
        """Per-sample timestamps in seconds (uniform grid when absent)."""
        if self.timestamps is not None:
            return self.timestamps
        return np.arange(self.n_samples, dtype=np.float64) / self.sampling_rate

    def validate(self) -> "Signal":
        """Raise :class:`SignalValidationError` on inconsistency; return self."""
        if self.samples.ndim != 1:
            raise SignalValidationError("samples must be 1-D")
        if self.n_samples == 0:
            raise SignalValidationError("samples must not be empty")
        if not np.isfinite(self.sampling_rate) or self.sampling_rate <= 0:
            raise SignalValidationError(f"invalid sampling_rate: {self.sampling_rate}")
        if self.modality not in MODALITIES:
            raise SignalValidationError(
                f"unknown modality {self.modality!r}; expected one of {MODALITIES}"
            )
        if self.timestamps is not None:
            if self.timestamps.shape != self.samples.shape:
                raise SignalValidationError(
                    "timestamps must have the same length as samples "
                    f"({self.timestamps.shape} vs {self.samples.shape})"
                )
            if np.any(np.diff(self.timestamps) < 0):
                raise SignalValidationError("timestamps must be non-decreasing")
        return self


def resample_signal(signal: Signal, target_fs: float) -> Signal:
    """Polyphase-resample ``signal`` to ``target_fs`` Hz (scipy ``resample_poly``).

    The up/down ratio is reduced to a rational approximation. Timestamps are
    regenerated on a uniform grid (the resampler assumes uniform sampling).
    """
    signal.validate()
    if not np.isfinite(target_fs) or target_fs <= 0:
        raise SignalValidationError(f"invalid target_fs: {target_fs}")
    if float(target_fs) == float(signal.sampling_rate):
        return signal
    from scipy.signal import resample_poly

    frac = Fraction(target_fs / signal.sampling_rate).limit_denominator(10_000)
    up, down = frac.numerator, frac.denominator
    new_samples = resample_poly(signal.samples, up, down)
    new_fs = signal.sampling_rate * up / down
    return Signal(
        samples=new_samples,
        sampling_rate=new_fs,
        modality=signal.modality,
        timestamps=np.arange(new_samples.shape[0], dtype=np.float64) / new_fs,
        units=signal.units,
        subject_id=signal.subject_id,
        recording_id=signal.recording_id,
        channel=signal.channel,
        annotations=dict(signal.annotations),
        metadata={
            **signal.metadata,
            "resampled_from_hz": signal.sampling_rate,
            "resample_ratio": f"{up}/{down}",
        },
    )
