"""Tests for the reproducibility manifest (task §10, SPEC §8)."""

import json
import subprocess
from pathlib import Path

from validation.manifest import build_manifest, package_versions, write_manifest

REPO_ROOT = Path(__file__).resolve().parents[2]


def test_manifest_contains_commit_versions_seed(tmp_path):
    m = build_manifest(REPO_ROOT, seed=42,
                       tolerances={"peak_match_sec": 0.150},
                       config={"dataset": "all", "smoke": True})
    expected = subprocess.run(
        ["git", "rev-parse", "HEAD"], capture_output=True, text=True,
        cwd=REPO_ROOT, check=True,
    ).stdout.strip()
    assert m["lamina_git_commit"] == expected
    assert m["seed"] == 42
    assert m["tolerances"]["peak_match_sec"] == 0.150
    assert m["config_snapshot"]["smoke"] is True
    assert m["lamina_version"] != "unknown"
    assert m["framework_version"]
    assert m["python_version"]
    assert m["package_versions"]["numpy"] is not None

    path = tmp_path / "manifest.json"
    write_manifest(path, m)
    loaded = json.loads(path.read_text())
    assert loaded["lamina_git_commit"] == expected


def test_package_versions_missing_package_is_null():
    versions = package_versions()
    for name in ("numpy", "scipy", "pandas", "wfdb", "matplotlib", "pytest"):
        assert name in versions
