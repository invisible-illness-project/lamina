use crate::error::map_signal_error;
use crate::features::PyMultimodalFeatureVector;
use lamina::autonomic::{
    ActivationWeights, AutonomicBaseline, AutonomicEstimator, AutonomicEstimatorConfig,
    AutonomicState, AutonomicStateSeries, BaselineFeatureStats, ConfidenceWeights, FeatureDirection,
    NormalizationConfig, NormalizationMethod, QualityConfig, RecoveryConfig, RegulationWeights,
    SmoothingConfig, StateConfidence,
};
use pyo3::prelude::*;

// ---- Enums ----

#[pyclass(eq, eq_int)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyNormalizationMethod {
    ZScore,
    RobustMedianMad,
}

impl From<PyNormalizationMethod> for NormalizationMethod {
    fn from(v: PyNormalizationMethod) -> Self {
        match v {
            PyNormalizationMethod::ZScore => NormalizationMethod::ZScore,
            PyNormalizationMethod::RobustMedianMad => NormalizationMethod::RobustMedianMad,
        }
    }
}

#[pyclass(eq, eq_int)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyFeatureDirection {
    Positive,
    Negative,
}

impl From<PyFeatureDirection> for FeatureDirection {
    fn from(v: PyFeatureDirection) -> Self {
        match v {
            PyFeatureDirection::Positive => FeatureDirection::Positive,
            PyFeatureDirection::Negative => FeatureDirection::Negative,
        }
    }
}

// ---- Leaf configs ----

#[pyclass]
#[derive(Clone)]
pub struct PyNormalizationConfig {
    pub inner: NormalizationConfig,
}

#[pymethods]
impl PyNormalizationConfig {
    #[new]
    #[pyo3(signature = (method=None, min_baseline_samples=None, bounded_scale=None, min_scale=None, mad_multiplier=None))]
    pub fn new(
        method: Option<PyNormalizationMethod>,
        min_baseline_samples: Option<usize>,
        bounded_scale: Option<f64>,
        min_scale: Option<f64>,
        mad_multiplier: Option<f64>,
    ) -> Self {
        let mut inner = NormalizationConfig::default();
        if let Some(v) = method {
            inner.method = v.into();
        }
        if let Some(v) = min_baseline_samples {
            inner.min_baseline_samples = v;
        }
        if let Some(v) = bounded_scale {
            inner.bounded_scale = v;
        }
        if let Some(v) = min_scale {
            inner.min_scale = v;
        }
        if let Some(v) = mad_multiplier {
            inner.mad_multiplier = v;
        }
        Self { inner }
    }
}

macro_rules! leaf_config {
    ($py:ident, $rust:ident, $($field:ident : $ty:ty),* $(,)?) => {
        #[pyclass]
        #[derive(Clone)]
        pub struct $py {
            pub inner: $rust,
        }

        #[pymethods]
        impl $py {
            #[new]
            #[pyo3(signature = ($($field=None),*))]
            #[allow(clippy::too_many_arguments)]
            pub fn new($($field: Option<$ty>),*) -> Self {
                let mut inner = $rust::default();
                $(if let Some(v) = $field { inner.$field = v; })*
                Self { inner }
            }
        }
    };
}

leaf_config!(PyActivationWeights, ActivationWeights,
    hr_weight: f64, eda_phasic_weight: f64, scr_rate_weight: f64, rsp_rate_weight: f64,
);
leaf_config!(PyRegulationWeights, RegulationWeights,
    cardiac_variability_weight: f64, resphr_coupling_weight: f64, phase_coupling_weight: f64,
);
leaf_config!(PyRecoveryConfig, RecoveryConfig,
    variability_weight: f64, heart_rate_weight: f64,
);
leaf_config!(PyConfidenceWeights, ConfidenceWeights,
    min_beats: usize, min_cycles: usize, quality_factor: f64,
    cardiac_weight: f64, eda_weight: f64, rsp_weight: f64, coupling_weight: f64,
);

#[pyclass]
#[derive(Clone)]
pub struct PyQualityConfig {
    pub inner: QualityConfig,
}

