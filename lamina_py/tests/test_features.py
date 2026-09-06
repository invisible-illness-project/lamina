import pytest
import numpy as np
import lamina

def test_extract_features():
    r_peaks = [100, 200, 300, 400, 500, 600, 700, 800, 900, 1000]
    inp = lamina.features.MultimodalInput()
    inp.with_ecg(r_peaks, sampling_rate=100.0)
    
    cfg = lamina.features.FeatureConfig()
    vecs = lamina.features.extract_features(inp, config=cfg)
    
    assert isinstance(vecs, list)
    if vecs:
        vec = vecs[0]
        assert hasattr(vec, "mean_hr_bpm")
        assert hasattr(vec, "sdnn_ms")
        assert hasattr(vec, "rmssd_ms")
