use crate::error::map_signal_error;
use numpy::{IntoPyArray, PyArray1, PyReadonlyArray1};
use pyo3::prelude::*;

#[pyclass]
#[derive(Clone)]
pub struct PyPpgPeakDetectionConfig {
    #[pyo3(get, set)]
    pub lowcut: Option<f64>,
    #[pyo3(get, set)]
    pub highcut: Option<f64>,
    #[pyo3(get, set)]
    pub filter_order: Option<usize>,
    #[pyo3(get, set)]
    pub w_peak_sec: Option<f64>,
    #[pyo3(get, set)]
    pub w_beat_sec: Option<f64>,
    #[pyo3(get, set)]
    pub alpha: Option<f64>,
    #[pyo3(get, set)]
    pub refractory_period_sec: Option<f64>,
}

#[pymethods]
impl PyPpgPeakDetectionConfig {
    #[new]
    #[pyo3(signature = (lowcut=None, highcut=None, filter_order=None, w_peak_sec=None, w_beat_sec=None, alpha=None, refractory_period_sec=None))]
    pub fn new(
        lowcut: Option<f64>,
        highcut: Option<f64>,
        filter_order: Option<usize>,
        w_peak_sec: Option<f64>,
        w_beat_sec: Option<f64>,
        alpha: Option<f64>,
        refractory_period_sec: Option<f64>,
    ) -> Self {
        Self {
            lowcut,
            highcut,
            filter_order,
            w_peak_sec,
            w_beat_sec,
            alpha,
            refractory_period_sec,
        }
    }
}

impl From<&PyPpgPeakDetectionConfig> for lamina::ppg::PpgPeakDetectionConfig {
    fn from(cfg: &PyPpgPeakDetectionConfig) -> Self {
        let mut c = lamina::ppg::PpgPeakDetectionConfig::new();
        c.lowcut = cfg.lowcut;
        c.highcut = cfg.highcut;
        c.filter_order = cfg.filter_order;
        c.w_peak_sec = cfg.w_peak_sec;
        c.w_beat_sec = cfg.w_beat_sec;
        c.alpha = cfg.alpha;
        c.refractory_period_sec = cfg.refractory_period_sec;
        c
    }
}

#[pyfunction]
#[pyo3(signature = (signal, sampling_rate))]
pub fn ppg_clean<'py>(
    py: Python<'py>,
    signal: PyReadonlyArray1<'py, f64>,
    sampling_rate: f64,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let array_view = signal.as_array();
    let arr = array_view.to_owned();

    let out = py
        .detach(|| lamina::ppg::ppg_clean(&arr, sampling_rate))
        .map_err(map_signal_error)?;

    Ok(out.into_pyarray(py))
}

#[pyfunction]
#[pyo3(signature = (signal, sampling_rate, config=None))]
pub fn ppg_findpeaks<'py>(
    py: Python<'py>,
    signal: PyReadonlyArray1<'py, f64>,
    sampling_rate: f64,
    config: Option<&PyPpgPeakDetectionConfig>,
) -> PyResult<Bound<'py, PyArray1<usize>>> {
    let array_view = signal.as_array();
    let arr = array_view.to_owned();
    let rust_cfg = config.map(|c| c.into()).unwrap_or_default();

    let out = py
        .detach(|| lamina::ppg::ppg_findpeaks_config(&arr, sampling_rate, &rust_cfg))
        .map_err(map_signal_error)?;

    let nd_out = ndarray::Array1::from_vec(out);
    Ok(nd_out.into_pyarray(py))
}

