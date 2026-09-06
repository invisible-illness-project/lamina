use crate::error::map_signal_error;
use numpy::{IntoPyArray, PyArray1, PyReadonlyArray1};
use pyo3::prelude::*;

#[pyclass]
#[derive(Clone)]
pub struct PyRspCleaningConfig {
    #[pyo3(get, set)]
    pub lowcut: Option<f64>,
    #[pyo3(get, set)]
    pub highcut: Option<f64>,
    #[pyo3(get, set)]
    pub filter_order: Option<usize>,
}

#[pymethods]
impl PyRspCleaningConfig {
    #[new]
    #[pyo3(signature = (lowcut=None, highcut=None, filter_order=None))]
    pub fn new(lowcut: Option<f64>, highcut: Option<f64>, filter_order: Option<usize>) -> Self {
        Self {
            lowcut,
            highcut,
            filter_order,
        }
    }
}

impl From<&PyRspCleaningConfig> for lamina::rsp::RspCleaningConfig {
    fn from(cfg: &PyRspCleaningConfig) -> Self {
        let mut c = lamina::rsp::RspCleaningConfig::new();
        c.lowcut = cfg.lowcut;
        c.highcut = cfg.highcut;
        c.filter_order = cfg.filter_order;
        c
    }
}

#[pyclass]
#[derive(Clone)]
pub struct PyRspProcessingConfig {
    #[pyo3(get, set)]
    pub lowcut: Option<f64>,
    #[pyo3(get, set)]
    pub highcut: Option<f64>,
    #[pyo3(get, set)]
    pub filter_order: Option<usize>,
    #[pyo3(get, set)]
    pub min_breath_interval_sec: Option<f64>,
    #[pyo3(get, set)]
    pub max_breath_interval_sec: Option<f64>,
    #[pyo3(get, set)]
    pub min_amplitude: Option<f64>,
    #[pyo3(get, set)]
    pub precleaned: Option<bool>,
}

#[pymethods]
impl PyRspProcessingConfig {
    #[new]
    #[pyo3(signature = (lowcut=None, highcut=None, filter_order=None, min_breath_interval_sec=None, max_breath_interval_sec=None, min_amplitude=None, precleaned=None))]
    pub fn new(
        lowcut: Option<f64>,
        highcut: Option<f64>,
        filter_order: Option<usize>,
        min_breath_interval_sec: Option<f64>,
        max_breath_interval_sec: Option<f64>,
        min_amplitude: Option<f64>,
        precleaned: Option<bool>,
    ) -> Self {
        Self {
            lowcut,
            highcut,
            filter_order,
            min_breath_interval_sec,
            max_breath_interval_sec,
            min_amplitude,
            precleaned,
        }
    }
}

impl From<&PyRspProcessingConfig> for lamina::rsp::RspProcessingConfig {
    fn from(cfg: &PyRspProcessingConfig) -> Self {
        let mut c = lamina::rsp::RspProcessingConfig::new();
        c.lowcut = cfg.lowcut;
        c.highcut = cfg.highcut;
        c.filter_order = cfg.filter_order;
        c.min_breath_interval_sec = cfg.min_breath_interval_sec;
        c.max_breath_interval_sec = cfg.max_breath_interval_sec;
        c.min_amplitude = cfg.min_amplitude;
        c.precleaned = cfg.precleaned;
        c
    }
}

#[pyclass]
#[derive(Clone)]
pub struct PyRespirationCycle {
    #[pyo3(get)]
    pub inspiration_index: usize,
    #[pyo3(get)]
    pub expiration_index: usize,
    #[pyo3(get)]
    pub next_inspiration_index: usize,
    #[pyo3(get)]
    pub duration_sec: f64,
    #[pyo3(get)]
    pub respiratory_rate_bpm: f64,
    #[pyo3(get)]
    pub amplitude: f64,
}