#[pymethods]
impl PyQualityConfig {
    #[new]
    #[pyo3(signature = (min_coverage=None, require_cardiac_validity=None, require_respiration_for_resphrv=None, confidence=None))]
    pub fn new(
        min_coverage: Option<f64>,
        require_cardiac_validity: Option<bool>,
        require_respiration_for_resphrv: Option<bool>,
        confidence: Option<PyConfidenceWeights>,
    ) -> Self {
        let mut inner = QualityConfig::default();
        if let Some(v) = min_coverage {
            inner.min_coverage = v;
        }
        if let Some(v) = require_cardiac_validity {
            inner.require_cardiac_validity = v;
        }
        if let Some(v) = require_respiration_for_resphrv {
            inner.require_respiration_for_resphrv = v;
        }
        if let Some(c) = confidence {
            inner.confidence = c.inner;
        }
        Self { inner }
    }
}

#[pyclass]
#[derive(Clone)]
pub struct PySmoothingConfig {
    pub inner: SmoothingConfig,
}

#[pymethods]
impl PySmoothingConfig {
    #[new]
    #[pyo3(signature = (alpha=None))]
    pub fn new(alpha: Option<f64>) -> Self {
        let inner = match alpha {
            Some(a) => SmoothingConfig { alpha: a },
            None => SmoothingConfig::default(),
        };
        Self { inner }
    }
}

// ---- Master config ----

#[pyclass]
#[derive(Clone)]
pub struct PyAutonomicEstimatorConfig {
    pub inner: AutonomicEstimatorConfig,
}

#[pymethods]
impl PyAutonomicEstimatorConfig {
    #[new]
    #[pyo3(signature = (normalization=None, activation=None, regulation=None, recovery=None, quality=None, smoothing=None))]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        normalization: Option<PyNormalizationConfig>,
        activation: Option<PyActivationWeights>,
        regulation: Option<PyRegulationWeights>,
        recovery: Option<PyRecoveryConfig>,
        quality: Option<PyQualityConfig>,
        smoothing: Option<PySmoothingConfig>,
    ) -> PyResult<Self> {
        let mut inner = AutonomicEstimatorConfig::default();
        if let Some(c) = normalization {
            inner.normalization = c.inner;
        }
        if let Some(c) = activation {
            inner.activation = c.inner;
        }
        if let Some(c) = regulation {
            inner.regulation = c.inner;
        }
        if let Some(c) = recovery {
            inner.recovery = c.inner;
        }
        if let Some(c) = quality {
            inner.quality = c.inner;
        }
        if let Some(c) = smoothing {
            inner.smoothing = Some(c.inner);
        }
        inner.validate().map_err(map_signal_error)?;
        Ok(Self { inner })
    }
}

// ---- Baseline ----

#[pyclass]
#[derive(Clone)]
pub struct PyBaselineFeatureStats {
    #[pyo3(get)]
    pub mean: Option<f64>,
    #[pyo3(get)]
    pub std: Option<f64>,
    #[pyo3(get)]
    pub median: Option<f64>,
    #[pyo3(get)]
    pub mad: Option<f64>,
    #[pyo3(get)]
    pub sample_count: usize,
    #[pyo3(get)]
    pub is_valid: bool,
}

impl From<&BaselineFeatureStats> for PyBaselineFeatureStats {
    fn from(s: &BaselineFeatureStats) -> Self {
        Self {
            mean: s.mean,
            std: s.std,
            median: s.median,
            mad: s.mad,
            sample_count: s.sample_count,
            is_valid: s.is_valid,
        }
    }
}

#[pyclass]
pub struct PyAutonomicBaseline {
    pub inner: AutonomicBaseline,
}

#[pymethods]
impl PyAutonomicBaseline {
    #[pyo3(name = "from_features")]
    #[staticmethod]
    #[pyo3(signature = (baseline_features, normalization=None))]
    pub fn from_features(
        baseline_features: Vec<PyMultimodalFeatureVector>,
        normalization: Option<PyNormalizationConfig>,
    ) -> PyResult<Self> {
        let rust_features: Vec<lamina::features::MultimodalFeatureVector> =
            baseline_features.into_iter().map(|f| f.inner).collect();
        let norm_cfg = normalization.map(|c| c.inner).unwrap_or_default();
        let baseline =
            AutonomicBaseline::fit(&rust_features, &norm_cfg).map_err(map_signal_error)?;
        Ok(Self { inner: baseline })
    }

    #[getter]
    pub fn hr_bpm_stats(&self) -> PyBaselineFeatureStats {
        PyBaselineFeatureStats::from(&self.inner.hr_bpm_stats)
    }

