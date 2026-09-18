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
    pub fn new(timestamp_sec: f64, width: usize, height: usize, data: Vec<u8>) -> PyResult<Self> {
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
        let stream =
            lamina::rppg::VideoStream::new(rust_frames, nominal_fps).map_err(map_signal_error)?;
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

// ---- Config enums + nested configs ----

#[pyclass(eq, eq_int)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyRppgAlgorithmId {
    GreenChannel,
    Chrom,
    Pos,
}

impl From<PyRppgAlgorithmId> for lamina::rppg::RppgAlgorithmId {
    fn from(v: PyRppgAlgorithmId) -> Self {
        match v {
            PyRppgAlgorithmId::GreenChannel => lamina::rppg::RppgAlgorithmId::GreenChannel,
            PyRppgAlgorithmId::Chrom => lamina::rppg::RppgAlgorithmId::Chrom,
            PyRppgAlgorithmId::Pos => lamina::rppg::RppgAlgorithmId::Pos,
        }
    }
}

#[pyclass(eq, eq_int)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PySignalPolarity {
    Normal,
    Inverted,
    AutoDetect,
}

impl From<PySignalPolarity> for lamina::rppg::SignalPolarity {
    fn from(v: PySignalPolarity) -> Self {
        match v {
            PySignalPolarity::Normal => lamina::rppg::SignalPolarity::Normal,
            PySignalPolarity::Inverted => lamina::rppg::SignalPolarity::Inverted,
            PySignalPolarity::AutoDetect => lamina::rppg::SignalPolarity::AutoDetect,
        }
    }
}

#[pyclass]
#[derive(Clone)]
pub struct PyRppgWindowConfig {
    #[pyo3(get, set)]
    pub window_sec: f64,
    #[pyo3(get, set)]
    pub step_sec: f64,
    #[pyo3(get, set)]
    pub min_window_fraction: f64,
}

#[pymethods]
impl PyRppgWindowConfig {
    #[new]
    #[pyo3(signature = (window_sec=3.0, step_sec=0.5, min_window_fraction=0.8))]
    pub fn new(window_sec: f64, step_sec: f64, min_window_fraction: f64) -> Self {
        Self {
            window_sec,
            step_sec,
            min_window_fraction,
        }
    }

    pub fn to_rust(&self) -> lamina::rppg::RppgWindowConfig {
        lamina::rppg::RppgWindowConfig {
            window_sec: self.window_sec,
            step_sec: self.step_sec,
            min_window_fraction: self.min_window_fraction,
        }
    }
}

#[pyclass]
#[derive(Clone)]
pub struct PyRppgPreprocessingConfig {
    #[pyo3(get, set)]
    pub normalize_channels: bool,
    #[pyo3(get, set)]
    pub detrend: bool,
}

#[pymethods]
impl PyRppgPreprocessingConfig {
    #[new]
    #[pyo3(signature = (normalize_channels=true, detrend=true))]
    pub fn new(normalize_channels: bool, detrend: bool) -> Self {
        Self {
            normalize_channels,
            detrend,
        }
    }

    pub fn to_rust(&self) -> lamina::rppg::RppgPreprocessingConfig {
        lamina::rppg::RppgPreprocessingConfig {
            normalize_channels: self.normalize_channels,
            detrend: self.detrend,
        }
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
    #[pyo3(signature = (algorithm=None, min_quality=None, minimum_roi_pixels=None, max_gap_sec=None, window=None, preprocessing=None, signal_band_hz=None, polarity=None))]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        algorithm: Option<PyRppgAlgorithmId>,
        min_quality: Option<f64>,
        minimum_roi_pixels: Option<usize>,
        max_gap_sec: Option<f64>,
        window: Option<PyRppgWindowConfig>,
        preprocessing: Option<PyRppgPreprocessingConfig>,
        signal_band_hz: Option<(f64, f64)>,
        polarity: Option<PySignalPolarity>,
    ) -> PyResult<Self> {
        let mut inner = lamina::rppg::RppgConfig::default();
        if let Some(v) = algorithm {
            inner.algorithm = v.into();
        }
        if let Some(v) = min_quality {
            inner.min_quality = v;
        }
        if let Some(v) = minimum_roi_pixels {
            inner.minimum_roi_pixels = v;
        }
        if let Some(v) = max_gap_sec {
            inner.max_gap_sec = v;
        }
        if let Some(v) = window {
            inner.window = v.to_rust();
        }
        if let Some(v) = preprocessing {
            inner.preprocessing = v.to_rust();
        }
        if let Some(v) = signal_band_hz {
            inner.signal_band_hz = v;
        }
        if let Some(v) = polarity {
            inner.polarity = v.into();
        }
        inner.validate().map_err(map_signal_error)?;
        Ok(Self { inner })
    }
}

impl Default for PyRppgConfig {
    fn default() -> Self {
        Self {
            inner: lamina::rppg::RppgConfig::default(),
        }
    }
}

#[pyclass]
#[derive(Clone)]
pub struct PyRppgSegmentQuality {
    #[pyo3(get)]
    pub start_sec: f64,
    #[pyo3(get)]
    pub end_sec: f64,
    #[pyo3(get)]
    pub overall: f64,
    #[pyo3(get)]
    pub roi_quality: f64,
    #[pyo3(get)]
    pub motion_quality: f64,
    #[pyo3(get)]
    pub illumination_quality: f64,
    #[pyo3(get)]
    pub signal_quality: f64,
    #[pyo3(get)]
    pub valid_fraction: f64,
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
    #[pyo3(get)]
    pub valid_fraction: f64,
    #[pyo3(get)]
    pub algorithm: String,
    #[pyo3(get)]
    pub segments: Vec<PyRppgSegmentQuality>,
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

    let segments = res
        .quality
        .segments
        .iter()
        .map(|s| PyRppgSegmentQuality {
            start_sec: s.start_sec,
            end_sec: s.end_sec,
            overall: s.overall,
            roi_quality: s.roi_quality,
            motion_quality: s.motion_quality,
            illumination_quality: s.illumination_quality,
            signal_quality: s.signal_quality,
            valid_fraction: s.valid_fraction,
        })
        .collect();

    Ok(PyRppgSignal {
        timestamps_sec: ts_py,
        waveform: wf_py,
        sampling_rate_hz: res.sampling_rate_hz,
        overall_quality: res.quality.overall,
        valid_fraction: res.quality.valid_fraction,
        algorithm: format!("{:?}", res.algorithm),
        segments,
    })
}