#[pymethods]
impl PyRespirationCycle {
    fn __repr__(&self) -> String {
        format!(
            "RespirationCycle(insp={}, exp={}, next_insp={}, duration={:.3}s, rate={:.1}BPM, amp={:.4})",
            self.inspiration_index,
            self.expiration_index,
            self.next_inspiration_index,
            self.duration_sec,
            self.respiratory_rate_bpm,
            self.amplitude
        )
    }
}

impl From<&lamina::rsp::RespirationCycle> for PyRespirationCycle {
    fn from(c: &lamina::rsp::RespirationCycle) -> Self {
        Self {
            inspiration_index: c.inspiration_index,
            expiration_index: c.expiration_index,
            next_inspiration_index: c.next_inspiration_index,
            duration_sec: c.duration_sec,
            respiratory_rate_bpm: c.respiratory_rate_bpm,
            amplitude: c.amplitude,
        }
    }
}

#[pyfunction]
#[pyo3(signature = (signal, sampling_rate, config=None))]
pub fn rsp_clean<'py>(
    py: Python<'py>,
    signal: PyReadonlyArray1<'py, f64>,
    sampling_rate: f64,
    config: Option<&PyRspCleaningConfig>,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let array_view = signal.as_array();
    let arr = array_view.to_owned();

    let out = py
        .detach(|| {
            if let Some(cfg) = config {
                let rust_cfg = cfg.into();
                lamina::rsp::rsp_clean_config(&arr, sampling_rate, &rust_cfg)
            } else {
                lamina::rsp::rsp_clean(&arr, sampling_rate)
            }
        })
        .map_err(map_signal_error)?;

    Ok(out.into_pyarray(py))
}

#[pyfunction]
#[pyo3(signature = (signal, sampling_rate, config=None))]
pub fn rsp_findpeaks<'py>(
    py: Python<'py>,
    signal: PyReadonlyArray1<'py, f64>,
    sampling_rate: f64,
    config: Option<&PyRspProcessingConfig>,
) -> PyResult<Bound<'py, PyArray1<usize>>> {
    let array_view = signal.as_array();
    let arr = array_view.to_owned();
    let rust_cfg = config.map(|c| c.into()).unwrap_or_default();

    let out = py
        .detach(|| lamina::rsp::rsp_findpeaks_config(&arr, sampling_rate, &rust_cfg))
        .map_err(map_signal_error)?;

    let nd_out = ndarray::Array1::from_vec(out);
    Ok(nd_out.into_pyarray(py))
}

#[pyfunction]
#[pyo3(signature = (signal, sampling_rate, config=None))]
pub fn rsp_cycles<'py>(
    py: Python<'py>,
    signal: PyReadonlyArray1<'py, f64>,
    sampling_rate: f64,
    config: Option<&PyRspProcessingConfig>,
) -> PyResult<Vec<PyRespirationCycle>> {
    let array_view = signal.as_array();
    let arr = array_view.to_owned();
    let rust_cfg = config.map(|c| c.into()).unwrap_or_default();

    let cycles = py
        .detach(|| lamina::rsp::rsp_cycles_config(&arr, sampling_rate, &rust_cfg))
        .map_err(map_signal_error)?;

    Ok(cycles.iter().map(PyRespirationCycle::from).collect())
}

#[pyfunction]
#[pyo3(signature = (signal, sampling_rate, config=None))]
pub fn rsp_rate<'py>(
    py: Python<'py>,
    signal: PyReadonlyArray1<'py, f64>,
    sampling_rate: f64,
    config: Option<&PyRspProcessingConfig>,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let array_view = signal.as_array();
    let arr = array_view.to_owned();

    let out = py
        .detach(|| {
            if let Some(cfg) = config {
                let rust_cfg = cfg.into();
                lamina::rsp::rsp_rate_config(&arr, sampling_rate, &rust_cfg)
            } else {
                lamina::rsp::rsp_rate(&arr, sampling_rate)
            }
        })
        .map_err(map_signal_error)?;

    Ok(out.into_pyarray(py))
}