    #[getter]
    pub fn sdnn_ms_stats(&self) -> PyBaselineFeatureStats {
        PyBaselineFeatureStats::from(&self.inner.sdnn_ms_stats)
    }

    #[getter]
    pub fn rmssd_ms_stats(&self) -> PyBaselineFeatureStats {
        PyBaselineFeatureStats::from(&self.inner.rmssd_ms_stats)
    }

    #[getter]
    pub fn eda_tonic_stats(&self) -> PyBaselineFeatureStats {
        PyBaselineFeatureStats::from(&self.inner.eda_tonic_stats)
    }

    #[getter]
    pub fn eda_phasic_stats(&self) -> PyBaselineFeatureStats {
        PyBaselineFeatureStats::from(&self.inner.eda_phasic_stats)
    }

    #[getter]
    pub fn scr_rate_stats(&self) -> PyBaselineFeatureStats {
        PyBaselineFeatureStats::from(&self.inner.scr_rate_stats)
    }

    #[getter]
    pub fn rsp_rate_stats(&self) -> PyBaselineFeatureStats {
        PyBaselineFeatureStats::from(&self.inner.rsp_rate_stats)
    }

    #[getter]
    pub fn rsp_amplitude_stats(&self) -> PyBaselineFeatureStats {
        PyBaselineFeatureStats::from(&self.inner.rsp_amplitude_stats)
    }

    #[getter]
    pub fn rsp_std_stats(&self) -> PyBaselineFeatureStats {
        PyBaselineFeatureStats::from(&self.inner.rsp_std_stats)
    }

    #[getter]
    pub fn rsa_bpm_stats(&self) -> PyBaselineFeatureStats {
        PyBaselineFeatureStats::from(&self.inner.rsa_bpm_stats)
    }

    #[getter]
    pub fn phase_coupling_stats(&self) -> PyBaselineFeatureStats {
        PyBaselineFeatureStats::from(&self.inner.phase_coupling_stats)
    }

    #[getter]
    pub fn pulse_delay_stats(&self) -> PyBaselineFeatureStats {
        PyBaselineFeatureStats::from(&self.inner.pulse_delay_stats)
    }
}

// ---- States ----

#[pyclass]
#[derive(Clone)]
pub struct PyCardiacState {
    #[pyo3(get)]
    pub variability_index: Option<f64>,
    #[pyo3(get)]
    pub heart_rate_index: Option<f64>,
    #[pyo3(get)]
    pub recovery_evidence: Option<f64>,
    #[pyo3(get)]
    pub beat_count: usize,
}

#[pyclass]
#[derive(Clone)]
pub struct PyElectrodermalState {
    #[pyo3(get)]
    pub tonic_level_index: Option<f64>,
    #[pyo3(get)]
    pub phasic_activation_index: Option<f64>,
    #[pyo3(get)]
    pub scr_rate_index: Option<f64>,
    #[pyo3(get)]
    pub scr_count: usize,
}

#[pyclass]
#[derive(Clone)]
pub struct PyRespiratoryState {
    #[pyo3(get)]
    pub rate_index: Option<f64>,
    #[pyo3(get)]
    pub amplitude_index: Option<f64>,
    #[pyo3(get)]
    pub regularity_index: Option<f64>,
    #[pyo3(get)]
    pub cycle_count: usize,
}

#[pyclass]
#[derive(Clone)]
pub struct PyCouplingState {
    #[pyo3(get)]
    pub resphr_coupling_index: Option<f64>,
    #[pyo3(get)]
    pub phase_coupling_index: Option<f64>,
    #[pyo3(get)]
    pub pulse_delay_index: Option<f64>,
    #[pyo3(get)]
    pub association_count: usize,
}

#[pyclass]
#[derive(Clone)]
pub struct PyStateConfidence {
    #[pyo3(get)]
    pub overall: Option<f64>,
    #[pyo3(get)]
    pub cardiac: Option<f64>,
    #[pyo3(get)]
    pub electrodermal: Option<f64>,
    #[pyo3(get)]
    pub respiratory: Option<f64>,
    #[pyo3(get)]
    pub coupling: Option<f64>,
}

impl From<&StateConfidence> for PyStateConfidence {
    fn from(c: &StateConfidence) -> Self {
        Self {
            overall: c.overall,
            cardiac: c.cardiac,
            electrodermal: c.electrodermal,
            respiratory: c.respiratory,
            coupling: c.coupling,
        }
    }
}

