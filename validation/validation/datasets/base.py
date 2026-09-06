"""Dataset adapter contract (SPEC §9). **This file is complete and final.**

Stage 2 adapter agents implement subclasses of :class:`DatasetAdapter` in
sibling modules; they must not modify this file.
"""

from __future__ import annotations

import enum
from dataclasses import dataclass, field
from datetime import datetime, timezone
from typing import Iterator

from ..schema import Signal


class AccessStatus(str, enum.Enum):
    """Dataset validation/access statuses (task §3)."""

    VALIDATED = "validated"
    PARTIALLY_VALIDATED = "partially_validated"
    INACCESSIBLE = "inaccessible"
    UNSUPPORTED_FORMAT = "unsupported_format"
    FAILED = "failed"
    NOT_ATTEMPTED = "not_attempted"


@dataclass
class DatasetInfo:
    """Static metadata describing a dataset and its Lamina coverage."""

    key: str                     # unique registry key, e.g. "mit-bih-arrhythmia"
    name: str                    # human-readable name
    category: str                # "ecg" | "ppg" | "autonomic" | "rppg"
    modalities: list[str]        # schema.MODALITIES values present in the dataset
    source_url: str              # canonical landing page / download location
    license: str = "unknown"
    version: str = "unknown"
    citation: str = ""
    lamina_ops: list[str] = field(default_factory=list)  # bridge ops exercised (SPEC §3.2)
    notes: str = ""              # access requirements, caveats, ground-truth notes


@dataclass
class AccessReport:
    """Result of an access attempt (honest; never fabricated)."""

    status: AccessStatus
    reason: str = ""
    detail: str = ""
    checked_at: str = field(
        default_factory=lambda: datetime.now(timezone.utc).isoformat(timespec="seconds")
    )


@dataclass
class Recording:
    """One recording/unit of analysis yielded by an adapter.

    ``signals`` maps channel name -> canonical :class:`Signal`.
    ``references`` carries dataset-provided ground truth, e.g.
    ``{"ecg_peak_indices": np.ndarray, "ecg_peak_fs": 360.0, "hr_bpm": ...}``.
    """

    recording_id: str
    subject_id: str
    signals: dict[str, Signal] = field(default_factory=dict)
    references: dict = field(default_factory=dict)
    metadata: dict = field(default_factory=dict)


class DatasetAdapter:
    """Base class for dataset adapters.

    Subclasses MUST set ``info()`` with real metadata and implement
    ``check_access()`` honestly. ``iter_recordings()`` is only called after a
    successful access check.
    """

    key: str = ""

    def info(self) -> DatasetInfo:  # pragma: no cover - abstract
        raise NotImplementedError

    def check_access(self, cache_dir: str | None = None) -> AccessReport:
        """Attempt/assess access. MUST NOT fabricate availability."""
        raise NotImplementedError

    def iter_recordings(
        self,
        subjects: list[str] | None = None,
        recordings: list[str] | None = None,
        max_recordings: int | None = None,
        seed: int = 0,
        smoke: bool = False,
        cache_dir: str | None = None,
    ) -> Iterator[Recording]:
        """Yield recordings with deterministic subset selection.

        Implementations must apply ``subjects``/``recordings`` filters and, when
        ``max_recordings`` is set, select a deterministic subset seeded by
        ``seed``. ``smoke=True`` selects a small fast deterministic subset.
        """
        raise NotImplementedError

    # Convenience ------------------------------------------------------------

    def __repr__(self) -> str:
        return f"<{type(self).__name__} key={self.key!r}>"


class StubDatasetAdapter(DatasetAdapter):
    """Honest placeholder adapter (SPEC §9): metadata is real, but the adapter
    implementation itself is pending (Stage 2 TODO).

    ``check_access()`` reports ``not_attempted`` (or ``inaccessible`` when the
    dataset is known a priori to require credentials/approval) and
    ``iter_recordings()`` always raises.
    """

    #: Override in subclasses for datasets with known access barriers.
    pending_status: AccessStatus = AccessStatus.NOT_ATTEMPTED
    pending_reason: str = "adapter not yet implemented (Stage 2 TODO)"

    def info(self) -> DatasetInfo:
        """Placeholder metadata; subclasses override with real metadata."""
        return DatasetInfo(
            key=self.key or "stub",
            name=f"{self.key or 'stub'} (stub adapter)",
            category="unknown",
            modalities=[],
            source_url="",
            notes="stub adapter — Stage 2 TODO",
        )

    def check_access(self, cache_dir: str | None = None) -> AccessReport:
        return AccessReport(
            status=self.pending_status,
            reason=self.pending_reason,
            detail="stub adapter: no acquisition attempted",
        )

    def iter_recordings(self, **kwargs) -> Iterator[Recording]:
        raise NotImplementedError(
            f"adapter for {self.key!r} not yet implemented (Stage 2 TODO)"
        )
