"""Reproducibility manifest (task §10, SPEC §6).

Answers: exactly which code, data, parameters, and preprocessing produced a
result?
"""

from __future__ import annotations

import json
import platform
import subprocess
from datetime import datetime, timezone
from importlib.metadata import PackageNotFoundError
from importlib.metadata import version as pkg_version
from pathlib import Path
from typing import Any

from ._version import __version__ as FRAMEWORK_VERSION

#: Python packages pinned into every manifest when installed.
TRACKED_PACKAGES = ("numpy", "scipy", "pandas", "wfdb", "matplotlib", "pytest")


def git_commit(repo_root: str | Path) -> str:
    """Lamina git commit under test (`git rev-parse HEAD`)."""
    try:
        proc = subprocess.run(
            ["git", "rev-parse", "HEAD"],
            capture_output=True, text=True, cwd=str(repo_root), check=True,
        )
        return proc.stdout.strip()
    except Exception:
        return "unknown"


def git_branch(repo_root: str | Path) -> str:
    try:
        proc = subprocess.run(
            ["git", "rev-parse", "--abbrev-ref", "HEAD"],
            capture_output=True, text=True, cwd=str(repo_root), check=True,
        )
        return proc.stdout.strip()
    except Exception:
        return "unknown"


def package_versions() -> dict[str, str | None]:
    """Installed versions of tracked packages via importlib.metadata."""
    out: dict[str, str | None] = {}
    for name in TRACKED_PACKAGES:
        try:
            out[name] = pkg_version(name)
        except PackageNotFoundError:
            out[name] = None
    return out


def build_manifest(
    repo_root: str | Path,
    *,
    seed: int = 0,
    tolerances: dict[str, float] | None = None,
    config: dict[str, Any] | None = None,
    lamina_version: str | None = None,
) -> dict[str, Any]:
    """Assemble the reproducibility manifest."""
    repo_root = Path(repo_root)
    return {
        "created_utc": datetime.now(timezone.utc).isoformat(timespec="seconds"),
        "lamina_git_commit": git_commit(repo_root),
        "lamina_git_branch": git_branch(repo_root),
        "lamina_version": lamina_version or _detect_lamina_version(repo_root),
        "framework_version": FRAMEWORK_VERSION,
        "python_version": platform.python_version(),
        "package_versions": package_versions(),
        "seed": seed,
        "tolerances": tolerances or {},
        "config_snapshot": config or {},
    }


def write_manifest(path: str | Path, manifest: dict[str, Any]) -> None:
    Path(path).parent.mkdir(parents=True, exist_ok=True)
    Path(path).write_text(json.dumps(manifest, indent=2, default=str))


def _detect_lamina_version(repo_root: Path) -> str:
    """Parse the Lamina crate version from the root Cargo.toml [package] section."""
    try:
        in_package = False
        for line in (repo_root / "Cargo.toml").read_text().splitlines():
            line = line.strip()
            if line.startswith("["):
                in_package = line == "[package]"
                continue
            if in_package and line.startswith("version"):
                return line.split("=", 1)[1].strip().strip('"')
    except Exception:
        pass
    return "unknown"
