use pyo3::prelude::*;

mod autonomic;
mod complexity;
mod ecg;
mod eda;
mod error;
mod features;
mod hrv;
mod multimodal;
mod ppg;
mod rppg;
mod rsp;
mod signal;

#[pymodule]
fn _lamina(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;

    // Errors
    error::register_errors(m)?;

    // Signal
    m.add_class::<signal::PyPeakDetectionConfig>()?;
    m.add_function(wrap_pyfunction!(signal::smooth_moving_average, m)?)?;
    m.add_function(wrap_pyfunction!(signal::filter, m)?)?;
    m.add_function(wrap_pyfunction!(signal::filtfilt, m)?)?;
    m.add_function(wrap_pyfunction!(signal::findpeaks, m)?)?;
    m.add_function(wrap_pyfunction!(signal::findpeaks_mask, m)?)?;

    // ECG
    m.add_class::<ecg::PyEcgPeakDetectionConfig>()?;
    m.add_function(wrap_pyfunction!(ecg::ecg_clean, m)?)?;
    m.add_function(wrap_pyfunction!(ecg::ecg_findpeaks, m)?)?;
    m.add_function(wrap_pyfunction!(ecg::ecg_findpeaks_mask, m)?)?;

    // PPG
    m.add_class::<ppg::PyPpgPeakDetectionConfig>()?;
    m.add_function(wrap_pyfunction!(ppg::ppg_clean, m)?)?;
    m.add_function(wrap_pyfunction!(ppg::ppg_findpeaks, m)?)?;
    m.add_function(wrap_pyfunction!(ppg::ppg_findpeaks_mask, m)?)?;

    // EDA
    m.add_class::<eda::PyEdaDecompositionConfig>()?;
    m.add_class::<eda::PyEdaPeakDetectionConfig>()?;
    m.add_class::<eda::PyScrEvent>()?;
    m.add_class::<eda::PyEdaComponents>()?;
    m.add_function(wrap_pyfunction!(eda::eda_clean, m)?)?;
    m.add_function(wrap_pyfunction!(eda::eda_decompose, m)?)?;
    m.add_function(wrap_pyfunction!(eda::eda_phasic, m)?)?;
    m.add_function(wrap_pyfunction!(eda::eda_findpeaks, m)?)?;
    m.add_function(wrap_pyfunction!(eda::eda_findpeaks_events, m)?)?;

    // RSP
    m.add_class::<rsp::PyRspCleaningConfig>()?;
    m.add_class::<rsp::PyRspProcessingConfig>()?;
    m.add_class::<rsp::PyRespirationCycle>()?;
    m.add_function(wrap_pyfunction!(rsp::rsp_clean, m)?)?;
    m.add_function(wrap_pyfunction!(rsp::rsp_findpeaks, m)?)?;
    m.add_function(wrap_pyfunction!(rsp::rsp_cycles, m)?)?;
    m.add_function(wrap_pyfunction!(rsp::rsp_rate, m)?)?;

    // HRV
    m.add_function(wrap_pyfunction!(hrv::peaks_to_intervals, m)?)?;
    m.add_function(wrap_pyfunction!(hrv::indices_to_intervals, m)?)?;
    m.add_function(wrap_pyfunction!(hrv::rmssd, m)?)?;
    m.add_function(wrap_pyfunction!(hrv::mean_nn, m)?)?;

    // Complexity
    m.add_function(wrap_pyfunction!(complexity::sample_entropy, m)?)?;

    // Autonomic
    m.add_class::<autonomic::PyAutonomicEstimatorConfig>()?;
    m.add_class::<autonomic::PyBaselineFeatureStats>()?;
    m.add_class::<autonomic::PyAutonomicBaseline>()?;
    m.add_class::<autonomic::PyAutonomicState>()?;
    m.add_class::<autonomic::PyAutonomicEstimator>()?;

    // rPPG
    m.add_class::<rppg::PyVideoFrame>()?;
    m.add_class::<rppg::PyVideoStream>()?;
    m.add_class::<rppg::PyRoi>()?;
    m.add_class::<rppg::PyRppgConfig>()?;
    m.add_class::<rppg::PyRppgSignal>()?;
    m.add_function(wrap_pyfunction!(rppg::extract_rppg, m)?)?;

    // Features
    m.add_class::<features::PyFeatureConfig>()?;
    m.add_class::<features::PyMultimodalInput>()?;
    m.add_class::<features::PyMultimodalFeatureVector>()?;
    m.add_function(wrap_pyfunction!(features::extract_features, m)?)?;

    // Multimodal
    m.add_class::<multimodal::PyRsaResult>()?;
    m.add_class::<multimodal::PyPhaseCouplingResult>()?;
    m.add_class::<multimodal::PyModalityQuality>()?;
    m.add_class::<multimodal::PyMultimodalQuality>()?;
    m.add_function(wrap_pyfunction!(multimodal::rsa, m)?)?;
    m.add_function(wrap_pyfunction!(multimodal::cardiorespiratory_phase_coupling, m)?)?;
    m.add_function(wrap_pyfunction!(multimodal::multimodal_quality, m)?)?;

    Ok(())
}
