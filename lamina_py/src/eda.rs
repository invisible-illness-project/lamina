use crate::error::map_signal_error;
use numpy::{IntoPyArray, PyArray1, PyReadonlyArray1};
use pyo3::prelude::*;

#[pyclass]
#[derive(Clone)]
pub struct PyEdaDecompositionConfig {
    #[pyo3(get, set)]
    pub tonic_cutoff_hz: Option<f64>,
    #[pyo3(get, set)]
    pub filter_order: Option<usize>,
}

#[pymethods]
impl PyEdaDecompositionConfig {
    #[new]
    #[pyo3(signature = (tonic_cutoff_hz=None, filter_order=None))]
    pub fn new(tonic_cutoff_hz: Option<f64>, filter_order: Option<usize>) -> Self {
        Self {
            tonic_cutoff_hz,
            filter_order,
        }
    }
}

impl From<&PyEdaDecompositionConfig> for lamina::eda::EdaDecompositionConfig {
    fn from(cfg: &PyEdaDecompositionConfig) -> Self {
        let mut c = lamina::eda::EdaDecompositionConfig::new();
        c.tonic_cutoff_hz = cfg.tonic_cutoff_hz;
        c.filter_order = cfg.filter_order;
        c
    }
}

#[pyclass]
#[derive(Clone)]
pub struct PyEdaPeakDetectionConfig {
    #[pyo3(get, set)]
    pub min_amplitude: Option<f64>,
    #[pyo3(get, set)]
    pub min_prominence: Option<f64>,
    #[pyo3(get, set)]
    pub min_distance_sec: Option<f64>,
    #[pyo3(get, set)]
    pub min_rise_time_sec: Option<f64>,
    #[pyo3(get, set)]
    pub max_rise_time_sec: Option<f64>,
}

#[pymethods]
impl PyEdaPeakDetectionConfig {
    #[new]
    #[pyo3(signature = (min_amplitude=None, min_prominence=None, min_distance_sec=None, min_rise_time_sec=None, max_rise_time_sec=None))]
    pub fn new(
        min_amplitude: Option<f64>,
        min_prominence: Option<f64>,
        min_distance_sec: Option<f64>,
        min_rise_time_sec: Option<f64>,
        max_rise_time_sec: Option<f64>,
    ) -> Self {
        Self {
            min_amplitude,
            min_prominence,
            min_distance_sec,
            min_rise_time_sec,
            max_rise_time_sec,
        }
    }
}

impl From<&PyEdaPeakDetectionConfig> for lamina::eda::EdaPeakDetectionConfig {
    fn from(cfg: &PyEdaPeakDetectionConfig) -> Self {
        let mut c = lamina::eda::EdaPeakDetectionConfig::new();
        c.min_amplitude = cfg.min_amplitude;
        c.min_prominence = cfg.min_prominence;
        c.min_distance_sec = cfg.min_distance_sec;
        c.min_rise_time_sec = cfg.min_rise_time_sec;
        c.max_rise_time_sec = cfg.max_rise_time_sec;
        c
    }
}

#[pyclass]
#[derive(Clone)]
pub struct PyScrEvent {
    #[pyo3(get)]
    pub onset_index: usize,
    #[pyo3(get)]
    pub peak_index: usize,
    #[pyo3(get)]
    pub amplitude: f64,
    #[pyo3(get)]
    pub rise_time_sec: f64,
}

#[pymethods]
impl PyScrEvent {
    fn __repr__(&self) -> String {
        format!(
            "ScrEvent(onset_index={}, peak_index={}, amplitude={:.4}, rise_time_sec={:.3})",
            self.onset_index, self.peak_index, self.amplitude, self.rise_time_sec
        )
    }
}

impl From<&lamina::eda::ScrEvent> for PyScrEvent {
    fn from(e: &lamina::eda::ScrEvent) -> Self {
        Self {
            onset_index: e.onset_index,
            peak_index: e.peak_index,
            amplitude: e.amplitude,
            rise_time_sec: e.rise_time_sec,
        }
    }
}

