"""BUG-018 (partial) revalidation: correction-policy API path exists — hrv-correct smoke."""
import sys
sys.path.insert(0, "/home/kimi/work")
import numpy as np
from validation.bridge import LaminaBridge, LaminaBridgeError

b = LaminaBridge()

# RR sequence with an ectopic couplet (bigeminy-like): alternating 440/830 ms
base = [800.0] * 20
ectopic = []
for i in range(10):
    ectopic += [440.0, 830.0]
rr = base + ectopic + base

print("== raw hrv op (no correction; BUG-018 original path) ==")
# emulate hrv op input via peaks at fs=1000
peaks = np.cumsum([int(r) for r in rr]).tolist()
siglen = peaks[-1] + 500
res = b.hrv(peaks, siglen, 1000.0)
print("hrv:", {k: v for k, v in res.items()})

print("\n== hrv-correct policies ==")
for pol in ["none", "reject_invalid", "interpolate_linear", "interpolate_cubic"]:
    try:
        r = b.hrv_correct(rr_intervals_ms=rr, policy=pol)
        print(f"policy={pol}: n_in={r['n_input_intervals']} n_nn={r['n_nn']} "
              f"rmssd={r['rmssd_ms']} mean_nn={r['mean_nn_ms']}")
        print(f"   quality kinds: {sorted(set(r['interval_quality']))}, "
              f"n ectopic={r['interval_quality'].count('ectopic_rr')}, "
              f"n artifact={r['interval_quality'].count('artifact_rr')}")
    except LaminaBridgeError as e:
        print(f"policy={pol}: ERROR kind={e.kind} msg={e}")

print("\n== percent_threshold policy ==")
try:
    r = b.hrv_correct(rr_intervals_ms=rr, policy="percent_threshold", percent_threshold=0.2)
    print(f"percent_threshold(0.2): n_nn={r['n_nn']} rmssd={r['rmssd_ms']}")
except LaminaBridgeError as e:
    print(f"percent_threshold: ERROR kind={e.kind} msg={e}")

print("\n== cubic vs linear identical? (SPEC A.3 known gap) ==")
rl = b.hrv_correct(rr_intervals_ms=rr, policy="interpolate_linear")
rc = b.hrv_correct(rr_intervals_ms=rr, policy="interpolate_cubic")
same = rl["nn_intervals_ms"] == rc["nn_intervals_ms"]
print(f"linear nn == cubic nn: {same}; rmssd lin={rl['rmssd_ms']} cub={rc['rmssd_ms']}")

print("\n== empty interval input ==")
r = b.hrv_correct(rr_intervals_ms=[])
print(f"empty: n_in={r['n_input_intervals']} n_nn={r['n_nn']} rmssd={r['rmssd_ms']} mean_nn={r['mean_nn_ms']}")

print("\n== peaks envelope input ==")
r = b.hrv_correct(peaks=peaks, signal_length=siglen, fs=1000.0, policy="reject_invalid")
print(f"peaks-input reject_invalid: n_in={r['n_input_intervals']} n_nn={r['n_nn']} rmssd={r['rmssd_ms']}")
