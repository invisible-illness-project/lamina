import pytest
import numpy as np
import lamina

def test_cardiorespiratory_phase_coupling():
    phases = [0.1, 0.15, 0.08, 0.12, 0.09]
    res = lamina.multimodal.cardiorespiratory_phase_coupling(phases)
    assert hasattr(res, "concentration")
    assert hasattr(res, "mean_phase")
    assert hasattr(res, "sample_count")
    assert res.concentration > 0.9

def test_multimodal_quality():
    q_ecg = lamina.multimodal.ModalityQuality(score=0.95, valid=True)
    q_rsp = lamina.multimodal.ModalityQuality(score=0.85, valid=True)
    res = lamina.multimodal.multimodal_quality(ecg_quality=q_ecg, rsp_quality=q_rsp)
    assert hasattr(res, "overall_quality")
    assert res.overall_quality == pytest.approx(0.90)
