import pytest
import numpy as np
import lamina

def test_sample_entropy_sine():
    t = np.linspace(0, 10.0, 500)
    sig = np.sin(2 * np.pi * 2.0 * t)
    samp_en = lamina.complexity.sample_entropy(sig, m=2, r=0.2)
    assert isinstance(samp_en, float)
    assert samp_en >= 0.0

def test_sample_entropy_noise():
    np.random.seed(42)
    noise = np.random.randn(500)
    samp_en = lamina.complexity.sample_entropy(noise, m=2, r=0.2)
    assert isinstance(samp_en, float)
    assert samp_en > 0.0
