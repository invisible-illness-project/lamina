use crate::error::map_signal_error;
use pyo3::prelude::*;

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
pub struct PyModalityQuality {
    #[pyo3(get)]
    pub score: f64,
    #[pyo3(get)]
    pub valid: bool,
}

#[pymethods]
impl PyModalityQuality {
    #[new]
    pub fn new(score: f64, valid: bool) -> Self {
        Self { score, valid }
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
    let rust_cycles: Vec<lamina::rsp::RespirationCycle> = rsp_cycles
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
#[pyo3(signature = (ecg_quality=None, ppg_quality=None, eda_quality=None, rsp_quality=None))]
pub fn multimodal_quality<'py>(
    py: Python<'py>,
    ecg_quality: Option<PyModalityQuality>,
    ppg_quality: Option<PyModalityQuality>,
    eda_quality: Option<PyModalityQuality>,
    rsp_quality: Option<PyModalityQuality>,
) -> PyResult<PyMultimodalQuality> {
    let convert_mod =
        |m: Option<PyModalityQuality>| -> Option<lamina::multimodal::ModalityQuality> {
            m.map(|q| lamina::multimodal::ModalityQuality {
                score: q.score,
                valid: q.valid,
                issues: vec![],
            })
        };

    let rust_ecg = convert_mod(ecg_quality);
    let rust_ppg = convert_mod(ppg_quality);
    let rust_eda = convert_mod(eda_quality);
    let rust_rsp = convert_mod(rsp_quality);

    let res = py
        .detach(|| lamina::multimodal::multimodal_quality(rust_ecg, rust_ppg, rust_eda, rust_rsp))
        .map_err(map_signal_error)?;

    let map_mod = |m: Option<lamina::multimodal::ModalityQuality>| -> Option<PyModalityQuality> {
        m.map(|q| PyModalityQuality {
            score: q.score,
            valid: q.valid,
        })
    };

    Ok(PyMultimodalQuality {
        ecg_quality: map_mod(res.ecg_quality),
        ppg_quality: map_mod(res.ppg_quality),
        eda_quality: map_mod(res.eda_quality),
        rsp_quality: map_mod(res.rsp_quality),
        overall_quality: res.overall_quality,
    })
}
