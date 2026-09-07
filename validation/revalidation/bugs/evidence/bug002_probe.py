"""BUG-002 revalidation: threshold_multiplier range validation (post-remediation)."""
import sys
sys.path.insert(0, "/home/kimi/work")
import numpy as np
from validation.bridge import LaminaBridge, LaminaBridgeError

b = LaminaBridge()
rng = np.random.default_rng(7)
fs = 360.0
t = np.arange(int(30 * fs)) / fs
sig = 0.05 * rng.standard_normal(t.size)
for beat in np.arange(0.5, 29.5, 1.0):
    idx = int(beat * fs)
    sig[idx] += 1.0
assert np.all(np.isfinite(sig)), "input must be provably finite"
print(f"input finite: {np.all(np.isfinite(sig))}, n={sig.size}")

values = [0.0, 0.5, 1.0, 1.5, -0.5, float("nan"), 0.9999, 1e-12, 0.25]
for tm in values:
    try:
        r = b.ecg_peaks(sig, fs, config={"threshold_multiplier": tm})
        print(f"tm={tm!r}: OK peaks={r['count']}")
    except LaminaBridgeError as e:
        print(f"tm={tm!r}: ERROR kind={e.kind}\n    msg={e}")
