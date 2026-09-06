import concurrent.futures
import numpy as np
import lamina

def _worker(seed):
    np.random.seed(seed)
    t = np.linspace(0, 10.0, 2500)
    sig = np.sin(2 * np.pi * 10 * t) + np.random.randn(len(t)) * 0.1
    filtered = lamina.signal.filtfilt(sig, sampling_rate=250.0, low_cutoff=5.0, high_cutoff=15.0)
    peaks = lamina.ecg.findpeaks(filtered, sampling_rate=250.0)
    return len(peaks)

def test_concurrent_multithreaded_execution():
    with concurrent.futures.ThreadPoolExecutor(max_workers=4) as executor:
        futures = [executor.submit(_worker, i) for i in range(8)]
        results = [f.result() for f in futures]
    assert len(results) == 8
    assert all(r >= 0 for r in results)
