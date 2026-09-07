"""BUG-001 revalidation: ecg_clean method dispatch (post-remediation fec2668)."""
import sys, json
sys.path.insert(0, "/home/kimi/work")
import numpy as np
from validation.bridge import LaminaBridge, LaminaBridgeError

b = LaminaBridge()
print("bridge version:", b.version())

rng = np.random.default_rng(42)
fs = 360.0
t = np.arange(int(30 * fs)) / fs
# Synthetic ECG-like: QRS spikes at 1 Hz + baseline wander 0.3 Hz + noise
sig = 0.3 * np.sin(2 * np.pi * 0.3 * t) + 0.02 * rng.standard_normal(t.size)
for beat in np.arange(0.5, 29.5, 1.0):
    idx = int(beat * fs)
    sig[idx] += 1.0
    sig[idx + 1] += 0.3

methods = ["", "none", "neurokit", "pantompkins", "biosppy"]
outs = {}
for m in methods:
    try:
        outs[m] = b.ecg_clean(sig, fs, method=m)
        print(f"method={m!r}: OK, len={len(outs[m])}")
    except LaminaBridgeError as e:
        print(f"method={m!r}: ERROR kind={e.kind} msg={e}")

# Pairwise max abs diff
print("\npairwise max-abs-diff:")
keys = list(outs)
for i in range(len(keys)):
    for j in range(i + 1, len(keys)):
        d = np.max(np.abs(outs[keys[i]] - outs[keys[j]]))
        print(f"  {keys[i]!r} vs {keys[j]!r}: {d:.3e}")

# cleaning actually modifies signal
d_raw = np.max(np.abs(outs[""] - sig))
print(f"\nraw vs cleaned max-abs-diff: {d_raw:.6f}")

# unsupported methods
for bad in ["bogus", "NEUROKIT2", "pan-tompkins", "hamilton", "  neurokit  ", "BioSPPy"]:
    try:
        out = b.ecg_clean(sig, fs, method=bad)
        print(f"method={bad!r}: ACCEPTED (maxdiff vs '' = {np.max(np.abs(out - outs[''])):.3e})")
    except LaminaBridgeError as e:
        print(f"method={bad!r}: REJECTED kind={e.kind} msg={e}")

# case-insensitivity / whitespace handling check (documenting normalization)
