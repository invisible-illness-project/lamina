import numpy as np
import pytest
import lamina
from lamina import signal as sig, hrv

def test_peaks_to_intervals_units_consistent():
    idx = np.arange(0, 5000, 250)
    mask = np.zeros(5000, dtype=bool); mask[idx] = True
    via_idx = hrv.peaks_to_intervals(idx, sampling_rate=250.0)
    via_mask = hrv.peaks_to_intervals(mask, sampling_rate=250.0)
    np.testing.assert_allclose(via_idx, via_mask)
    np.testing.assert_allclose(via_idx, 1000.0)

def test_rmssd_mean_nn_units():
    const = np.full(10, 800.0)
    assert hrv.rmssd(const) == 0.0
    assert hrv.mean_nn(const) == 800.0

def test_filter_btype_kinds():
    t = np.arange(1000) / 250.0
    x = np.sin(2 * np.pi * 0.5 * t) + np.sin(2 * np.pi * 50 * t)
    for btype, lo, hi in [("lowpass", None, 40.0), ("highpass", 1.0, None),
                          ("bandpass", 0.5, 5.0), ("notch", 49.0, 51.0)]:
        y = sig.filter(x, 250.0, low_cutoff=lo, high_cutoff=hi, btype=btype)
        assert y.shape == x.shape and np.all(np.isfinite(y))
    with pytest.raises(ValueError):
        sig.filter(x, 250.0, low_cutoff=1.0, btype="bandpass")
    with pytest.raises(ValueError):
        sig.filter(x, 250.0, low_cutoff=1.0, high_cutoff=5.0, btype="bogus")

def test_findpeaks_extra_params():
    x = np.array([0, 0, 1, 0, 0, 0, 0, 0.2, 0, 0, 0, 1, 0, 0], dtype=float)
    all_p = sig.findpeaks(x, 10.0, min_height=0.5, min_distance_sec=0.0)
    prom_p = sig.findpeaks(x, 10.0, min_height=0.5, min_distance_sec=0.0, min_prominence=0.9)
    assert len(all_p) == 2 and len(prom_p) == 1

def test_mask_variants_match_indices():
    phasic = np.zeros(2000); phasic[[500, 1000, 1500]] = 1.0
    cfg = lamina.eda.EdaPeakDetectionConfig(min_height=0.5)
    idx = lamina.eda.findpeaks(phasic, 100.0, config=cfg)
    mask = lamina.eda.findpeaks_mask(phasic, 100.0, config=cfg)
    assert set(np.asarray(idx).tolist()) == set(np.nonzero(np.asarray(mask))[0].tolist())
    rsp_sig = np.sin(np.linspace(0, 6 * np.pi, 2000))
    ridx = lamina.rsp.findpeaks(rsp_sig, 100.0)
    rmask = lamina.rsp.findpeaks_mask(rsp_sig, 100.0)
    assert set(np.asarray(ridx).tolist()) == set(np.nonzero(np.asarray(rmask))[0].tolist())

def test_multimodal_input_with_rsp():
    cyc = lamina.rsp.RespirationCycle(
        inspiration_index=100, expiration_index=300, next_inspiration_index=500,
        duration_sec=4.0, respiratory_rate_bpm=15.0, amplitude=1.0)
    inp = lamina.features.MultimodalInput()
    inp.with_rsp([cyc], sampling_rate=100.0)
    fvs = lamina.features.extract_features(inp)
    assert len(fvs) >= 1 and fvs[0].mean_rsp_rate_bpm is not None

def test_hrv_quality():
    rr = np.array([800.0, 820.0, 250.0, 810.0, np.nan, 805.0])
    labels = hrv.classify_intervals(rr)
    assert labels[2] == "ArtifactRR" and labels[4] == "Missing"
    cleaned = hrv.clean_rr_intervals(rr, hrv.CorrectionPolicy.reject_invalid())
    assert len(cleaned) < len(rr) and np.all(np.isfinite(cleaned))
