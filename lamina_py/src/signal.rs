use crate::error::map_signal_error;
use numpy::{IntoPyArray, PyArray1, PyReadonlyArray1};
use pyo3::prelude::*;

#[pyclass]
#[derive(Clone)]
pub struct PyPeakDetectionConfig {
    #[pyo3(get, set)]
    pub min_height: Option<f64>,
    #[pyo3(get, set)]
    pub min_distance: Option<usize>,
    #[pyo3(get, set)]
    pub min_prominence: Option<f64>,
    #[pyo3(get, set)]
    pub min_width: Option<usize>,
    #[pyo3(get, set)]
    pub threshold: Option<f64>,
}

#[pymethods]
impl PyPeakDetectionConfig {
    #[new]
    #[pyo3(signature = (min_height=None, min_distance=None, min_prominence=None, min_width=None, threshold=None))]
    pub fn new(
        min_height: Option<f64>,
        min_distance: Option<usize>,
        min_prominence: Option<f64>,
        min_width: Option<usize>,
        threshold: Option<f64>,
    ) -> Self {
        Self {
            min_height,
            min_distance,
            min_prominence,
            min_width,
            threshold,
        }
    }
}

impl From<&PyPeakDetectionConfig> for lamina::signal::peaks::PeakDetectionConfig {
    fn from(cfg: &PyPeakDetectionConfig) -> Self {
        let mut c = lamina::signal::peaks::PeakDetectionConfig::new();
        c.min_height = cfg.min_height;
        c.min_distance = cfg.min_distance;
        c.min_prominence = cfg.min_prominence;
        c.min_width = cfg.min_width;
        c.threshold = cfg.threshold;
        c
    }
}

#[pyfunction]
pub fn smooth_moving_average<'py>(
    py: Python<'py>,
    signal: PyReadonlyArray1<'py, f64>,
    window_size: usize,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let array_view = signal.as_array();
    let arr = array_view.to_owned();

    let out = py
        .detach(|| lamina::signal::smooth::signal_smooth_moving_average(&arr, window_size))
        .map_err(map_signal_error)?;

    Ok(out.into_pyarray(py))
}

#[pyfunction]
#[pyo3(signature = (signal, sampling_rate, lowcut=None, highcut=None, order=1))]
pub fn filter<'py>(
    py: Python<'py>,
    signal: PyReadonlyArray1<'py, f64>,
    sampling_rate: f64,
    lowcut: Option<f64>,
    highcut: Option<f64>,
    order: usize,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let array_view = signal.as_array();
    let arr = array_view.to_owned();

    let out = py
        .detach(|| {
            lamina::signal::filter::signal_filter(&arr, sampling_rate, lowcut, highcut, order)
        })
        .map_err(map_signal_error)?;

    Ok(out.into_pyarray(py))
}

#[pyfunction]
#[pyo3(signature = (signal, sampling_rate, lowcut=None, highcut=None, order=1))]
pub fn filtfilt<'py>(
    py: Python<'py>,
    signal: PyReadonlyArray1<'py, f64>,
    sampling_rate: f64,
    lowcut: Option<f64>,
    highcut: Option<f64>,
    order: usize,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    filter(py, signal, sampling_rate, lowcut, highcut, order)
}

#[pyfunction]
#[pyo3(signature = (signal, config=None))]
pub fn findpeaks<'py>(
    py: Python<'py>,
    signal: PyReadonlyArray1<'py, f64>,
    config: Option<&PyPeakDetectionConfig>,
) -> PyResult<Bound<'py, PyArray1<usize>>> {
    let array_view = signal.as_array();
    let arr = array_view.to_owned();
    let rust_cfg = config.map(|c| c.into()).unwrap_or_default();

    let out = py
        .detach(|| lamina::signal::peaks::signal_findpeaks_config(&arr, &rust_cfg))
        .map_err(map_signal_error)?;

    let nd_out = ndarray::Array1::from_vec(out);
    Ok(nd_out.into_pyarray(py))
}

#[pyfunction]
#[pyo3(signature = (signal, config=None))]
pub fn findpeaks_mask<'py>(
    py: Python<'py>,
    signal: PyReadonlyArray1<'py, f64>,
    config: Option<&PyPeakDetectionConfig>,
) -> PyResult<Bound<'py, PyArray1<bool>>> {
    let array_view = signal.as_array();
    let arr = array_view.to_owned();
    let rust_cfg = config.map(|c| c.into()).unwrap_or_default();

    let out = py
        .detach(|| lamina::signal::peaks::signal_findpeaks_mask(&arr, &rust_cfg))
        .map_err(map_signal_error)?;

    Ok(out.into_pyarray(py))
}
