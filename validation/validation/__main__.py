"""Validation framework CLI (SPEC §7).

    python -m validation list [--category ...]
    python -m validation check-access [--dataset KEY]
    python -m validation run --dataset KEY|all [--smoke] [--subjects ...] \
        [--recordings ...] [--max-recordings N] [--seed N] [--results-dir DIR]
    python -m validation report [--results-dir DIR] [--output PATH]
    python -m validation test [pytest-args...]

Exit codes: 0 success · 1 any dataset failed/inaccessible during ``run`` ·
2 usage error.
"""

from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path


def _repo_root() -> Path:
    return Path(__file__).resolve().parents[2]


def _results_dir_default() -> Path:
    return _repo_root() / "validation" / "results"


def _build_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(prog="python -m validation",
                                description="Lamina public-dataset validation framework")
    sub = p.add_subparsers(dest="command", required=True)

    pl = sub.add_parser("list", help="list registered datasets")
    pl.add_argument("--category", choices=["ecg", "ppg", "autonomic", "rppg"], default=None)

    pc = sub.add_parser("check-access", help="run adapter access checks (no downloads)")
    pc.add_argument("--dataset", default=None, help="single dataset key (default: all)")

    pr = sub.add_parser("run", help="run validation")
    pr.add_argument("--dataset", required=True, help="dataset key or 'all'")
    pr.add_argument("--smoke", action="store_true", help="fast deterministic subset")
    pr.add_argument("--subjects", nargs="*", default=None)
    pr.add_argument("--recordings", nargs="*", default=None)
    pr.add_argument("--max-recordings", type=int, default=None)
    pr.add_argument("--seed", type=int, default=0)
    pr.add_argument("--results-dir", default=str(_results_dir_default()))

    pp = sub.add_parser("report", help="generate the human-readable report")
    pp.add_argument("--results-dir", default=str(_results_dir_default()))
    pp.add_argument("--output",
                    default=str(_repo_root() / "docs" / "validation"
                                / "public-dataset-validation.md"))

    pt = sub.add_parser("test", help="run the framework pytest suite")
    pt.add_argument("pytest_args", nargs=argparse.REMAINDER,
                    help="extra args passed to pytest (after '--')")
    return p


def _cmd_list(args) -> int:
    from .registry import list_datasets

    adapters = list_datasets(category=args.category)
    rows = []
    for a in adapters:
        info = a.info()
        rows.append((info.key, info.category, ", ".join(info.modalities), info.name))
    w = max(len(r[0]) for r in rows)
    print(f"{'KEY'.ljust(w)}  CATEGORY   MODALITIES                    NAME")
    print(f"{'-' * w}  --------   ----------                    ----")
    for key, cat, mods, name in rows:
        print(f"{key.ljust(w)}  {cat.ljust(10)} {mods.ljust(29)} {name}")
    print(f"\n{len(rows)} dataset(s) registered.")
    return 0


def _cmd_check_access(args) -> int:
    from .registry import get_dataset, list_datasets

    adapters = [get_dataset(args.dataset)] if args.dataset else list_datasets()
    any_bad = False
    for a in adapters:
        report = a.check_access()
        flag = "" if report.status.value == "validated" else "!"
        any_bad = any_bad or report.status.value in ("inaccessible", "failed")
        print(f"[{report.status.value:<20}] {a.key:<22} {report.reason} {flag}")
    # check-access is a diagnostic: exits 0 even when datasets are inaccessible.
    return 0


def _cmd_run(args) -> int:
    from .bridge import LaminaBridge, LaminaBridgeError
    from .manifest import build_manifest, write_manifest
    from .registry import get_dataset, list_datasets
    from .runner import RunOptions, run_dataset, write_all

    options = RunOptions(
        subjects=args.subjects,
        recordings=args.recordings,
        max_recordings=args.max_recordings,
        seed=args.seed,
        smoke=args.smoke,
    )
    adapters = [get_dataset(args.dataset)] if args.dataset != "all" else list_datasets()
    results_dir = Path(args.results_dir)
    results_dir.mkdir(parents=True, exist_ok=True)

    try:
        bridge = LaminaBridge(repo_root=_repo_root())
        lamina_version = bridge.version()["lamina_version"]
    except LaminaBridgeError as exc:
        print(f"error: bridge unavailable: {exc}", file=sys.stderr)
        return 1

    results = [run_dataset(a, bridge=bridge, options=options, results_dir=results_dir)
               for a in adapters]
    write_all(results_dir, results, adapters={a.key: a for a in adapters})
    write_manifest(
        results_dir / "manifest.json",
        build_manifest(
            _repo_root(),
            seed=args.seed,
            tolerances=options.tolerances,
            config={"dataset": args.dataset, "smoke": args.smoke,
                    "subjects": args.subjects, "recordings": args.recordings,
                    "max_recordings": args.max_recordings},
            lamina_version=lamina_version,
        ),
    )

    counts: dict[str, int] = {}
    for r in results:
        counts[r.status] = counts.get(r.status, 0) + 1
        print(f"[{r.status:<20}] {r.dataset:<22} ok={r.n_ok} failed={r.n_failed} {r.reason or r.error or ''}")
    print("\nStatus summary:")
    for label, key in [("Validated", "validated"),
                       ("Partially validated", "partially_validated"),
                       ("Inaccessible", "inaccessible"),
                       ("Unsupported format", "unsupported_format"),
                       ("Failed", "failed"),
                       ("Not attempted", "not_attempted")]:
        print(f"  {label:<22} {counts.get(key, 0)}")
    print(f"\nResults written to {results_dir}")
    bad = counts.get("failed", 0) + counts.get("inaccessible", 0)
    return 1 if bad and args.dataset != "all" else 0


def _cmd_report(args) -> int:
    from .report import generate

    out = generate(args.results_dir, args.output)
    print(f"report written to {out}")
    return 0


def _cmd_test(args) -> int:
    tests_dir = _repo_root() / "validation" / "tests"
    extra = [a for a in args.pytest_args if a != "--"]
    cmd = [sys.executable, "-m", "pytest", str(tests_dir), "-q", *extra]
    proc = subprocess.run(cmd, cwd=str(_repo_root()))
    return proc.returncode


def main(argv: list[str] | None = None) -> int:
    args = _build_parser().parse_args(argv)
    handler = {
        "list": _cmd_list,
        "check-access": _cmd_check_access,
        "run": _cmd_run,
        "report": _cmd_report,
        "test": _cmd_test,
    }[args.command]
    return handler(args)


if __name__ == "__main__":
    raise SystemExit(main())
