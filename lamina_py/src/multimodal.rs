use crate::error::map_signal_error;
use pyo3::prelude::*;

#[pyclass]
#[derive(Clone)]
pub struct PyRsaConfig {
    #[pyo3(get, set)]
    pub min_valid_beats: Option<usize>,
}

#[pymethods]
impl PyRsaConfig {
    #[new]
    #[pyo3(signature = (min_valid_beats=None))]
    pub fn new(min_valid_beats: Option<usize>) -> Self {
        Self { min_valid_beats }
    }
}

#[pyclass]
#[derive(Clone)]
pub struct PyPulseTimingConfig {
    #[pyo3(get, set)]
    pub min_delay_sec: Option<f64>,
    #[pyo3(get, set)]
    pub max_delay_sec: Option<f64>,
}

#[pymethods]
impl PyPulseTimingConfig {
    #[new]
    #[pyo3(signature = (min_delay_sec=None, max_delay_sec=None))]
    pub fn new(min_delay_sec: Option<f64>, max_delay_sec: Option<f64>) -> Self {
        Self {
            min_delay_sec,
            max_delay_sec,
        }
    }
}

#[pyclass]
#[derive(Clone)]
pub struct PyRsaResult {
    #[pyo3(get)]
    pub amplitude_bpm: f64,
    #[pyo3(get)]
    pub amplitude_rr_sec: f64,
    #[pyo3(get)]
    pub valid_beats: usize,
    #[pyo3(get)]
    pub valid_cycles: usize,
}

#[pyclass]
#[derive(Clone)]
pub struct PyPhaseCouplingResult {
    #[pyo3(get)]
    pub concentration: f64,
    #[pyo3(get)]
    pub mean_phase: f64,
    #[pyo3(get)]
    pub sample_count: usize,
}

#[pyclass]
#[derive(Clone)]
pub struct PyPulseTimingResult {
    #[pyo3(get)]
    pub ecg_peak_index: usize,
    #[pyo3(get)]
    pub ppg_peak_index: usize,
    #[pyo3(get)]
    pub ecg_timestamp_sec: f64,
    #[pyo3(get)]
    pub ppg_timestamp_sec: f64,
    #[pyo3(get)]
    pub pulse_delay_sec: f64,
}

#[pyclass]
#[derive(Clone)]
pub struct PyCardiacRespiratoryEvent {
    #[pyo3(get)]
    pub r_peak_index: usize,
    #[pyo3(get)]
    pub timestamp_sec: f64,
    #[pyo3(get)]
    pub respiratory_phase: f64,
    #[pyo3(get)]
    pub rr_interval_sec: Option<f64>,
    #[pyo3(get)]
    pub heart_rate_bpm: Option<f64>,
}

#[pyclass]
#[derive(Clone)]
pub struct PyScrCardiorespiratoryAssociation {
    #[pyo3(get)]
    pub scr_peak_index: usize,
    #[pyo3(get)]
    pub scr_peak_time_sec: f64,
    #[pyo3(get)]
    pub scr_amplitude: f64,
    #[pyo3(get)]
    pub respiratory_phase_rad: Option<f64>,
    #[pyo3(get)]
    pub nearest_r_peak_time_sec: Option<f64>,
    #[pyo3(get)]
    pub cardiac_delay_sec: Option<f64>,
}

fn quality_issue_str(q: &lamina::multimodal::QualityIssue) -> &'static str {
    use lamina::multimodal::QualityIssue;
    match q {
        QualityIssue::EmptySignal => "EmptySignal",
        QualityIssue::InsufficientEvents => "InsufficientEvents",
        QualityIssue::UnplausibleHeartRate => "UnplausibleHeartRate",
        QualityIssue::UnplausibleRespirationRate => "UnplausibleRespirationRate",
        QualityIssue::ExtremeArtifact => "ExtremeArtifact",
        QualityIssue::NonFiniteValues => "NonFiniteValues",
    }
}

#[pyclass]
#[derive(Clone)]
pub struct PyModalityQuality {
    #[pyo3(get)]
    pub score: f64,
    #[pyo3(get)]
    pub valid: bool,
    #[pyo3(get)]
    pub issues: Vec<String>,
}

#[pymethods]
impl PyModalityQuality {
    #[new]
    #[pyo3(signature = (score, valid, issues=None))]
    pub fn new(score: f64, valid: bool, issues: Option<Vec<String>>) -> Self {
        Self {
            score,
            valid,
            issues: issues.unwrap_or_default(),
        }
    }
}

