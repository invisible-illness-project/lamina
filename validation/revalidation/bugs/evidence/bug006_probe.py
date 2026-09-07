"""BUG-006 revalidation: sample_entropy contracts (post-remediation)."""
import sys
sys.path.insert(0, "/home/kimi/work")
import numpy as np
from validation.bridge import LaminaBridge, LaminaBridgeError

b = LaminaBridge()
rng = np.random.default_rng(123)
noise = rng.standard_normal(300)

print("== a) zero template matches (tiny r on noise) ==")
for r in [1e-3, 1e-4, 1e-6]:
    res = b.sample_entropy(noise, m=2, r=r)
    print(f"r={r}: sample_entropy={res['sample_entropy']} is_infinite={res['is_infinite']}")

print("\n== b) tolerance r <= 0 ==")
for r in [0.0, -0.1]:
    try:
        res = b.sample_entropy(noise, m=2, r=r)
        print(f"r={r}: OK -> {res}")
    except LaminaBridgeError as e:
        print(f"r={r}: ERROR kind={e.kind} msg={e}")

print("\n== c) constant signal with default r (0.2*std = 0) ==")
const = np.ones(200)
try:
    res = b.sample_entropy(const)
    print(f"const default-r: {res}")
except LaminaBridgeError as e:
    print(f"const default-r: ERROR kind={e.kind} msg={e}")

print("\n== d) highly regular signal (repeating pattern) ==")
periodic = np.tile(np.array([0.0, 1.0, 0.0, -1.0]), 100)  # perfectly periodic
res = b.sample_entropy(periodic, m=2, r=0.2 * np.std(periodic))
print(f"periodic: sample_entropy={res['sample_entropy']} is_infinite={res['is_infinite']}")

print("\n== e) linear ramp (original repro observed -0.0) ==")
ramp = np.linspace(0, 10, 300)
res = b.sample_entropy(ramp, m=2, r=0.2 * np.std(ramp))
print(f"ramp: sample_entropy={res['sample_entropy']} is_infinite={res['is_infinite']}")

print("\n== f) normal noise, default r ==")
res = b.sample_entropy(noise)
print(f"noise default-r: sample_entropy={res['sample_entropy']} is_infinite={res['is_infinite']} r={res.get('r')}")
