"""Tests for CLI behavior (SPEC §7/§8)."""

import json
from pathlib import Path

import numpy as np
import pytest

from validation.__main__ import main
from validation.report import generate

from conftest import cargo_available


def test_cli_list_shows_18_datasets(capsys):
    assert main(["list"]) == 0
    out = capsys.readouterr().out
    assert "18 dataset(s) registered." in out
    for key in ("mit-bih-arrhythmia", "bidmc", "wesad", "pure", "scamps"):
        assert key in out


def test_cli_list_category_filter(capsys):
    assert main(["list", "--category", "ecg"]) == 0
    out = capsys.readouterr().out
    assert "mit-bih-arrhythmia" in out
    assert "wesad" not in out
    assert "2 dataset(s) registered." in out


def test_cli_check_access_runs_and_exits_zero(capsys):
    assert main(["check-access"]) == 0
    out = capsys.readouterr().out
    assert out.count("[") == 18
    assert "not_attempted" in out


def test_cli_check_access_single_dataset(capsys):
    assert main(["check-access", "--dataset", "mimic-iii-waveform"]) == 0
    out = capsys.readouterr().out
    assert "inaccessible" in out  # known credentialed-access barrier


@pytest.mark.skipif(not cargo_available(), reason="cargo unavailable")
def test_cli_run_records_stub_statuses(tmp_path, capsys):
    code = main(["run", "--dataset", "mit-bih-arrhythmia",
                 "--results-dir", str(tmp_path)])
    # Stub adapter is not_attempted -> not a "bad" status for exit-code purposes.
    assert code == 0
    out = capsys.readouterr().out
    assert "not_attempted" in out
    assert (tmp_path / "summary.csv").exists()
    manifest = json.loads((tmp_path / "manifest.json").read_text())
    assert manifest["lamina_git_commit"]
    assert manifest["seed"] == 0


@pytest.mark.skipif(not cargo_available(), reason="cargo unavailable")
def test_cli_report_generates_skeleton(tmp_path):
    out = tmp_path / "report.md"
    assert main(["report", "--results-dir", str(tmp_path), "--output", str(out)]) == 0
    text = out.read_text()
    for section in ("Executive Summary", "Dataset Inventory", "Methodology",
                    "Results", "Robustness", "Failures", "Known/Potential Bugs",
                    "Limitations", "Reproducibility"):
        assert f"## {section}" in text
    assert text.count("| mit-") + text.count("| ") > 0
    assert "| mit-bih-arrhythmia |" in text


def test_report_falls_back_to_registry(tmp_path):
    out = generate(tmp_path / "no-results", tmp_path / "r.md")
    text = out.read_text()
    assert "not_attempted" in text
    assert text.count("not_attempted") >= 18
