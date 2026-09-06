use crate::error::map_signal_error;
use numpy::PyReadonlyArray1;
use pyo3::prelude::*;

#[pyfunction]
#[pyo3(signature = (signal, m, r))]
pub fn sample_entropy<'py>(
    py: Python<'py>,
    signal: PyReadonlyArray1<'py, f64>,
    m: usize,
    r: f64,
) -> PyResult<f64> {
    let array_view = signal.as_array();
    let arr = array_view.to_owned();

    py.detach(|| lamina::complexity::entropy::sample_entropy(&arr, m, r))
        .map_err(map_signal_error)
}
