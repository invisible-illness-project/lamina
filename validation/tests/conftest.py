"""Shared fixtures/config for the framework test suite (SPEC §8)."""

import shutil
from pathlib import Path

import numpy as np
import pytest

REPO_ROOT = Path(__file__).resolve().parents[2]
FIXTURES_DIR = REPO_ROOT / "validation" / "fixtures"


@pytest.fixture(scope="session")
def fixtures_dir() -> Path:
    return FIXTURES_DIR


@pytest.fixture(scope="session")
def ecg_fixture() -> dict:
    with np.load(FIXTURES_DIR / "ecg_synthetic.npz", allow_pickle=False) as z:
        return {"signal": z["signal"], "fs": float(z["fs"]),
                "true_peaks": z["true_peaks"]}


@pytest.fixture(scope="session")
def ppg_fixture() -> dict:
    with np.load(FIXTURES_DIR / "ppg_synthetic.npz", allow_pickle=False) as z:
        return {"signal": z["signal"], "fs": float(z["fs"]),
                "true_peaks": z["true_peaks"]}


@pytest.fixture(scope="session")
def rsp_fixture() -> dict:
    with np.load(FIXTURES_DIR / "rsp_synthetic.npz", allow_pickle=False) as z:
        return {"signal": z["signal"], "fs": float(z["fs"]),
                "true_peaks": z["true_peaks"]}


@pytest.fixture(scope="session")
def eda_fixture() -> dict:
    with np.load(FIXTURES_DIR / "eda_synthetic.npz", allow_pickle=False) as z:
        return {"signal": z["signal"], "fs": float(z["fs"]),
                "true_onsets": z["true_onsets"], "true_peaks": z["true_peaks"]}


def cargo_available() -> bool:
    return shutil.which("cargo") is not None


@pytest.fixture(scope="session")
def bridge():
    """Session-scoped bridge; builds the Rust binary once via cargo."""
    if not cargo_available():
        pytest.skip("cargo not available; cannot build lamina_bridge")
    from validation.bridge import LaminaBridge

    return LaminaBridge(repo_root=REPO_ROOT)
