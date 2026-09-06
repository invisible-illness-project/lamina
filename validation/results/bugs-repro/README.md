# Bug reproduction scripts (bug-triage reviewer, 2026-09-07)

Minimal, dataset-free reproductions for entries in `docs/validation/BUGS.md`.
All scripts use the `lamina_bridge` JSON bridge against the current Lamina
implementation; run from the repository root (or this directory, which adds
the repo root to `sys.path`). `.txt` files are captured outputs.

| Script | BUGS.md entry | Result |
| ------ | ------------- | ------ |
| `repro_ecg_001_method_ignored.py` / `.txt` | BUG-001 | confirmed |
| `repro_ecg_002_threshold_multiplier.py` / `.txt` | BUG-002 | confirmed (boundary exact at 1.0) |
| `repro_a03_eda_5hz_lowpass.py` / `.txt` | BUG-003 | confirmed (fs<=10 fails) |
| `repro_fs100_hardcoded_probe.rs` / `.txt` | BUG-004 | confirmed (Rust probe; crate lives outside repo, path-dep on lamina) |
| `repro_entropy_and_rsp_double_clean.py` / `.txt` | BUG-006, BUG-007 | both confirmed |
| `repro_r01_rppg_sign.py` / `.txt` | BUG-005 | sign inconsistency confirmed (synthetic) |
| `repro_ecg_003_amplitude_disparity.py` / `.txt` | BUG-011 | NOT reproduced on idealized synthetics (entry kept `suspected`) |
| `repro_ppg_001_biphasic_probe.txt` (inline probe) | BUG-012 | NOT reproduced on idealized synthetics (entry kept `suspected`) |
| `repro_parity_tests_missing_goldens.txt` | BUG-009 | confirmed (cargo output) |
