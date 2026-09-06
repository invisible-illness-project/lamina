use crate::error::map_signal_error;
use numpy::PyReadonlyArray1;
use pyo3::prelude::*;

#[pyclass]
#[derive(Clone)]
pub struct PyFeatureConfig {
    pub inner: lamina::features::FeatureConfig,
}

#[pymethods]
impl PyFeatureConfig {
    #[new]
    pub fn new() -> Self {
        Self {
            inner: lamina::features::FeatureConfig::default(),
        }
    }
}

impl Default for PyFeatureConfig {
    fn default() -> Self {
        Self::new()
    }
}

#[pyclass]
#[derive(Clone)]
pub struct PyMultimodalInput {
    pub inner: lamina::features::MultimodalInput,
}

#[pymethods]
impl PyMultimodalInput {
    #[new]
    pub fn new() -> Self {
        Self {
            inner: lamina::features::MultimodalInput::new(),
        }
    }

    #[pyo3(signature = (r_peaks, sampling_rate, offset_sec=0.0))]
    pub fn with_ecg(
        mut self_: PyRefMut<'_, Self>,
        r_peaks: Vec<usize>,
        sampling_rate: f64,
        offset_sec: f64,
    ) -> PyResult<()> {
        let input = self_.inner.clone();
        let updated = input
            .with_ecg(r_peaks, sampling_rate, offset_sec)
            .map_err(map_signal_error)?;
        self_.inner = updated;
        Ok(())
    }

    #[pyo3(signature = (peaks, sampling_rate, offset_sec=0.0))]
    pub fn with_ppg(
        mut self_: PyRefMut<'_, Self>,
        peaks: Vec<usize>,
        sampling_rate: f64,
        offset_sec: f64,
    ) -> PyResult<()> {
        let input = self_.inner.clone();
        let updated = input
            .with_ppg(peaks, sampling_rate, offset_sec)
            .map_err(map_signal_error)?;
        self_.inner = updated;
        Ok(())
    }

    #[pyo3(signature = (tonic, phasic, scr_events, sampling_rate, offset_sec=0.0))]
    pub fn with_eda(
        mut self_: PyRefMut<'_, Self>,
        tonic: PyReadonlyArray1<'_, f64>,
        phasic: PyReadonlyArray1<'_, f64>,
        scr_events: Vec<crate::eda::PyScrEvent>,
        sampling_rate: f64,
        offset_sec: f64,
    ) -> PyResult<()> {
        let tonic_arr = tonic.as_array().to_owned();
        let phasic_arr = phasic.as_array().to_owned();
        let rust_scrs: Vec<lamina::eda::ScrEvent> = scr_events
            .into_iter()
            .map(|e| lamina::eda::ScrEvent {
                onset_index: e.onset_index,
                peak_index: e.peak_index,
                amplitude: e.amplitude,
                rise_time_sec: e.rise_time_sec,
            })
            .collect();

        let input = self_.inner.clone();
        let updated = input
            .with_eda(tonic_arr, phasic_arr, rust_scrs, sampling_rate, offset_sec)
            .map_err(map_signal_error)?;
        self_.inner = updated;
        Ok(())
    }
}

impl Default for PyMultimodalInput {
    fn default() -> Self {
        Self::new()
    }
}

#[pyclass]
#[derive(Clone)]
pub struct PyMultimodalFeatureVector {
    pub inner: lamina::features::MultimodalFeatureVector,
}

#[pymethods]
impl PyMultimodalFeatureVector {
    #[getter]
    pub fn start_time_sec(&self) -> f64 {
        self.inner.window.start_time_sec
    }

    #[getter]
    pub fn end_time_sec(&self) -> f64 {
        self.inner.window.end_time_sec
    }

    #[getter]
    pub fn mean_hr_bpm(&self) -> Option<f64> {
        self.inner.cardiac.mean_hr_bpm
    }

    #[getter]
    pub fn sdnn_ms(&self) -> Option<f64> {
        self.inner.cardiac.sdnn_ms
    }

    #[getter]
    pub fn rmssd_ms(&self) -> Option<f64> {
        self.inner.cardiac.rmssd_ms
    }

    #[getter]
    pub fn mean_tonic_us(&self) -> Option<f64> {
        self.inner.eda.mean_tonic_us
    }

    #[getter]
    pub fn mean_phasic_us(&self) -> Option<f64> {
        self.inner.eda.mean_phasic_us
    }

    #[getter]
    pub fn scr_count(&self) -> usize {
        self.inner.eda.scr_count
    }

    #[getter]
    pub fn mean_rsp_rate_bpm(&self) -> Option<f64> {
        self.inner.respiration.mean_rate_bpm
    }

    fn __repr__(&self) -> String {
        format!(
            "MultimodalFeatureVector(t=[{:.1}s - {:.1}s], HR={:?}, SDNN={:?}, Tonic={:?})",
            self.inner.window.start_time_sec,
            self.inner.window.end_time_sec,
            self.inner.cardiac.mean_hr_bpm,
            self.inner.cardiac.sdnn_ms,
            self.inner.eda.mean_tonic_us
        )
    }
}

#[pyfunction]
#[pyo3(signature = (input, config=None))]
pub fn extract_features<'py>(
    py: Python<'py>,
    input: &PyMultimodalInput,
    config: Option<&PyFeatureConfig>,
) -> PyResult<Vec<PyMultimodalFeatureVector>> {
    let rust_cfg = config.map(|c| c.inner.clone()).unwrap_or_default();
    let rust_input = input.inner.clone();

    let vectors = py
        .detach(|| lamina::features::extract_features(&rust_input, &rust_cfg))
        .map_err(map_signal_error)?;

    Ok(vectors
        .into_iter()
        .map(|v| PyMultimodalFeatureVector { inner: v })
        .collect())
}
