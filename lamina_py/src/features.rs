use crate::error::map_signal_error;
use numpy::PyReadonlyArray1;
use pyo3::prelude::*;

#[pyclass]
#[derive(Clone)]
pub struct PyWindowConfig {
    pub inner: lamina::features::WindowConfig,
}

#[pymethods]
impl PyWindowConfig {
    #[new]
    #[pyo3(signature = (window_duration_sec=None, step_sec=None, min_coverage=None))]
    pub fn new(
        window_duration_sec: Option<f64>,
        step_sec: Option<f64>,
        min_coverage: Option<f64>,
    ) -> Self {
        let mut inner = lamina::features::WindowConfig::default();
        if let Some(v) = window_duration_sec {
            inner.window_duration_sec = v;
        }
        if let Some(v) = step_sec {
            inner.step_sec = v;
        }
        if let Some(v) = min_coverage {
            inner.min_coverage = v;
        }
        Self { inner }
    }
}

#[pyclass]
#[derive(Clone)]
pub struct PyFeatureConfig {
    pub inner: lamina::features::FeatureConfig,
}

#[pymethods]
impl PyFeatureConfig {
    #[new]
    #[pyo3(signature = (window=None, min_beats=None, min_respiration_cycles=None, min_scr_events=None, require_cardiac=None, require_respiration=None, require_eda=None))]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        window: Option<PyWindowConfig>,
        min_beats: Option<usize>,
        min_respiration_cycles: Option<usize>,
        min_scr_events: Option<usize>,
        require_cardiac: Option<bool>,
        require_respiration: Option<bool>,
        require_eda: Option<bool>,
    ) -> Self {
        let mut inner = lamina::features::FeatureConfig::default();
        if let Some(w) = window {
            inner.window = w.inner;
        }
        if let Some(v) = min_beats {
            inner.min_beats = v;
        }
        if let Some(v) = min_respiration_cycles {
            inner.min_respiration_cycles = v;
        }
        if let Some(v) = min_scr_events {
            inner.min_scr_events = v;
        }
        if let Some(v) = require_cardiac {
            inner.require_cardiac = v;
        }
        if let Some(v) = require_respiration {
            inner.require_respiration = v;
        }
        if let Some(v) = require_eda {
            inner.require_eda = v;
        }
        Self { inner }
    }
}

