use crate::error::map_signal_error;
use lamina::hrv::quality::{
    classify_intervals as rust_classify_intervals, clean_rr_intervals as rust_clean_rr_intervals,
    CorrectionPolicy, IntervalQuality,
};
use numpy::{IntoPyArray, PyArray1, PyReadonlyArray1};
use pyo3::prelude::*;

#[pyfunction]
#[pyo3(signature = (peaks, sampling_rate))]
pub fn peaks_to_intervals<'py>(
    py: Python<'py>,
    peaks: PyReadonlyArray1<'py, bool>,
    sampling_rate: f64,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let array_view = peaks.as_array();
    let arr = array_view.to_owned();

    let out = py
        .detach(|| lamina::hrv::intervals::peaks_to_intervals(&arr, sampling_rate))
        .map_err(map_signal_error)?;

    Ok(out.into_pyarray(py))
}

#[pyfunction]
#[pyo3(signature = (peak_indices, sampling_rate))]
pub fn indices_to_intervals<'py>(
    py: Python<'py>,
    peak_indices: Vec<usize>,
    sampling_rate: f64,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    if peak_indices.len() < 2 {
        return Err(crate::error::InsufficientPeaksError::new_err(
            "Requires at least 2 peak indices",
        ));
    }
    if !sampling_rate.is_finite() || sampling_rate <= 0.0 {
        return Err(crate::error::InvalidSamplingRateError::new_err(format!(
            "Invalid sampling rate: {}",
            sampling_rate
        )));
    }

    let mut intervals = Vec::with_capacity(peak_indices.len() - 1);
    for i in 0..(peak_indices.len() - 1) {
        let diff = peak_indices[i + 1] - peak_indices[i];
        intervals.push((diff as f64 / sampling_rate) * 1000.0);
    }

    let nd_arr = ndarray::Array1::from_vec(intervals);
    Ok(nd_arr.into_pyarray(py))
}

#[pyfunction]
pub fn rmssd<'py>(py: Python<'py>, intervals: PyReadonlyArray1<'py, f64>) -> PyResult<f64> {
    let array_view = intervals.as_array();
    let arr = array_view.to_owned();

    py.detach(|| lamina::hrv::time::hrv_rmssd(&arr))
        .map_err(map_signal_error)
}

#[pyfunction]
pub fn mean_nn<'py>(py: Python<'py>, intervals: PyReadonlyArray1<'py, f64>) -> PyResult<f64> {
    let array_view = intervals.as_array();
    let arr = array_view.to_owned();

    py.detach(|| lamina::hrv::time::hrv_mean_nn(&arr))
        .map_err(map_signal_error)
}

fn interval_quality_str(q: IntervalQuality) -> &'static str {
    match q {
        IntervalQuality::NormalNN => "NormalNN",
        IntervalQuality::EctopicRR => "EctopicRR",
        IntervalQuality::ArtifactRR => "ArtifactRR",
        IntervalQuality::Missing => "Missing",
    }
}

/// Correction policy for handling invalid/ectopic inter-beat intervals.
///
/// Construct via static methods: CorrectionPolicy.none(),
/// .reject_invalid(), .interpolate_linear(), .interpolate_cubic(),
/// .percent_threshold(pct).
#[pyclass]
#[derive(Clone)]
pub struct PyCorrectionPolicy {
    pub inner: CorrectionPolicy,
}

#[pymethods]
impl PyCorrectionPolicy {
    #[staticmethod]
    pub fn none() -> Self {
        Self {
            inner: CorrectionPolicy::None,
        }
    }
    #[staticmethod]
    pub fn reject_invalid() -> Self {
        Self {
            inner: CorrectionPolicy::RejectInvalid,
        }
    }
    #[staticmethod]
    pub fn interpolate_linear() -> Self {
        Self {
            inner: CorrectionPolicy::InterpolateLinear,
        }
    }
    #[staticmethod]
    pub fn interpolate_cubic() -> Self {
        Self {
            inner: CorrectionPolicy::InterpolateCubic,
        }
    }
    #[staticmethod]
    pub fn percent_threshold(pct: f64) -> Self {
        Self {
            inner: CorrectionPolicy::PercentThreshold(pct),
        }
    }
}

/// Classify inter-beat intervals (ms) into quality categories.
///
/// Returns one of "NormalNN", "EctopicRR", "ArtifactRR", "Missing" per interval.
#[pyfunction]
#[pyo3(signature = (intervals, percent_threshold=None))]
pub fn classify_intervals<'py>(
    py: Python<'py>,
    intervals: PyReadonlyArray1<'py, f64>,
    percent_threshold: Option<f64>,
) -> Vec<String> {
    let arr = intervals.as_array().to_owned();
    py.detach(|| {
        rust_classify_intervals(&arr, percent_threshold)
            .into_iter()
            .map(|q| interval_quality_str(q).to_string())
            .collect()
    })
}

/// Clean inter-beat intervals (ms) into a validated N-N series per the correction policy.
#[pyfunction]
pub fn clean_rr_intervals<'py>(
    py: Python<'py>,
    intervals: PyReadonlyArray1<'py, f64>,
    policy: &PyCorrectionPolicy,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let arr = intervals.as_array().to_owned();

    let out = py
        .detach(|| rust_clean_rr_intervals(&arr, &policy.inner))
        .map_err(map_signal_error)?;

    Ok(out.into_pyarray(py))
}
