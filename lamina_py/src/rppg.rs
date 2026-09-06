use crate::error::map_signal_error;
use numpy::{IntoPyArray, PyArray1};
use pyo3::prelude::*;

#[pyclass]
#[derive(Clone)]
pub struct PyVideoFrame {
    pub inner: lamina::rppg::VideoFrame,
}

#[pymethods]
impl PyVideoFrame {
    #[new]
    pub fn new(
        timestamp_sec: f64,
        width: usize,
        height: usize,
        data: Vec<u8>,
    ) -> PyResult<Self> {
        let frame = lamina::rppg::VideoFrame::new(timestamp_sec, width, height, data)
            .map_err(map_signal_error)?;
        Ok(Self { inner: frame })
    }

    #[getter]
    pub fn timestamp_sec(&self) -> f64 {
        self.inner.timestamp_sec
    }

    #[getter]
    pub fn width(&self) -> usize {
        self.inner.width
    }

    #[getter]
    pub fn height(&self) -> usize {
        self.inner.height
    }
}

#[pyclass]
#[derive(Clone)]
pub struct PyVideoStream {
    pub inner: lamina::rppg::VideoStream,
}

#[pymethods]
impl PyVideoStream {
    #[new]
    #[pyo3(signature = (frames, nominal_fps=None))]
    pub fn new(frames: Vec<PyVideoFrame>, nominal_fps: Option<f64>) -> PyResult<Self> {
        let rust_frames: Vec<lamina::rppg::VideoFrame> =
            frames.into_iter().map(|f| f.inner).collect();
        let stream = lamina::rppg::VideoStream::new(rust_frames, nominal_fps)
            .map_err(map_signal_error)?;
        Ok(Self { inner: stream })
    }
}

#[pyclass]
#[derive(Clone)]
pub struct PyRoi {
    pub inner: lamina::rppg::Roi,
}

#[pymethods]
impl PyRoi {
    #[new]
    pub fn new(x: usize, y: usize, width: usize, height: usize) -> PyResult<Self> {
        let roi = lamina::rppg::Roi::new(x, y, width, height).map_err(map_signal_error)?;
        Ok(Self { inner: roi })
    }

    #[getter]
    pub fn x(&self) -> usize {
        self.inner.x
    }
    #[getter]
    pub fn y(&self) -> usize {
        self.inner.y
    }
    #[getter]
    pub fn width(&self) -> usize {
        self.inner.width
    }
    #[getter]
    pub fn height(&self) -> usize {
        self.inner.height
    }
}

#[pyclass]
#[derive(Clone)]
pub struct PyRppgConfig {
    pub inner: lamina::rppg::RppgConfig,
}

#[pymethods]
impl PyRppgConfig {
    #[new]
    pub fn new() -> Self {
        Self {
            inner: lamina::rppg::RppgConfig::default(),
        }
    }
}

impl Default for PyRppgConfig {
    fn default() -> Self {
        Self::new()
    }
}

#[pyclass]
pub struct PyRppgSignal {
    #[pyo3(get)]
    pub timestamps_sec: Py<PyAny>,
    #[pyo3(get)]
    pub waveform: Py<PyAny>,
    #[pyo3(get)]
    pub sampling_rate_hz: f64,
    #[pyo3(get)]
    pub overall_quality: f64,
}

#[pyfunction]
#[pyo3(signature = (video, roi, config=None))]
pub fn extract_rppg<'py>(
    py: Python<'py>,
    video: &PyVideoStream,
    roi: &PyRoi,
    config: Option<&PyRppgConfig>,
) -> PyResult<PyRppgSignal> {
    let static_roi = lamina::rppg::StaticRoi::new(roi.inner);
    let rust_cfg = config.map(|c| c.inner.clone()).unwrap_or_default();

    let stream = video.inner.clone();
    let res = py
        .detach(|| lamina::rppg::extract_rppg(&stream, &static_roi, &rust_cfg))
        .map_err(map_signal_error)?;

    let timestamps_nd = ndarray::Array1::from_vec(res.timestamps_sec);
    let waveform_nd = ndarray::Array1::from_vec(res.waveform);

    let ts_py = timestamps_nd.into_pyarray(py).into_any().unbind();
    let wf_py = waveform_nd.into_pyarray(py).into_any().unbind();

    Ok(PyRppgSignal {
        timestamps_sec: ts_py,
        waveform: wf_py,
        sampling_rate_hz: res.sampling_rate_hz,
        overall_quality: res.quality.overall,
    })
}
