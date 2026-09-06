import time
import numpy as np
import lamina

def run_benchmarks():
    print("=" * 60)
    print("LAMINA PYTHON BINDINGS PERFORMANCE BENCHMARKS")
    print("=" * 60)

    # 1. ECG Clean & Peak Detection Benchmarks across signal sizes
    sizes = [250, 2500, 25000, 250000]
    for n in sizes:
        t = np.linspace(0, n / 250.0, n)
        sig = np.sin(2 * np.pi * 1.0 * t) + 2.0 * np.exp(-((t % 1.0 - 0.2)**2) / 0.001)

        start = time.perf_counter()
        iterations = 100 if n <= 2500 else 10
        for _ in range(iterations):
            cleaned = lamina.ecg.clean(sig, sampling_rate=250.0)
            _ = lamina.ecg.findpeaks(cleaned, sampling_rate=250.0)
        elapsed = (time.perf_counter() - start) / iterations * 1000.0

        throughput = (n / (elapsed / 1000.0)) / 1e6
        print(f"ECG Pipeline (N={n:6d} samples): {elapsed:7.3f} ms/call ({throughput:6.2f} MSamples/sec)")

    # 2. Contiguous vs Non-Contiguous NumPy Arrays
    data = np.random.randn(50000)
    data_strided = data[::2]  # non-contiguous

    start = time.perf_counter()
    for _ in range(50):
        _ = lamina.signal.smooth_moving_average(data, 5)
    t_cont = (time.perf_counter() - start) / 50 * 1000.0

    start = time.perf_counter()
    for _ in range(50):
        _ = lamina.signal.smooth_moving_average(data_strided, 5)
    t_stride = (time.perf_counter() - start) / 50 * 1000.0

    print(f"\nNumPy Array Layout Overhead:")
    print(f"  Contiguous array (N=50000):     {t_cont:.3f} ms/call")
    print(f"  Non-contiguous array (N=25000): {t_stride:.3f} ms/call")

    # 3. Multimodal Feature Extraction
    r_peaks = list(range(100, 24000, 250))
    inp = lamina.features.MultimodalInput()
    inp.with_ecg(r_peaks, sampling_rate=250.0)

    start = time.perf_counter()
    for _ in range(50):
        vecs = lamina.features.extract_features(inp)
    t_feat = (time.perf_counter() - start) / 50 * 1000.0
    print(f"\nMultimodal Feature Extraction (N={len(r_peaks)} R-peaks): {t_feat:.3f} ms/call ({len(vecs)} windows)")

    print("=" * 60)

if __name__ == "__main__":
    run_benchmarks()
