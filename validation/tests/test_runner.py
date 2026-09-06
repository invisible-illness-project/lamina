"""Tests for the runner: per-recording isolation and result writers (SPEC §6/§8)."""

import csv
import json
from pathlib import Path

import numpy as np
import pytest

from validation.datasets.base import (
    AccessReport,
    AccessStatus,
    DatasetInfo,
    Recording,
    StubDatasetAdapter,
)
from validation.runner import RunOptions, run_dataset, write_all
from validation.schema import Signal

from conftest import FIXTURES_DIR, cargo_available


class _FixtureEcgAdapter(StubDatasetAdapter):
    """In-test adapter yielding one real fixture recording + one broken one."""

    key = "fixture-ecg"

    def info(self) -> DatasetInfo:
        return DatasetInfo(
            key=self.key, name="Fixture ECG", category="ecg", modalities=["ecg"],
            source_url="https://example.invalid/fixture", lamina_ops=["ecg-peaks"],
        )

    def check_access(self, cache_dir=None) -> AccessReport:
        return AccessReport(status=AccessStatus.VALIDATED, reason="fixture")

    def iter_recordings(self, **kwargs):
        with np.load(FIXTURES_DIR / "ecg_synthetic.npz") as z:
            sig = Signal(samples=z["signal"], sampling_rate=float(z["fs"]),
                         modality="ecg", subject_id="sub-001",
                         recording_id="rec-001")
            yield Recording(
                recording_id="rec-001", subject_id="sub-001",
                signals={"ecg": sig},
                references={"ecg_peak_indices": z["true_peaks"]},
            )
        # Broken recording: 3-sample signal triggers a Lamina error; the run
        # must isolate the failure and continue.
        bad = Signal(samples=np.array([1.0, 2.0, 3.0]), sampling_rate=360.0,
                     modality="ecg", subject_id="sub-001", recording_id="rec-bad")
        yield Recording(recording_id="rec-bad", subject_id="sub-001",
                        signals={"ecg": bad})


@pytest.mark.skipif(not cargo_available(), reason="cargo unavailable")
def test_run_dataset_isolates_recording_failures(tmp_path):
    result = run_dataset(_FixtureEcgAdapter(), options=RunOptions(),
                         results_dir=tmp_path)
    assert result.status == AccessStatus.PARTIALLY_VALIDATED.value
    assert result.n_ok == 1 and result.n_failed == 1
    ok = next(r for r in result.recordings if r.status == "ok")
    # Real metric computed against the fixture's known peak train.
    assert ok.metrics["ecg_ecg_recall"] > 0.8
    bad = next(r for r in result.recordings if r.status == "failed")
    assert bad.error
    assert (tmp_path / "logs" / "fixture-ecg.log").exists()


def test_run_dataset_records_inaccessible_without_iterating(tmp_path):
    adapter = StubDatasetAdapter()
    adapter.key = "stub-x"
    result = run_dataset(adapter, results_dir=tmp_path)
    assert result.status == AccessStatus.NOT_ATTEMPTED.value
    assert result.recordings == []
    assert "Stage 2" in result.reason


@pytest.mark.skipif(not cargo_available(), reason="cargo unavailable")
def test_writers_emit_machine_readable_results(tmp_path):
    results = [run_dataset(_FixtureEcgAdapter(), options=RunOptions(),
                           results_dir=tmp_path)]
    write_all(tmp_path, results)
    for name in ("recordings.csv", "metrics.csv", "summary.csv", "datasets.json"):
        assert (tmp_path / name).exists(), name

    with open(tmp_path / "recordings.csv") as fh:
        rows = list(csv.DictReader(fh))
    assert {r["status"] for r in rows} == {"ok", "failed"}

    with open(tmp_path / "metrics.csv") as fh:
        mrows = list(csv.DictReader(fh))
    assert any(r["metric"].endswith("_f1") for r in mrows)

    with open(tmp_path / "summary.csv") as fh:
        srows = list(csv.DictReader(fh))
    assert srows[0]["status"] == "partially_validated"

    datasets = json.loads((tmp_path / "datasets.json").read_text())
    entry = datasets[0]
    assert entry["dataset"] == "fixture-ecg"
    assert entry["recordings_evaluated"] == 1
    assert entry["git_commit"] != ""
    assert entry["failures"]  # the broken recording's error preserved
