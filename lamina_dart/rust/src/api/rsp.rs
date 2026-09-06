use crate::api::error::SignalError;
use lamina::rsp::clean::rsp_clean;
use lamina::rsp::peaks::{
    rsp_cycles_config, rsp_findpeaks as core_rsp_findpeaks,
    rsp_findpeaks_config as core_rsp_findpeaks_config,
    rsp_findpeaks_mask as core_rsp_findpeaks_mask, rsp_rate_config,
    RespirationCycle as CoreRespirationCycle, RspProcessingConfig as CoreRspProcessingConfig,
};
use ndarray::Array1;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct RspProcessingConfig {
    pub lowcut: Option<f64>,
    pub highcut: Option<f64>,
    pub filter_order: Option<usize>,
    pub min_breath_interval_sec: Option<f64>,
    pub max_breath_interval_sec: Option<f64>,
    pub min_amplitude: Option<f64>,
}

impl From<RspProcessingConfig> for CoreRspProcessingConfig {
    fn from(c: RspProcessingConfig) -> Self {
        Self {
            lowcut: c.lowcut,
            highcut: c.highcut,
            filter_order: c.filter_order,
            min_breath_interval_sec: c.min_breath_interval_sec,
            max_breath_interval_sec: c.max_breath_interval_sec,
            min_amplitude: c.min_amplitude,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RespirationCycle {
    pub inspiration_index: usize,
    pub expiration_index: usize,
    pub next_inspiration_index: usize,
    pub duration_sec: f64,
    pub respiratory_rate_bpm: f64,
    pub amplitude: f64,
}

impl From<CoreRespirationCycle> for RespirationCycle {
    fn from(c: CoreRespirationCycle) -> Self {
        Self {
            inspiration_index: c.inspiration_index,
            expiration_index: c.expiration_index,
            next_inspiration_index: c.next_inspiration_index,
            duration_sec: c.duration_sec,
            respiratory_rate_bpm: c.respiratory_rate_bpm,
            amplitude: c.amplitude,
        }
    }
}

pub fn process_rsp_clean(signal: Vec<f64>, sampling_rate: f64) -> Result<Vec<f64>, SignalError> {
    let nd = Array1::from_vec(signal);
    let cleaned = rsp_clean(&nd, sampling_rate)?;
    Ok(cleaned.into_raw_vec_and_offset().0)
}

pub fn process_rsp_findpeaks(
    cleaned_signal: Vec<f64>,
    sampling_rate: f64,
    config: Option<RspProcessingConfig>,
) -> Result<Vec<usize>, SignalError> {
    let nd = Array1::from_vec(cleaned_signal);
    if let Some(cfg) = config {
        let core_cfg = CoreRspProcessingConfig::from(cfg);
        let indices = core_rsp_findpeaks_config(&nd, sampling_rate, &core_cfg)?;
        Ok(indices)
    } else {
        let mask = core_rsp_findpeaks(&nd)?;
        let indices = mask
            .iter()
            .enumerate()
            .filter_map(|(idx, &is_peak)| if is_peak { Some(idx) } else { None })
            .collect();
        Ok(indices)
    }
}

pub fn process_rsp_findpeaks_mask(
    cleaned_signal: Vec<f64>,
    sampling_rate: f64,
    config: Option<RspProcessingConfig>,
) -> Result<Vec<u8>, SignalError> {
    let nd = Array1::from_vec(cleaned_signal);
    let cfg = config.map(CoreRspProcessingConfig::from).unwrap_or_default();
    let mask = core_rsp_findpeaks_mask(&nd, sampling_rate, &cfg)?;
    let byte_mask: Vec<u8> = mask.iter().map(|&b| if b { 1 } else { 0 }).collect();
    Ok(byte_mask)
}

pub fn process_rsp_cycles(
    cleaned_signal: Vec<f64>,
    sampling_rate: f64,
    config: Option<RspProcessingConfig>,
) -> Result<Vec<RespirationCycle>, SignalError> {
    let nd = Array1::from_vec(cleaned_signal);
    let cfg = config.map(CoreRspProcessingConfig::from).unwrap_or_default();
    let cycles = rsp_cycles_config(&nd, sampling_rate, &cfg)?;
    Ok(cycles.into_iter().map(RespirationCycle::from).collect())
}

pub fn process_rsp_rate(
    cleaned_signal: Vec<f64>,
    sampling_rate: f64,
    config: Option<RspProcessingConfig>,
) -> Result<Vec<f64>, SignalError> {
    let nd = Array1::from_vec(cleaned_signal);
    let cfg = config.map(CoreRspProcessingConfig::from).unwrap_or_default();
    let rate = rsp_rate_config(&nd, sampling_rate, &cfg)?;
    Ok(rate.into_raw_vec_and_offset().0)
}
