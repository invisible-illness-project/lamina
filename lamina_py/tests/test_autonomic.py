import pytest
import lamina

def test_autonomic_estimator_lifecycle():
    cfg = lamina.autonomic.AutonomicEstimatorConfig()
    estimator = lamina.autonomic.AutonomicEstimator(cfg)
    
    r_peaks = [100, 200, 300, 400, 500, 600, 700, 800, 900, 1000]
    inp = lamina.features.MultimodalInput()
    inp.with_ecg(r_peaks, sampling_rate=100.0)
    
    features_list = lamina.features.extract_features(inp)
    
    if features_list:
        state = estimator.update(features_list[0])
        assert isinstance(state, lamina.autonomic.AutonomicState)
        assert hasattr(state, "arousal_index")
        assert hasattr(state, "valence_proxy")