#[pyfunction]
#[pyo3(signature = (signal, sampling_rate, config=None))]
pub fn ppg_findpeaks_mask<'py>(
    py: Python<'py>,
    signal: PyReadonlyArray1<'py, f64>,
    sampling_rate: f64,
    config: Option<&PyPpgPeakDetectionConfig>,
) -> PyResult<Bound<'py, PyArray1<bool>>> {
    let array_view = signal.as_array();
    let arr = array_view.to_owned();
    let rust_cfg = config.map(|c| c.into()).unwrap_or_default();

    let out = py
        .detach(|| lamina::ppg::ppg_findpeaks_mask(&arr, sampling_rate, &rust_cfg))
        .map_err(map_signal_error)?;

    Ok(out.into_pyarray(py))
}

#[pyclass]
#[derive(Clone)]
pub struct PyPulseLmPipeline {
    pipeline: lamina::ppg::PulseLmPipeline,
}

#[pymethods]
impl PyPulseLmPipeline {
    #[new]
    #[pyo3(signature = (target_fs=125.0, filter_cutoff_hz=8.0, filter_order=4, window_sec=10.0, stride_sec=10.0, tail_policy="drop", degenerate_policy="midpoint"))]
    pub fn new(
        target_fs: f64,
        filter_cutoff_hz: f64,
        filter_order: usize,
        window_sec: f64,
        stride_sec: f64,
        tail_policy: &str,
        degenerate_policy: &str,
    ) -> PyResult<Self> {
        let t_policy = match tail_policy {
            "drop" | "drop_incomplete" => lamina::signal::IncompleteTailPolicy::DropIncomplete,
            "pad" | "pad_zeros" => lamina::signal::IncompleteTailPolicy::PadZeros,
            "keep" | "keep_partial" => lamina::signal::IncompleteTailPolicy::KeepPartial,
            other => {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "Unknown tail_policy: {}",
                    other
                )));
            }
        };

        let d_policy = match degenerate_policy {
            "error" => lamina::signal::DegeneratePolicy::Error,
            "zero" => lamina::signal::DegeneratePolicy::Zero,
            "midpoint" => lamina::signal::DegeneratePolicy::Midpoint,
            other => {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "Unknown degenerate_policy: {}",
                    other
                )));
            }
        };

        let mut spec = lamina::ppg::PulseLmPipelineSpec::default();
        spec.target_fs = target_fs;
        spec.filter_cutoff_hz = filter_cutoff_hz;
        spec.filter_order = filter_order;
        spec.window_sec = window_sec;
        spec.stride_sec = stride_sec;
        spec.tail_policy = tail_policy.to_string();
        spec.degenerate_policy = degenerate_policy.to_string();

        let pipeline = lamina::ppg::PulseLmPipeline::new(spec, t_policy, d_policy);
        Ok(Self { pipeline })
    }

    pub fn compute_sha256_hash(&self) -> String {
        self.pipeline.spec.compute_sha256_hash()
    }

    pub fn process<'py>(
        &self,
        py: Python<'py>,
        signal: PyReadonlyArray1<'py, f64>,
        sampling_rate: f64,
    ) -> PyResult<Vec<Bound<'py, PyArray1<f64>>>> {
        let array_view = signal.as_array();
        let arr = array_view.to_owned();

        let segments = py
            .detach(|| self.pipeline.process(&arr, sampling_rate))
            .map_err(map_signal_error)?;

        let py_segs = segments
            .into_iter()
            .map(|seg| seg.into_pyarray(py))
            .collect();

        Ok(py_segs)
    }
}

#[pyfunction]
#[pyo3(signature = (signal, sampling_rate))]
pub fn ppg_preprocess_pulselm<'py>(
    py: Python<'py>,
    signal: PyReadonlyArray1<'py, f64>,
    sampling_rate: f64,
) -> PyResult<Vec<Bound<'py, PyArray1<f64>>>> {
    let array_view = signal.as_array();
    let arr = array_view.to_owned();

    let segments = py
        .detach(|| lamina::ppg::ppg_preprocess_pulselm(&arr, sampling_rate))
        .map_err(map_signal_error)?;

    let py_segs = segments
        .into_iter()
        .map(|seg| seg.into_pyarray(py))
        .collect();

    Ok(py_segs)
}