fn convert_state(s: &AutonomicState) -> PyAutonomicState {
    let c = &s.cardiac;
    let e = &s.electrodermal;
    let r = &s.respiratory;
    let k = &s.coupling;
    PyAutonomicState {
        timestamp: s.timestamp,
        duration_sec: s.duration_sec,
        activation_score: s.activation_score,
        regulation_score: s.regulation_score,
        cardiac: PyCardiacState {
            variability_index: c.variability_index,
            heart_rate_index: c.heart_rate_index,
            recovery_evidence: c.recovery_evidence,
            beat_count: c.beat_count,
        },
        electrodermal: PyElectrodermalState {
            tonic_level_index: e.tonic_level_index,
            phasic_activation_index: e.phasic_activation_index,
            scr_rate_index: e.scr_rate_index,
            scr_count: e.scr_count,
        },
        respiratory: PyRespiratoryState {
            rate_index: r.rate_index,
            amplitude_index: r.amplitude_index,
            regularity_index: r.regularity_index,
            cycle_count: r.cycle_count,
        },
        coupling: PyCouplingState {
            resphr_coupling_index: k.resphr_coupling_index,
            phase_coupling_index: k.phase_coupling_index,
            pulse_delay_index: k.pulse_delay_index,
            association_count: k.association_count,
        },
        confidence: PyStateConfidence::from(&s.confidence),
    }
}

#[pyclass]
#[derive(Clone)]
pub struct PyAutonomicState {
    #[pyo3(get)]
    pub timestamp: f64,
    #[pyo3(get)]
    pub duration_sec: f64,
    #[pyo3(get)]
    pub activation_score: Option<f64>,
    #[pyo3(get)]
    pub regulation_score: Option<f64>,
    #[pyo3(get)]
    pub cardiac: PyCardiacState,
    #[pyo3(get)]
    pub electrodermal: PyElectrodermalState,
    #[pyo3(get)]
    pub respiratory: PyRespiratoryState,
    #[pyo3(get)]
    pub coupling: PyCouplingState,
    #[pyo3(get)]
    pub confidence: PyStateConfidence,
}

#[pyclass]
#[derive(Clone)]
pub struct PyAutonomicStateSeries {
    #[pyo3(get)]
    pub states: Vec<PyAutonomicState>,
    #[pyo3(get)]
    pub smoothed_states: Option<Vec<PyAutonomicState>>,
    #[pyo3(get)]
    pub window_duration_sec: f64,
    #[pyo3(get)]
    pub step_sec: f64,
}

// ---- Estimator ----

#[pyclass]
pub struct PyAutonomicEstimator {
    pub inner: AutonomicEstimator,
}

#[pymethods]
impl PyAutonomicEstimator {
    #[new]
    #[pyo3(signature = (config=None))]
    pub fn new(config: Option<&PyAutonomicEstimatorConfig>) -> PyResult<Self> {
        let cfg = config.map(|c| c.inner.clone()).unwrap_or_default();
        Ok(Self {
            inner: AutonomicEstimator::new(cfg),
        })
    }

    pub fn estimate(
        &self,
        features: &PyMultimodalFeatureVector,
        baseline: &PyAutonomicBaseline,
    ) -> PyResult<PyAutonomicState> {
        let state = self
            .inner
            .estimate(&features.inner, &baseline.inner)
            .map_err(map_signal_error)?;
        Ok(convert_state(&state))
    }

    #[pyo3(signature = (feature_series, baseline))]
    pub fn estimate_series(
        &self,
        feature_series: Vec<PyMultimodalFeatureVector>,
        baseline: &PyAutonomicBaseline,
    ) -> PyResult<PyAutonomicStateSeries> {
        let rust_series: Vec<lamina::features::MultimodalFeatureVector> =
            feature_series.into_iter().map(|f| f.inner).collect();
        let series: AutonomicStateSeries = self
            .inner
            .estimate_series(&rust_series, &baseline.inner)
            .map_err(map_signal_error)?;
        let smoothed = series.smoothed_states.as_ref().map(|states| {
            states.iter().map(convert_state).collect()
        });
        Ok(PyAutonomicStateSeries {
            states: series.states.iter().map(convert_state).collect(),
            smoothed_states: smoothed,
            window_duration_sec: series.window_duration_sec,
            step_sec: series.step_sec,
        })
    }
}
