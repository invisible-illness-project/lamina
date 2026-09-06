use crate::api::error::SignalError;
use lamina::ecg::clean::ecg_clean;
use lamina::ecg::peaks::{
    ecg_findpeaks as core_ecg_findpeaks, ecg_findpeaks_config as core_ecg_findpeaks_config,
    ecg_findpeaks_mask as core_ecg_findpeaks_mask,
    EcgPeakDetectionConfig as CoreEcgPeakDetectionConfig,
};
use ndarray::Array1;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct EcgPeakDetectionConfig {
    pub lowcut: Option<f64>,
    pub highcut: Option<f64>,
    pub filter_order: Option<usize>,
    pub integration_window_sec: Option<f64>,
    pub refractory_period_sec: Option<f64>,
    pub searchback: Option<bool>,
    pub threshold_multiplier: Option<f64>,
}

impl From<EcgPeakDetectionConfig> for CoreEcgPeakDetectionConfig {
    fn from(c: EcgPeakDetectionConfig) -> Self {
        Self {
            lowcut: c.lowcut,
            highcut: c.highcut,
            filter_order: c.filter_order,
            integration_window_sec: c.integration_window_sec,
            refractory_period_sec: c.refractory_period_sec,
            searchback: c.searchback,
            threshold_multiplier: c.threshold_multiplier,
        }
    }
}

pub fn process_ecg_clean(
    signal: Vec<f64>,
    sampling_rate: f64,
    method: String,
) -> Result<Vec<f64>, SignalError> {
    let nd = Array1::from_vec(signal);
    let cleaned = ecg_clean(&nd, sampling_rate, &method)?;
    Ok(cleaned.into_raw_vec_and_offset().0)
}

pub fn process_ecg_findpeaks(
    signal: Vec<f64>,
    sampling_rate: f64,
    config: Option<EcgPeakDetectionConfig>,
) -> Result<Vec<usize>, SignalError> {
    let nd = Array1::from_vec(signal);
    if let Some(cfg) = config {
        let core_cfg = CoreEcgPeakDetectionConfig::from(cfg);
        let indices = core_ecg_findpeaks_config(&nd, sampling_rate, &core_cfg)?;
        Ok(indices)
    } else {
        let mask = core_ecg_findpeaks(&nd, sampling_rate)?;
        let indices = mask
            .iter()
            .enumerate()
            .filter_map(|(idx, &is_peak)| if is_peak { Some(idx) } else { None })
            .collect();
        Ok(indices)
    }
}

pub fn process_ecg_findpeaks_mask(
    signal: Vec<f64>,
    sampling_rate: f64,
) -> Result<Vec<u8>, SignalError> {
    let nd = Array1::from_vec(signal);
    let cfg = CoreEcgPeakDetectionConfig::default();
    let mask = core_ecg_findpeaks_mask(&nd, sampling_rate, &cfg)?;
    let byte_mask: Vec<u8> = mask.iter().map(|&b| if b { 1 } else { 0 }).collect();
    Ok(byte_mask)
}