impl From<&lamina::multimodal::ModalityQuality> for PyModalityQuality {
    fn from(q: &lamina::multimodal::ModalityQuality) -> Self {
        Self {
            score: q.score,
            valid: q.valid,
            issues: q.issues.iter().map(quality_issue_str).collect(),
        }
    }
}

#[pyclass]
#[derive(Clone)]
pub struct PyMultimodalQuality {
    #[pyo3(get)]
    pub ecg_quality: Option<PyModalityQuality>,
    #[pyo3(get)]
    pub ppg_quality: Option<PyModalityQuality>,
    #[pyo3(get)]
    pub eda_quality: Option<PyModalityQuality>,
    #[pyo3(get)]
    pub rsp_quality: Option<PyModalityQuality>,
    #[pyo3(get)]
    pub overall_quality: f64,
}

fn convert_cycles(cycles: Vec<crate::rsp::PyRespirationCycle>) -> Vec<lamina::rsp::RespirationCycle> {
    cycles
        .into_iter()
        .map(|c| lamina::rsp::RespirationCycle {
            inspiration_index: c.inspiration_index,
            expiration_index: c.expiration_index,
            next_inspiration_index: c.next_inspiration_index,
            duration_sec: c.duration_sec,
            respiratory_rate_bpm: c.respiratory_rate_bpm,
            amplitude: c.amplitude,
        })
        .collect()
}

#[pyfunction]
#[pyo3(signature = (r_peaks, ecg_sampling_rate, ecg_offset_sec, rsp_cycles, rsp_sampling_rate, rsp_offset_sec))]
pub fn rsa<'py>(
    py: Python<'py>,
    r_peaks: Vec<usize>,
    ecg_sampling_rate: f64,
    ecg_offset_sec: f64,
    rsp_cycles: Vec<crate::rsp::PyRespirationCycle>,
    rsp_sampling_rate: f64,
    rsp_offset_sec: f64,
) -> PyResult<PyRsaResult> {
    let rust_cycles = convert_cycles(rsp_cycles);

    let res = py
        .detach(|| {
            lamina::multimodal::rsa(
                &r_peaks,
                ecg_sampling_rate,
                ecg_offset_sec,
                &rust_cycles,
                rsp_sampling_rate,
                rsp_offset_sec,
            )
        })
        .map_err(map_signal_error)?;

    Ok(PyRsaResult {
        amplitude_bpm: res.amplitude_bpm,
        amplitude_rr_sec: res.amplitude_rr_sec,
        valid_beats: res.valid_beats,
        valid_cycles: res.valid_cycles,
    })
}

#[pyfunction]
#[pyo3(signature = (r_peaks, ecg_sampling_rate, ecg_offset_sec, rsp_cycles, rsp_sampling_rate, rsp_offset_sec, config=None))]
pub fn rsa_config<'py>(
    py: Python<'py>,
    r_peaks: Vec<usize>,
    ecg_sampling_rate: f64,
    ecg_offset_sec: f64,
    rsp_cycles: Vec<crate::rsp::PyRespirationCycle>,
    rsp_sampling_rate: f64,
    rsp_offset_sec: f64,
    config: Option<&PyRsaConfig>,
) -> PyResult<PyRsaResult> {
    let rust_cycles = convert_cycles(rsp_cycles);
    let rust_cfg = lamina::multimodal::RsaConfig {
        min_valid_beats: config.and_then(|c| c.min_valid_beats),
    };

    let res = py
        .detach(|| {
            lamina::multimodal::rsa_config(
                &r_peaks,
                ecg_sampling_rate,
                ecg_offset_sec,
                &rust_cycles,
                rsp_sampling_rate,
                rsp_offset_sec,
                &rust_cfg,
            )
        })
        .map_err(map_signal_error)?;

    Ok(PyRsaResult {
        amplitude_bpm: res.amplitude_bpm,
        amplitude_rr_sec: res.amplitude_rr_sec,
        valid_beats: res.valid_beats,
        valid_cycles: res.valid_cycles,
    })
}

