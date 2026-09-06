"""Tests for the 18-dataset registry and honest stub adapters (SPEC §8/§9)."""

import pytest

from validation.datasets.base import AccessStatus
from validation.registry import DATASET_CLASSES, get_dataset, list_datasets


def test_registry_has_18_datasets():
    assert len(DATASET_CLASSES) == 18


def test_registry_keys_unique_and_match_adapter_key():
    for key, cls in DATASET_CLASSES.items():
        adapter = cls()
        assert adapter.key == key
        assert adapter.info().key == key


def test_registry_categories():
    cats = {}
    for a in list_datasets():
        cats.setdefault(a.info().category, []).append(a.key)
    assert cats == {
        "ecg": ["mit-bih-arrhythmia", "mit-bih-noise-stress"],
        "ppg": ["bidmc", "wrist-ppg-exercise", "pulsedb", "mimic-iii-waveform",
                "mimic-iii-ext-ppg"],
        "autonomic": ["wesad", "autonomic-aging", "wearable-exam-stress", "big-ideas"],
        "rppg": ["pure", "ubfc-rppg", "cohface", "ubfc-phys", "mmpd", "ibvp", "scamps"],
    }


def test_info_fields_populated():
    for a in list_datasets():
        info = a.info()
        assert info.name and info.source_url.startswith("http")
        assert info.modalities, info.key
        assert info.lamina_ops, info.key
        assert info.category in ("ecg", "ppg", "autonomic", "rppg")


def test_stub_check_access_is_honest_placeholder():
    allowed = {AccessStatus.NOT_ATTEMPTED, AccessStatus.INACCESSIBLE}
    for a in list_datasets():
        report = a.check_access()
        assert report.status in allowed
        assert report.reason  # never an empty reason
        assert report.checked_at


def test_stub_iter_recordings_raises_not_implemented():
    for a in list_datasets():
        with pytest.raises(NotImplementedError):
            next(a.iter_recordings())


def test_get_dataset_unknown_key():
    with pytest.raises(KeyError):
        get_dataset("no-such-dataset")


def test_category_filter():
    ecg = list_datasets(category="ecg")
    assert {a.key for a in ecg} == {"mit-bih-arrhythmia", "mit-bih-noise-stress"}
