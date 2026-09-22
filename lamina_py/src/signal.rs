use crate::error::map_signal_error;
use lamina::error::SignalError;
use lamina::signal::filter::FilterSpec;
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

fn build_spec(
    kind: &str,
    sampling_rate: f64,
    lowcut: Option<f64>,
    highcut: Option<f64>,
    order: usize,
) -> Result<FilterSpec, SignalError> {
    match kind {
        "lowpass" => {
            let fc = lowcut.or(highcut).ok_or_else(|| {
                SignalError::InvalidCutoffFrequency(
                    "lowpass requires low_cutoff (or high_cutoff)".to_string(),
                )
            })?;
            Ok(FilterSpec::lowpass(sampling_rate, fc, order))
        }
        "highpass" => {
            let fc = lowcut.or(highcut).ok_or_else(|| {
                SignalError::InvalidCutoffFrequency(
                    "highpass requires low_cutoff (or high_cutoff)".to_string(),
                )
            })?;
            Ok(FilterSpec::highpass(sampling_rate, fc, order))
        }
        "bandpass" => {
            let (lc, hc) = lowcut.zip(highcut).ok_or_else(|| {
                SignalError::InvalidCutoffFrequency(
                    "bandpass requires both low_cutoff and high_cutoff".to_string(),
                )
            })?;
            Ok(FilterSpec::bandpass(sampling_rate, lc, hc, order))
        }
        "notch" => {
            let (lc, hc) = lowcut.zip(highcut).ok_or_else(|| {
                SignalError::InvalidCutoffFrequency(
                    "notch requires both low_cutoff and high_cutoff".to_string(),
                )
            })?;
            Ok(FilterSpec::notch(sampling_rate, lc, hc, order))
        }
        other => Err(SignalError::InvalidCutoffFrequency(format!(
            "Unknown filter kind: {} (expected lowpass|highpass|bandpass|notch)",
            other
        ))),
    }
}

#[pyfunction]
#[pyo3(signature = (signal, sampling_rate, lowcut=None, highcut=None, order=1, kind="bandpass"))]
pub fn filter<'py>(
    py: Python<'py>,
    signal: PyReadonlyArray1<'py, f64>,
    sampling_rate: f64,
    lowcut: Option<f64>,
    highcut: Option<f64>,
    order: usize,
    kind: &str,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let array_view = signal.as_array();
    let arr = array_view.to_owned();

    let out = py
        .detach(|| {
            let spec = build_spec(kind, sampling_rate, lowcut, highcut, order)?;
            lamina::signal::filter::signal_filtfilt(&arr, &spec)
        })
        .map_err(map_signal_error)?;

    Ok(out.into_pyarray(py))
}

#[pyfunction]
#[pyo3(signature = (signal, sampling_rate, lowcut=None, highcut=None, order=1, kind="bandpass"))]
pub fn filtfilt<'py>(
    py: Python<'py>,
    signal: PyReadonlyArray1<'py, f64>,
    sampling_rate: f64,
    lowcut: Option<f64>,
    highcut: Option<f64>,
    order: usize,
    kind: &str,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    filter(py, signal, sampling_rate, lowcut, highcut, order, kind)
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

#[pyfunction]
pub fn resample_poly<'py>(
    py: Python<'py>,
    signal: PyReadonlyArray1<'py, f64>,
    up: usize,
    down: usize,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let array_view = signal.as_array();
    let arr = array_view.to_owned();

    let out = py
        .detach(|| lamina::signal::resample::signal_resample_poly(&arr, up, down))
        .map_err(map_signal_error)?;

    Ok(out.into_pyarray(py))
}

#[pyfunction]
#[pyo3(signature = (signal, window_samples, step_samples, tail_policy="drop"))]
pub fn segment_signal<'py>(
    py: Python<'py>,
    signal: PyReadonlyArray1<'py, f64>,
    window_samples: usize,
    step_samples: usize,
    tail_policy: &str,
) -> PyResult<Vec<Bound<'py, PyArray1<f64>>>> {
    let array_view = signal.as_array();
    let arr = array_view.to_owned();

    let policy = match tail_policy {
        "drop" | "drop_incomplete" => lamina::signal::segment::IncompleteTailPolicy::DropIncomplete,
        "pad" | "pad_zeros" => lamina::signal::segment::IncompleteTailPolicy::PadZeros,
        "keep" | "keep_partial" => lamina::signal::segment::IncompleteTailPolicy::KeepPartial,
        other => {
            return Err(pyo3::exceptions::PyValueError::new_err(format!(
                "Unknown tail_policy: {} (expected drop|pad|keep)",
                other
            )));
        }
    };

    let segments = py
        .detach(|| lamina::signal::segment::signal_segment(&arr, window_samples, step_samples, policy))
        .map_err(map_signal_error)?;

    let py_segs = segments
        .into_iter()
        .map(|seg| seg.into_pyarray(py))
        .collect();

    Ok(py_segs)
}

#[pyfunction]
pub fn remove_dc<'py>(
    py: Python<'py>,
    signal: PyReadonlyArray1<'py, f64>,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let array_view = signal.as_array();
    let arr = array_view.to_owned();

    let out = py
        .detach(|| lamina::signal::dc::signal_remove_dc(&arr))
        .map_err(map_signal_error)?;

    Ok(out.into_pyarray(py))
}

#[pyfunction]
#[pyo3(signature = (signal, feature_range=(0.0, 1.0), degenerate_policy="midpoint"))]
pub fn minmax_scale<'py>(
    py: Python<'py>,
    signal: PyReadonlyArray1<'py, f64>,
    feature_range: (f64, f64),
    degenerate_policy: &str,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let array_view = signal.as_array();
    let arr = array_view.to_owned();

    let policy = match degenerate_policy {
        "error" => lamina::signal::normalize::DegeneratePolicy::Error,
        "zero" => lamina::signal::normalize::DegeneratePolicy::Zero,
        "midpoint" => lamina::signal::normalize::DegeneratePolicy::Midpoint,
        other => {
            return Err(pyo3::exceptions::PyValueError::new_err(format!(
                "Unknown degenerate_policy: {} (expected error|zero|midpoint)",
                other
            )));
        }
    };

    let out = py
        .detach(|| lamina::signal::normalize::signal_minmax(&arr, feature_range, policy))
        .map_err(map_signal_error)?;

    Ok(out.into_pyarray(py))
}

