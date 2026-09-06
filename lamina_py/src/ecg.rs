use crate::error::map_signal_error;
use numpy::{IntoPyArray, PyArray1, PyReadonlyArray1};
use pyo3::prelude::*;

#[pyclass]
#[derive(Clone)]
pub struct PyEcgPeakDetectionConfig {
    #[pyo3(get, set)]
    pub lowcut: Option<f64>,
    #[pyo3(get, set)]
    pub highcut: Option<f64>,
    #[pyo3(get, set)]
    pub filter_order: Option<usize>,
    #[pyo3(get, set)]
    pub integration_window_sec: Option<f64>,
    #[pyo3(get, set)]
    pub refractory_period_sec: Option<f64>,
    #[pyo3(get, set)]
    pub searchback: Option<bool>,
    #[pyo3(get, set)]
    pub threshold_multiplier: Option<f64>,
}

#[pymethods]
impl PyEcgPeakDetectionConfig {
    #[new]
    #[pyo3(signature = (lowcut=None, highcut=None, filter_order=None, integration_window_sec=None, refractory_period_sec=None, searchback=None, threshold_multiplier=None))]
    pub fn new(
        lowcut: Option<f64>,
        highcut: Option<f64>,
        filter_order: Option<usize>,
        integration_window_sec: Option<f64>,
        refractory_period_sec: Option<f64>,
        searchback: Option<bool>,
        threshold_multiplier: Option<f64>,
    ) -> Self {
        Self {
            lowcut,
            highcut,
            filter_order,
            integration_window_sec,
            refractory_period_sec,
            searchback,
            threshold_multiplier,
        }
    }
}

impl From<&PyEcgPeakDetectionConfig> for lamina::ecg::EcgPeakDetectionConfig {
    fn from(cfg: &PyEcgPeakDetectionConfig) -> Self {
        let mut c = lamina::ecg::EcgPeakDetectionConfig::new();
        c.lowcut = cfg.lowcut;
        c.highcut = cfg.highcut;
        c.filter_order = cfg.filter_order;
        c.integration_window_sec = cfg.integration_window_sec;
        c.refractory_period_sec = cfg.refractory_period_sec;
        c.searchback = cfg.searchback;
        c.threshold_multiplier = cfg.threshold_multiplier;
        c
    }
}

#[pyfunction]
#[pyo3(signature = (signal, sampling_rate, method="biosppy"))]
pub fn ecg_clean<'py>(
    py: Python<'py>,
    signal: PyReadonlyArray1<'py, f64>,
    sampling_rate: f64,
    method: &str,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let array_view = signal.as_array();
    let arr = array_view.to_owned();

    let out = py
        .detach(|| lamina::ecg::ecg_clean(&arr, sampling_rate, method))
        .map_err(map_signal_error)?;

    Ok(out.into_pyarray(py))
}

#[pyfunction]
#[pyo3(signature = (signal, sampling_rate, config=None))]
pub fn ecg_findpeaks<'py>(
    py: Python<'py>,
    signal: PyReadonlyArray1<'py, f64>,
    sampling_rate: f64,
    config: Option<&PyEcgPeakDetectionConfig>,
) -> PyResult<Bound<'py, PyArray1<usize>>> {
    let array_view = signal.as_array();
    let arr = array_view.to_owned();
    let rust_cfg = config.map(|c| c.into()).unwrap_or_default();

    let out = py
        .detach(|| lamina::ecg::ecg_findpeaks_config(&arr, sampling_rate, &rust_cfg))
        .map_err(map_signal_error)?;

    let nd_out = ndarray::Array1::from_vec(out);
    Ok(nd_out.into_pyarray(py))
}

#[pyfunction]
#[pyo3(signature = (signal, sampling_rate, config=None))]
pub fn ecg_findpeaks_mask<'py>(
    py: Python<'py>,
    signal: PyReadonlyArray1<'py, f64>,
    sampling_rate: f64,
    config: Option<&PyEcgPeakDetectionConfig>,
) -> PyResult<Bound<'py, PyArray1<bool>>> {
    let array_view = signal.as_array();
    let arr = array_view.to_owned();
    let rust_cfg = config.map(|c| c.into()).unwrap_or_default();

    let out = py
        .detach(|| lamina::ecg::ecg_findpeaks_mask(&arr, sampling_rate, &rust_cfg))
        .map_err(map_signal_error)?;

    Ok(out.into_pyarray(py))
}
