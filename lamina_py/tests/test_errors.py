import pytest
import numpy as np
import lamina

def test_empty_signal_error():
    empty = np.array([], dtype=np.float64)
    with pytest.raises(lamina.EmptySignalError) as exc_info:
        lamina.signal.smooth_moving_average(empty, 3)
    assert isinstance(exc_info.value, lamina.LaminaInputError)
    assert isinstance(exc_info.value, ValueError)

def test_non_finite_input_error():
    nan_arr = np.array([1.0, np.nan, 3.0], dtype=np.float64)
    with pytest.raises(lamina.NonFiniteInputError) as exc_info:
        lamina.signal.smooth_moving_average(nan_arr, 3)
    assert isinstance(exc_info.value, lamina.LaminaInputError)

def test_invalid_sampling_rate_error():
    data = np.array([1.0, 2.0, 3.0, 4.0, 5.0], dtype=np.float64)
    with pytest.raises(lamina.InvalidSamplingRateError) as exc_info:
        lamina.signal.filter(data, sampling_rate=0.0, low_cutoff=1.0, high_cutoff=5.0)
    assert isinstance(exc_info.value, lamina.LaminaConfigurationError)
