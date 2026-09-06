use crate::api::error::SignalError;
use lamina::eda::clean::eda_clean;
use lamina::eda::peaks::{
    eda_findpeaks as core_eda_findpeaks,
    eda_findpeaks_config as core_eda_findpeaks_config,
    eda_findpeaks_events as core_eda_findpeaks_events,
    eda_findpeaks_mask as core_eda_findpeaks_mask,
    EdaPeakDetectionConfig as CoreEdaPeakDetectionConfig,
    ScrEvent as CoreScrEvent,
};
use lamina::eda::phasic::{
    eda_decompose as core_eda_decompose,
    eda_phasic as core_eda_phasic,
    EdaDecompositionConfig as CoreEdaDecompositionConfig,
};
use ndarray::Array1;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct EdaDecompositionConfig {
    pub tonic_cutoff_hz: Option<f64>,
    pub filter_order: Option<usize>,
}

impl From<EdaDecompositionConfig> for CoreEdaDecompositionConfig {
    fn from(c: EdaDecompositionConfig) -> Self {
        Self {
            tonic_cutoff_hz: c.tonic_cutoff_hz,
            filter_order: c.filter_order,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EdaComponentSignals {
    pub tonic: Vec<f64>,
    pub phasic: Vec<f64>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct EdaPeakDetectionConfig {
    pub min_amplitude: Option<f64>,
    pub min_prominence: Option<f64>,
    pub min_distance_sec: Option<f64>,
    pub min_rise_time_sec: Option<f64>,
    pub max_rise_time_sec: Option<f64>,
}

impl From<EdaPeakDetectionConfig> for CoreEdaPeakDetectionConfig {
    fn from(c: EdaPeakDetectionConfig) -> Self {
        Self {
            min_amplitude: c.min_amplitude,
            min_prominence: c.min_prominence,
            min_distance_sec: c.min_distance_sec,
            min_rise_time_sec: c.min_rise_time_sec,
            max_rise_time_sec: c.max_rise_time_sec,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EdaPeakEvent {
    pub onset_index: usize,
    pub peak_index: usize,
    pub amplitude: f64,
    pub rise_time_sec: f64,
}

impl From<CoreScrEvent> for EdaPeakEvent {
    fn from(e: CoreScrEvent) -> Self {
        Self {
            onset_index: e.onset_index,
            peak_index: e.peak_index,
            amplitude: e.amplitude,
            rise_time_sec: e.rise_time_sec,
        }
    }
}

pub fn process_eda_clean(signal: Vec<f64>, sampling_rate: f64) -> Result<Vec<f64>, SignalError> {
    let nd = Array1::from_vec(signal);
    let cleaned = eda_clean(&nd, sampling_rate)?;
    Ok(cleaned.into_raw_vec_and_offset().0)
}

pub fn process_eda_phasic(signal: Vec<f64>, sampling_rate: f64) -> Result<Vec<f64>, SignalError> {
    let nd = Array1::from_vec(signal);
    let phasic = core_eda_phasic(&nd, sampling_rate)?;
    Ok(phasic.into_raw_vec_and_offset().0)
}

pub fn process_eda_decompose(
    signal: Vec<f64>,
    sampling_rate: f64,
    config: Option<EdaDecompositionConfig>,
) -> Result<EdaComponentSignals, SignalError> {
    let nd = Array1::from_vec(signal);
    let cfg = config.map(CoreEdaDecompositionConfig::from).unwrap_or_default();
    let comp = core_eda_decompose(&nd, sampling_rate, &cfg)?;
    Ok(EdaComponentSignals {
        tonic: comp.tonic.into_raw_vec_and_offset().0,
        phasic: comp.phasic.into_raw_vec_and_offset().0,
    })
}

pub fn process_eda_findpeaks(
    phasic_signal: Vec<f64>,
    sampling_rate: f64,
    config: Option<EdaPeakDetectionConfig>,
) -> Result<Vec<usize>, SignalError> {
    let nd = Array1::from_vec(phasic_signal);
    if let Some(cfg) = config {
        let core_cfg = CoreEdaPeakDetectionConfig::from(cfg);
        let indices = core_eda_findpeaks_config(&nd, sampling_rate, &core_cfg)?;
        Ok(indices)
    } else {
        let mask = core_eda_findpeaks(&nd)?;
        let indices = mask
            .iter()
            .enumerate()
            .filter_map(|(idx, &is_peak)| if is_peak { Some(idx) } else { None })
            .collect();
        Ok(indices)
    }
}

pub fn process_eda_findpeaks_events(
    phasic_signal: Vec<f64>,
    sampling_rate: f64,
    config: Option<EdaPeakDetectionConfig>,
) -> Result<Vec<EdaPeakEvent>, SignalError> {
    let nd = Array1::from_vec(phasic_signal);
    let cfg = config.map(CoreEdaPeakDetectionConfig::from).unwrap_or_default();
    let events = core_eda_findpeaks_events(&nd, sampling_rate, &cfg)?;
    Ok(events.into_iter().map(EdaPeakEvent::from).collect())
}

pub fn process_eda_findpeaks_mask(
    phasic_signal: Vec<f64>,
    sampling_rate: f64,
) -> Result<Vec<u8>, SignalError> {
    let nd = Array1::from_vec(phasic_signal);
    let cfg = CoreEdaPeakDetectionConfig::default();
    let mask = core_eda_findpeaks_mask(&nd, sampling_rate, &cfg)?;
    let byte_mask: Vec<u8> = mask.iter().map(|&b| if b { 1 } else { 0 }).collect();
    Ok(byte_mask)
}