impl Default for PyFeatureConfig {
    fn default() -> Self {
        Self {
            inner: lamina::features::FeatureConfig::default(),
        }
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

    #[pyo3(signature = (cycles, sampling_rate, offset_sec=0.0))]
    pub fn with_rsp(
        mut self_: PyRefMut<'_, Self>,
        cycles: Vec<crate::rsp::PyRespirationCycle>,
        sampling_rate: f64,
        offset_sec: f64,
    ) -> PyResult<()> {
        let rust_cycles: Vec<lamina::rsp::RespirationCycle> = cycles
            .into_iter()
            .map(|c| lamina::rsp::RespirationCycle {
                inspiration_index: c.inspiration_index,
                expiration_index: c.expiration_index,
                next_inspiration_index: c.next_inspiration_index,
                duration_sec: c.duration_sec,
                respiratory_rate_bpm: c.respiratory_rate_bpm,
                amplitude: c.amplitude,
            })
            .collect();

        let input = self_.inner.clone();
        let updated = input
            .with_rsp(rust_cycles, sampling_rate, offset_sec)
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
    pub fn duration_sec(&self) -> f64 {
        self.inner.window.duration_sec
    }

    // Cardiac
    #[getter]
    pub fn mean_hr_bpm(&self) -> Option<f64> {
        self.inner.cardiac.mean_hr_bpm
    }
    #[getter]
    pub fn median_hr_bpm(&self) -> Option<f64> {
        self.inner.cardiac.median_hr_bpm
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
    pub fn pnn50(&self) -> Option<f64> {
        self.inner.cardiac.pnn50
    }
    #[getter]
    pub fn rr_mean_ms(&self) -> Option<f64> {
        self.inner.cardiac.rr_mean_ms
    }
    #[getter]
    pub fn rr_std_ms(&self) -> Option<f64> {
        self.inner.cardiac.rr_std_ms
    }

    #[getter]
    pub fn beat_count(&self) -> usize {
        self.inner.cardiac.beat_count
    }

    // EDA
    #[getter]
    pub fn mean_tonic_us(&self) -> Option<f64> {
        self.inner.eda.mean_tonic_us
    }
    #[getter]
    pub fn median_tonic_us(&self) -> Option<f64> {
        self.inner.eda.median_tonic_us
    }
    #[getter]
    pub fn tonic_std_us(&self) -> Option<f64> {
        self.inner.eda.tonic_std_us
    }
    #[getter]
    pub fn mean_phasic_us(&self) -> Option<f64> {
        self.inner.eda.mean_phasic_us
    }
    #[getter]
    pub fn phasic_std_us(&self) -> Option<f64> {
        self.inner.eda.phasic_std_us
    }

    #[getter]
    pub fn scr_count(&self) -> usize {
        self.inner.eda.scr_count
    }

    #[getter]
    pub fn scr_rate_per_min(&self) -> Option<f64> {
        self.inner.eda.scr_rate_per_min
    }
    #[getter]
    pub fn mean_scr_amplitude_us(&self) -> Option<f64> {
        self.inner.eda.mean_scr_amplitude_us
    }
    #[getter]
    pub fn median_scr_amplitude_us(&self) -> Option<f64> {
        self.inner.eda.median_scr_amplitude_us
    }
    #[getter]
    pub fn mean_scr_rise_time_sec(&self) -> Option<f64> {
        self.inner.eda.mean_scr_rise_time_sec
    }

    // Respiration
    #[getter]
    pub fn mean_rsp_rate_bpm(&self) -> Option<f64> {
        self.inner.respiration.mean_rate_bpm
    }
    #[getter]
    pub fn median_rsp_rate_bpm(&self) -> Option<f64> {
        self.inner.respiration.median_rate_bpm
    }
    #[getter]
    pub fn rsp_rate_std_bpm(&self) -> Option<f64> {
        self.inner.respiration.rate_std_bpm
    }
    #[getter]
    pub fn mean_cycle_duration_sec(&self) -> Option<f64> {
        self.inner.respiration.mean_cycle_duration_sec
    }

    #[getter]
    pub fn cycle_count(&self) -> usize {
        self.inner.respiration.cycle_count
    }

    #[getter]
    pub fn mean_rsp_amplitude(&self) -> Option<f64> {
        self.inner.respiration.mean_amplitude
    }
    #[getter]
    pub fn rsp_amplitude_std(&self) -> Option<f64> {
        self.inner.respiration.amplitude_std
    }

    // Coupling
    #[getter]
    pub fn rsa_amplitude_bpm(&self) -> Option<f64> {
        self.inner.coupling.rsa_amplitude_bpm
    }
    #[getter]
    pub fn rsa_amplitude_rr_sec(&self) -> Option<f64> {
        self.inner.coupling.rsa_amplitude_rr_sec
    }
    #[getter]
    pub fn cardiac_respiratory_concentration(&self) -> Option<f64> {
        self.inner.coupling.cardiac_respiratory_concentration
    }
    #[getter]
    pub fn cardiac_respiratory_mean_phase(&self) -> Option<f64> {
        self.inner.coupling.cardiac_respiratory_mean_phase
    }
    #[getter]
    pub fn mean_pulse_delay_sec(&self) -> Option<f64> {
        self.inner.coupling.mean_pulse_delay_sec
    }
    #[getter]
    pub fn pulse_delay_std_sec(&self) -> Option<f64> {
        self.inner.coupling.pulse_delay_std_sec
    }

    #[getter]
    pub fn scr_cardiac_association_count(&self) -> usize {
        self.inner.coupling.scr_cardiac_association_count
    }

    // Quality
    #[getter]
    pub fn coverage(&self) -> f64 {
        self.inner.quality.coverage
    }

    #[getter]
    pub fn coverage_overall(&self) -> f64 {
        self.inner.quality.modality_coverage.overall
    }

    #[getter]
    pub fn coverage_ecg(&self) -> Option<f64> {
        self.inner.quality.modality_coverage.ecg
    }

    #[getter]
    pub fn coverage_ppg(&self) -> Option<f64> {
        self.inner.quality.modality_coverage.ppg
    }

    #[getter]
    pub fn coverage_eda(&self) -> Option<f64> {
        self.inner.quality.modality_coverage.eda
    }

    #[getter]
    pub fn coverage_rsp(&self) -> Option<f64> {
        self.inner.quality.modality_coverage.rsp
    }

    #[getter]
    pub fn cardiac_valid(&self) -> bool {
        self.inner.quality.cardiac_valid
    }

    #[getter]
    pub fn eda_valid(&self) -> bool {
        self.inner.quality.eda_valid
    }

    #[getter]
    pub fn respiration_valid(&self) -> bool {
        self.inner.quality.respiration_valid
    }

    #[getter]
    pub fn coupling_valid(&self) -> bool {
        self.inner.quality.coupling_valid
    }

    #[getter]
    pub fn usable_feature_count(&self) -> usize {
        self.inner.quality.usable_feature_count
    }

    #[getter]
    pub fn total_feature_count(&self) -> usize {
        self.inner.quality.total_feature_count
    }

    #[getter]
    pub fn quality_issues(&self) -> Vec<String> {
        self.inner
            .quality
            .issues
            .iter()
            .map(|i| format!("{:?}", i))
            .collect()
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
