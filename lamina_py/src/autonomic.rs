use crate::error::map_signal_error;
use pyo3::prelude::*;

#[pyclass]
#[derive(Clone)]
pub struct PyAutonomicEstimatorConfig {
    pub inner: lamina::autonomic::AutonomicEstimatorConfig,
}

#[pymethods]
impl PyAutonomicEstimatorConfig {
    #[new]
    pub fn new() -> Self {
        Self {
            inner: lamina::autonomic::AutonomicEstimatorConfig::default(),
        }
    }
}

impl Default for PyAutonomicEstimatorConfig {
    fn default() -> Self {
        Self::new()
    }
}

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

#[pymethods]
impl PyBaselineFeatureStats {
    fn __repr__(&self) -> String {
        format!(
            "BaselineFeatureStats(mean={:?}, std={:?}, median={:?}, mad={:?}, count={})",
            self.mean, self.std, self.median, self.mad, self.sample_count
        )
    }
}

impl From<&lamina::autonomic::BaselineFeatureStats> for PyBaselineFeatureStats {
    fn from(s: &lamina::autonomic::BaselineFeatureStats) -> Self {
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
#[derive(Clone)]
pub struct PyAutonomicBaseline {
    pub inner: lamina::autonomic::AutonomicBaseline,
}

#[pymethods]
impl PyAutonomicBaseline {
    #[pyo3(name = "from_features")]
    #[staticmethod]
    pub fn from_features(
        baseline_features: Vec<crate::features::PyMultimodalFeatureVector>,
    ) -> PyResult<Self> {
        let rust_features: Vec<lamina::features::MultimodalFeatureVector> =
            baseline_features.into_iter().map(|f| f.inner).collect();
        let norm_cfg = lamina::autonomic::NormalizationConfig::default();
        let baseline = lamina::autonomic::AutonomicBaseline::fit(&rust_features, &norm_cfg)
            .map_err(map_signal_error)?;
        Ok(Self { inner: baseline })
    }

    #[getter]
    pub fn hr_bpm_stats(&self) -> PyBaselineFeatureStats {
        (&self.inner.hr_bpm_stats).into()
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
}

#[pymethods]
impl PyAutonomicState {
    fn __repr__(&self) -> String {
        format!(
            "AutonomicState(t={:.2}s, dur={:.1}s, activation={:?}, regulation={:?})",
            self.timestamp, self.duration_sec, self.activation_score, self.regulation_score
        )
    }
}

impl From<&lamina::autonomic::AutonomicState> for PyAutonomicState {
    fn from(s: &lamina::autonomic::AutonomicState) -> Self {
        Self {
            timestamp: s.timestamp,
            duration_sec: s.duration_sec,
            activation_score: s.activation_score,
            regulation_score: s.regulation_score,
        }
    }
}

#[pyclass]
pub struct PyAutonomicEstimator {
    pub inner: lamina::autonomic::AutonomicEstimator,
}

#[pymethods]
impl PyAutonomicEstimator {
    #[new]
    #[pyo3(signature = (config=None))]
    pub fn new(config: Option<&PyAutonomicEstimatorConfig>) -> Self {
        let rust_cfg = config.map(|c| c.inner.clone()).unwrap_or_default();
        Self {
            inner: lamina::autonomic::AutonomicEstimator::new(rust_cfg),
        }
    }

    pub fn estimate(
        &self,
        feature_vector: &crate::features::PyMultimodalFeatureVector,
        baseline: &PyAutonomicBaseline,
    ) -> PyResult<PyAutonomicState> {
        let state = self
            .inner
            .estimate(&feature_vector.inner, &baseline.inner)
            .map_err(map_signal_error)?;
        Ok((&state).into())
    }

    pub fn estimate_series(
        &self,
        feature_series: Vec<crate::features::PyMultimodalFeatureVector>,
        baseline: &PyAutonomicBaseline,
    ) -> PyResult<Vec<PyAutonomicState>> {
        let rust_series: Vec<lamina::features::MultimodalFeatureVector> =
            feature_series.into_iter().map(|f| f.inner).collect();
        let series = self
            .inner
            .estimate_series(&rust_series, &baseline.inner)
            .map_err(map_signal_error)?;
        Ok(series.states.iter().map(|s| s.into()).collect())
    }
}
