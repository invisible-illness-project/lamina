"""Python client for the Rust ``lamina_bridge`` binary (SPEC §3/§4).

The bridge compiles a small Rust bin crate that links against Lamina and
exposes its public API behind a JSON-in/JSON-out protocol. Lamina therefore
remains the implementation under test; Python is orchestration only.
"""

from __future__ import annotations

import json
import os
import shutil
import subprocess
import tempfile
from pathlib import Path
from typing import Any, Sequence

import numpy as np


class LaminaBridgeError(RuntimeError):
    """Raised when the bridge binary fails, cannot build, or reports an error."""

    def __init__(self, message: str, *, kind: str | None = None, op: str | None = None):
        super().__init__(message)
        self.kind = kind
        self.op = op


def default_repo_root() -> Path:
    """Repository root (parent of the ``validation/`` project directory)."""
    return Path(__file__).resolve().parents[2]


class LaminaBridge:
    """Auto-building client for the ``lamina_bridge`` Rust binary."""

    def __init__(
        self,
        repo_root: str | os.PathLike | None = None,
        binary_path: str | os.PathLike | None = None,
        auto_build: bool = True,
        timeout_sec: float = 120.0,
    ):
        self.repo_root = Path(repo_root) if repo_root else default_repo_root()
        self.crate_dir = self.repo_root / "validation" / "lamina_bridge"
        self.binary_path = Path(binary_path) if binary_path else self._default_binary()
        self.timeout_sec = timeout_sec
        if auto_build:
            self.ensure_built()

    # -- build management -------------------------------------------------

    def _default_binary(self) -> Path:
        name = "lamina_bridge.exe" if os.name == "nt" else "lamina_bridge"
        return self.crate_dir / "target" / "release" / name

    def _sources_newer_than_binary(self) -> bool:
        if not self.binary_path.exists():
            return True
        bin_mtime = self.binary_path.stat().st_mtime
        sources = list((self.crate_dir / "src").glob("**/*.rs"))
        sources += [self.crate_dir / "Cargo.toml", self.crate_dir / "build.rs"]
        return any(p.stat().st_mtime > bin_mtime for p in sources if p.exists())

    def ensure_built(self) -> Path:
        """Build the bridge with cargo if missing or stale."""
        if not self._sources_newer_than_binary():
            return self.binary_path
        cargo = shutil.which("cargo")
        if cargo is None:
            raise LaminaBridgeError(
                "cargo not found on PATH; cannot auto-build lamina_bridge. "
                "Install Rust or pass binary_path to a prebuilt binary."
            )
        cmd = [cargo, "build", "--release", "--manifest-path", str(self.crate_dir / "Cargo.toml")]
        proc = subprocess.run(cmd, capture_output=True, text=True, cwd=str(self.crate_dir))
        if proc.returncode != 0:
            raise LaminaBridgeError(
                "cargo build of lamina_bridge failed:\n" + (proc.stderr or proc.stdout)[-4000:]
            )
        if not self.binary_path.exists():
            raise LaminaBridgeError(f"build succeeded but binary missing: {self.binary_path}")
        return self.binary_path

    # -- protocol -----------------------------------------------------------

    def run_op(self, op: str, payload: dict[str, Any] | None = None) -> dict[str, Any]:
        """Run one bridge op; return the parsed output envelope.

        Raises :class:`LaminaBridgeError` on protocol errors (exit 3), Lamina
        errors (exit 1), or panics (exit 2).
        """
        payload = payload or {}
        with tempfile.TemporaryDirectory(prefix="lamina_bridge_") as td:
            in_path = Path(td) / "input.json"
            out_path = Path(td) / "output.json"
            in_path.write_text(json.dumps(payload, default=_json_default))
            cmd = [str(self.binary_path), "--op", op, "--input", str(in_path),
                   "--output", str(out_path)]
            try:
                proc = subprocess.run(
                    cmd, capture_output=True, text=True, timeout=self.timeout_sec
                )
            except subprocess.TimeoutExpired as exc:
                raise LaminaBridgeError(
                    f"bridge op {op!r} timed out after {self.timeout_sec}s", op=op
                ) from exc
            if not out_path.exists():
                raise LaminaBridgeError(
                    f"bridge op {op!r} produced no output file (exit {proc.returncode}): "
                    f"{(proc.stderr or '')[-1000:]}",
                    op=op,
                )
            envelope = json.loads(out_path.read_text())
        if not envelope.get("ok", False):
            err = envelope.get("error") or {}
            raise LaminaBridgeError(
                f"bridge op {op!r} failed [{err.get('kind')}]: {err.get('message')}",
                kind=err.get("kind"),
                op=op,
            )
        return envelope

    def result(self, op: str, payload: dict[str, Any] | None = None) -> dict[str, Any]:
        """Run one op and return only its ``result`` payload."""
        return self.run_op(op, payload)["result"]

    # -- typed wrappers (SPEC §4) --------------------------------------------

    def version(self) -> dict[str, Any]:
        return self.result("version")

    def ecg_clean(self, signal, fs: float, method: str | None = None) -> np.ndarray:
        cfg = {} if method is None else {"method": method}
        return np.asarray(self.result("ecg-clean", _sig(signal, fs, cfg))["signal"], dtype=float)

    def ecg_peaks(self, signal, fs: float, config: dict | None = None) -> dict[str, Any]:
        return self.result("ecg-peaks", _sig(signal, fs, config))

    def ppg_clean(self, signal, fs: float) -> np.ndarray:
        return np.asarray(self.result("ppg-clean", _sig(signal, fs))["signal"], dtype=float)

    def ppg_peaks(self, signal, fs: float, config: dict | None = None) -> dict[str, Any]:
        return self.result("ppg-peaks", _sig(signal, fs, config))

    def eda_clean(self, signal, fs: float) -> np.ndarray:
        return np.asarray(self.result("eda-clean", _sig(signal, fs))["signal"], dtype=float)

    def eda_decompose(self, signal, fs: float, config: dict | None = None) -> dict[str, Any]:
        r = self.result("eda-decompose", _sig(signal, fs, config))
        return {"tonic": np.asarray(r["tonic"], dtype=float),
                "phasic": np.asarray(r["phasic"], dtype=float)}

    def eda_peaks(self, signal, fs: float, config: dict | None = None) -> dict[str, Any]:
        return self.result("eda-peaks", _sig(signal, fs, config))

    def rsp_clean(self, signal, fs: float, config: dict | None = None) -> np.ndarray:
        return np.asarray(self.result("rsp-clean", _sig(signal, fs, config))["signal"], dtype=float)

    def rsp_cycles(self, signal, fs: float, config: dict | None = None) -> dict[str, Any]:
        return self.result("rsp-cycles", _sig(signal, fs, config))

    def hrv(self, peaks: Sequence[int], signal_length: int, fs: float) -> dict[str, Any]:
        return self.result("hrv", {
            "peaks": [int(p) for p in peaks],
            "signal_length": int(signal_length),
            "sampling_rate": float(fs),
        })

    def signal_peaks(self, signal, config: dict | None = None) -> dict[str, Any]:
        return self.result("signal-peaks", _sig(signal, None, config))

    def filter(self, signal, fs: float, kind: str, cutoff=None, cutoffs=None,
               order: int | None = None, zero_phase: bool | None = None) -> np.ndarray:
        cfg: dict[str, Any] = {"kind": kind}
        if cutoff is not None:
            cfg["cutoff"] = float(cutoff)
        if cutoffs is not None:
            cfg["cutoffs"] = [float(c) for c in cutoffs]
        if order is not None:
            cfg["order"] = int(order)
        if zero_phase is not None:
            cfg["zero_phase"] = bool(zero_phase)
        return np.asarray(self.result("filter", _sig(signal, fs, cfg))["signal"], dtype=float)

    def sample_entropy(self, signal, m: int | None = None, r: float | None = None) -> dict[str, Any]:
        cfg: dict[str, Any] = {}
        if m is not None:
            cfg["m"] = int(m)
        if r is not None:
            cfg["r"] = float(r)
        return self.result("sample-entropy", _sig(signal, None, cfg))

    def rppg_algorithm(self, timestamps_sec, red, green, blue,
                       valid_pixel_counts=None, config: dict | None = None) -> dict[str, Any]:
        payload: dict[str, Any] = {
            "timestamps_sec": _tolist(timestamps_sec),
            "red": _tolist(red),
            "green": _tolist(green),
            "blue": _tolist(blue),
        }
        if valid_pixel_counts is not None:
            payload["valid_pixel_counts"] = [int(v) for v in valid_pixel_counts]
        if config:
            payload["config"] = config
        return self.result("rppg-algorithm", payload)


def _tolist(x) -> list:
    return np.asarray(x, dtype=float).tolist()


def _sig(signal, fs: float | None, config: dict | None = None) -> dict[str, Any]:
    payload: dict[str, Any] = {"signal": _tolist(signal)}
    if fs is not None:
        payload["sampling_rate"] = float(fs)
    if config:
        payload["config"] = config
    return payload


def _json_default(o):
    if isinstance(o, np.ndarray):
        return o.tolist()
    if isinstance(o, (np.floating, np.integer)):
        return o.item()
    raise TypeError(f"object of type {type(o).__name__} is not JSON serializable")