#[pyfunction]
#[pyo3(signature = (r_peaks, ecg_sampling_rate, ecg_offset_sec, rsp_cycles, rsp_sampling_rate, rsp_offset_sec))]
pub fn cardiac_respiratory_phase<'py>(
    py: Python<'py>,
    r_peaks: Vec<usize>,
    ecg_sampling_rate: f64,
    ecg_offset_sec: f64,
    rsp_cycles: Vec<crate::rsp::PyRespirationCycle>,
    rsp_sampling_rate: f64,
    rsp_offset_sec: f64,
) -> PyResult<Vec<PyCardiacRespiratoryEvent>> {
    let rust_cycles = convert_cycles(rsp_cycles);

    let events = py
        .detach(|| {
            lamina::multimodal::cardiac_respiratory_phase(
                &r_peaks,
                ecg_sampling_rate,
                ecg_offset_sec,
                &rust_cycles,
                rsp_sampling_rate,
                rsp_offset_sec,
            )
        })
        .map_err(map_signal_error)?;

    Ok(events
        .iter()
        .map(|e| PyCardiacRespiratoryEvent {
            r_peak_index: e.r_peak_index,
            timestamp_sec: e.timestamp_sec,
            respiratory_phase: e.respiratory_phase,
            rr_interval_sec: e.rr_interval_sec,
            heart_rate_bpm: e.heart_rate_bpm,
        })
        .collect())
}

#[pyfunction]
pub fn cardiorespiratory_phase_coupling<'py>(
    py: Python<'py>,
    phases: Vec<f64>,
) -> PyResult<PyPhaseCouplingResult> {
    let res = py
        .detach(|| lamina::multimodal::cardiorespiratory_phase_coupling(&phases))
        .map_err(map_signal_error)?;

    Ok(PyPhaseCouplingResult {
        concentration: res.concentration,
        mean_phase: res.mean_phase,
        sample_count: res.sample_count,
    })
}

#[pyfunction]
#[pyo3(signature = (ecg_peaks, ecg_sampling_rate, ecg_offset_sec, ppg_peaks, ppg_sampling_rate, ppg_offset_sec, config=None))]
pub fn ecg_ppg_timing<'py>(
    py: Python<'py>,
    ecg_peaks: Vec<usize>,
    ecg_sampling_rate: f64,
    ecg_offset_sec: f64,
    ppg_peaks: Vec<usize>,
    ppg_sampling_rate: f64,
    ppg_offset_sec: f64,
    config: Option<&PyPulseTimingConfig>,
) -> PyResult<Vec<PyPulseTimingResult>> {
    let matches = py
        .detach(|| {
            if let Some(cfg) = config {
                let rust_cfg = lamina::multimodal::PulseTimingConfig {
                    min_delay_sec: cfg.min_delay_sec,
                    max_delay_sec: cfg.max_delay_sec,
                };
                lamina::multimodal::ecg_ppg_timing_config(
                    &ecg_peaks,
                    ecg_sampling_rate,
                    ecg_offset_sec,
                    &ppg_peaks,
                    ppg_sampling_rate,
                    ppg_offset_sec,
                    &rust_cfg,
                )
            } else {
                lamina::multimodal::ecg_ppg_timing(
                    &ecg_peaks,
                    ecg_sampling_rate,
                    ecg_offset_sec,
                    &ppg_peaks,
                    ppg_sampling_rate,
                    ppg_offset_sec,
                )
            }
        })
        .map_err(map_signal_error)?;

    Ok(matches
        .iter()
        .map(|m| PyPulseTimingResult {
            ecg_peak_index: m.ecg_peak_index,
            ppg_peak_index: m.ppg_peak_index,
            ecg_timestamp_sec: m.ecg_timestamp_sec,
            ppg_timestamp_sec: m.ppg_timestamp_sec,
            pulse_delay_sec: m.ppg_timestamp_sec - m.ecg_timestamp_sec,
        })
        .collect())
}

#[pyfunction]
#[pyo3(signature = (scr_events, eda_sampling_rate, eda_offset_sec, r_peaks, ecg_sampling_rate, ecg_offset_sec, rsp_cycles, rsp_sampling_rate, rsp_offset_sec))]
pub fn eda_cardiorespiratory_association<'py>(
    py: Python<'py>,
    scr_events: Vec<crate::eda::PyScrEvent>,
    eda_sampling_rate: f64,
    eda_offset_sec: f64,
    r_peaks: Vec<usize>,
    ecg_sampling_rate: f64,
    ecg_offset_sec: f64,
    rsp_cycles: Vec<crate::rsp::PyRespirationCycle>,
    rsp_sampling_rate: f64,
    rsp_offset_sec: f64,
) -> PyResult<Vec<PyScrCardiorespiratoryAssociation>> {
    let rust_events: Vec<lamina::eda::ScrEvent> = scr_events
        .into_iter()
        .map(|e| lamina::eda::ScrEvent {
            onset_index: e.onset_index,
            peak_index: e.peak_index,
            amplitude: e.amplitude,
            rise_time_sec: e.rise_time_sec,
        })
        .collect();
    let rust_cycles = convert_cycles(rsp_cycles);

    let assocs = py
        .detach(|| {
            lamina::multimodal::eda_cardiorespiratory_association(
                &rust_events,
                eda_sampling_rate,
                eda_offset_sec,
                &r_peaks,
                ecg_sampling_rate,
                ecg_offset_sec,
                &rust_cycles,
                rsp_sampling_rate,
                rsp_offset_sec,
            )
        })
        .map_err(map_signal_error)?;

    Ok(assocs
        .iter()
        .map(|a| PyScrCardiorespiratoryAssociation {
            scr_peak_index: a.scr_peak_index,
            scr_peak_time_sec: a.scr_peak_time_sec,
            scr_amplitude: a.scr_amplitude,
            respiratory_phase_rad: a.respiratory_phase_rad,
            nearest_r_peak_time_sec: a.nearest_r_peak_time_sec,
            cardiac_delay_sec: a.cardiac_delay_sec,
        })
        .collect())
}