#[pyclass]
pub struct PyEdaComponents {
    #[pyo3(get)]
    pub tonic: Py<PyAny>,
    #[pyo3(get)]
    pub phasic: Py<PyAny>,
}

#[pyfunction]
#[pyo3(signature = (signal, sampling_rate))]
pub fn eda_clean<'py>(
    py: Python<'py>,
    signal: PyReadonlyArray1<'py, f64>,
    sampling_rate: f64,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let array_view = signal.as_array();
    let arr = array_view.to_owned();

    let out = py
        .detach(|| lamina::eda::eda_clean(&arr, sampling_rate))
        .map_err(map_signal_error)?;

    Ok(out.into_pyarray(py))
}

#[pyfunction]
#[pyo3(signature = (signal, sampling_rate, config=None))]
pub fn eda_decompose<'py>(
    py: Python<'py>,
    signal: PyReadonlyArray1<'py, f64>,
    sampling_rate: f64,
    config: Option<&PyEdaDecompositionConfig>,
) -> PyResult<PyEdaComponents> {
    let array_view = signal.as_array();
    let arr = array_view.to_owned();
    let rust_cfg = config.map(|c| c.into()).unwrap_or_default();

    let components = py
        .detach(|| lamina::eda::eda_decompose(&arr, sampling_rate, &rust_cfg))
        .map_err(map_signal_error)?;

    let tonic_py = components.tonic.into_pyarray(py).into_any().unbind();
    let phasic_py = components.phasic.into_pyarray(py).into_any().unbind();

    Ok(PyEdaComponents {
        tonic: tonic_py,
        phasic: phasic_py,
    })
}

#[pyfunction]
#[pyo3(signature = (signal, sampling_rate))]
pub fn eda_phasic<'py>(
    py: Python<'py>,
    signal: PyReadonlyArray1<'py, f64>,
    sampling_rate: f64,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let array_view = signal.as_array();
    let arr = array_view.to_owned();

    let out = py
        .detach(|| lamina::eda::eda_phasic(&arr, sampling_rate))
        .map_err(map_signal_error)?;

    Ok(out.into_pyarray(py))
}

#[pyfunction]
#[pyo3(signature = (phasic_signal, sampling_rate=100.0, config=None))]
pub fn eda_findpeaks<'py>(
    py: Python<'py>,
    phasic_signal: PyReadonlyArray1<'py, f64>,
    sampling_rate: f64,
    config: Option<&PyEdaPeakDetectionConfig>,
) -> PyResult<Bound<'py, PyArray1<usize>>> {
    let array_view = phasic_signal.as_array();
    let arr = array_view.to_owned();
    let rust_cfg = config.map(|c| c.into()).unwrap_or_default();

    let out = py
        .detach(|| lamina::eda::eda_findpeaks_config(&arr, sampling_rate, &rust_cfg))
        .map_err(map_signal_error)?;

    let nd_out = ndarray::Array1::from_vec(out);
    Ok(nd_out.into_pyarray(py))
}

#[pyfunction]
#[pyo3(signature = (phasic_signal, sampling_rate, config=None))]
pub fn eda_findpeaks_events<'py>(
    py: Python<'py>,
    phasic_signal: PyReadonlyArray1<'py, f64>,
    sampling_rate: f64,
    config: Option<&PyEdaPeakDetectionConfig>,
) -> PyResult<Vec<PyScrEvent>> {
    let array_view = phasic_signal.as_array();
    let arr = array_view.to_owned();
    let rust_cfg = config.map(|c| c.into()).unwrap_or_default();

    let events = py
        .detach(|| lamina::eda::eda_findpeaks_events(&arr, sampling_rate, &rust_cfg))
        .map_err(map_signal_error)?;

    Ok(events.iter().map(PyScrEvent::from).collect())
}
