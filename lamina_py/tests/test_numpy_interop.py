import pytest
import numpy as np
import lamina

def test_numpy_float32_conversion():
    f32_arr = np.array([1.0, 2.0, 3.0, 4.0, 5.0, 4.0, 3.0, 2.0, 1.0] * 10, dtype=np.float32)
    res = lamina.signal.smooth_moving_average(f32_arr, 3)
    assert res.dtype == np.float64
    assert len(res) == len(f32_arr)

def test_numpy_non_contiguous_strided_array():
    full_arr = np.array([1.0, 99.0, 2.0, 99.0, 3.0, 99.0, 4.0, 99.0, 5.0, 99.0] * 10, dtype=np.float64)
    strided_arr = full_arr[::2]  # non-contiguous slice
    assert not strided_arr.flags.c_contiguous
    res = lamina.signal.smooth_moving_average(strided_arr, 3)
    assert isinstance(res, np.ndarray)
    assert len(res) == len(strided_arr)

def test_numpy_read_only_array():
    arr = np.array([1.0, 2.0, 3.0, 4.0, 5.0, 4.0, 3.0, 2.0, 1.0] * 10, dtype=np.float64)
    arr.flags.writeable = False
    res = lamina.ecg.clean(arr, sampling_rate=250.0)
    assert isinstance(res, np.ndarray)

def test_python_list_input():
    lst = [1.0, 2.0, 3.0, 4.0, 5.0, 4.0, 3.0, 2.0, 1.0] * 10
    res = lamina.signal.smooth_moving_average(lst, 3)
    assert isinstance(res, np.ndarray)