#[pyfunction]
pub fn respiratory_phase_at_time(
    rsp_cycles: Vec<crate::rsp::PyRespirationCycle>,
    timestamp_sec: f64,
    rsp_sampling_rate: f64,
    rsp_offset_sec: f64,
) -> Option<f64> {
    let rust_cycles = convert_cycles(rsp_cycles);
    lamina::multimodal::respiratory_phase_at_time(
        &rust_cycles,
        timestamp_sec,
        rsp_sampling_rate,
        rsp_offset_sec,
    )
}

#[pyfunction]
pub fn evaluate_ecg_quality<'py>(
    py: Python<'py>,
    r_peaks: Vec<usize>,
    sampling_rate: f64,
    signal_duration_sec: f64,
) -> PyModalityQuality {
    let q = py
        .detach(|| lamina::multimodal::evaluate_ecg_quality(&r_peaks, sampling_rate, signal_duration_sec));
    PyModalityQuality::from(&q)
}

#[pyfunction]
pub fn evaluate_rsp_quality<'py>(
    py: Python<'py>,
    cycles_count: usize,
    signal_duration_sec: f64,
) -> PyModalityQuality {
    let q = py.detach(|| {
        lamina::multimodal::evaluate_rsp_quality(cycles_count, signal_duration_sec)
    });
    PyModalityQuality::from(&q)
}

fn str_to_issue(s: &str) -> lamina::multimodal::QualityIssue {
    use lamina::multimodal::QualityIssue;
    match s {
        "EmptySignal" => QualityIssue::EmptySignal,
        "InsufficientEvents" => QualityIssue::InsufficientEvents,
        "UnplausibleHeartRate" => QualityIssue::UnplausibleHeartRate,
        "UnplausibleRespirationRate" => QualityIssue::UnplausibleRespirationRate,
        "ExtremeArtifact" => QualityIssue::ExtremeArtifact,
        _ => QualityIssue::NonFiniteValues,
    }
}

#[pyfunction]
#[pyo3(signature = (ecg_quality=None, ppg_quality=None, eda_quality=None, rsp_quality=None))]
pub fn multimodal_quality<'py>(
    py: Python<'py>,
    ecg_quality: Option<PyModalityQuality>,
    ppg_quality: Option<PyModalityQuality>,
    eda_quality: Option<PyModalityQuality>,
    rsp_quality: Option<PyModalityQuality>,
) -> PyResult<PyMultimodalQuality> {
    let convert_mod = |m: Option<PyModalityQuality>| -> Option<lamina::multimodal::ModalityQuality> {
        m.map(|q| lamina::multimodal::ModalityQuality {
            score: q.score,
            valid: q.valid,
            issues: q.issues.iter().map(|s| str_to_issue(s)).collect(),
        })
    };

    let rust_ecg = convert_mod(ecg_quality);
    let rust_ppg = convert_mod(ppg_quality);
    let rust_eda = convert_mod(eda_quality);
    let rust_rsp = convert_mod(rsp_quality);

    let res = py
        .detach(|| lamina::multimodal::multimodal_quality(rust_ecg, rust_ppg, rust_eda, rust_rsp))
        .map_err(map_signal_error)?;

    Ok(PyMultimodalQuality {
        ecg_quality: res.ecg_quality.as_ref().map(PyModalityQuality::from),
        ppg_quality: res.ppg_quality.as_ref().map(PyModalityQuality::from),
        eda_quality: res.eda_quality.as_ref().map(PyModalityQuality::from),
        rsp_quality: res.rsp_quality.as_ref().map(PyModalityQuality::from),
        overall_quality: res.overall_quality,
    })
}
