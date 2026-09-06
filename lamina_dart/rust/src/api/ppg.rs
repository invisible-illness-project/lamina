use crate::api::error::SignalError;
use lamina::ppg::clean::ppg_clean;
use lamina::ppg::peaks::{
    ppg_findpeaks as core_ppg_findpeaks,
    ppg_findpeaks_config as core_ppg_findpeaks_config,
    ppg_findpeaks_mask as core_ppg_findpeaks_mask,
    PpgPeakDetectionConfig as CorePpgPeakDetectionConfig,
};
use ndarray::Array1;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct PpgPeakDetectionConfig {
    pub lowcut: Option<f64>,
    pub highcut: Option<f64>,
    pub filter_order: Option<usize>,
    pub w_peak_sec: Option<f64>,
    pub w_beat_sec: Option<f64>,
    pub alpha: Option<f64>,
    pub refractory_period_sec: Option<f64>,
}

impl From<PpgPeakDetectionConfig> for CorePpgPeakDetectionConfig {
    fn from(c: PpgPeakDetectionConfig) -> Self {
        Self {
            lowcut: c.lowcut,
            highcut: c.highcut,
            filter_order: c.filter_order,
            w_peak_sec: c.w_peak_sec,
            w_beat_sec: c.w_beat_sec,
            alpha: c.alpha,
            refractory_period_sec: c.refractory_period_sec,
        }
    }
}

pub fn process_ppg_clean(signal: Vec<f64>, sampling_rate: f64) -> Result<Vec<f64>, SignalError> {
    let nd = Array1::from_vec(signal);
    let cleaned = ppg_clean(&nd, sampling_rate)?;
    Ok(cleaned.into_raw_vec_and_offset().0)
}

pub fn process_ppg_findpeaks(
    signal: Vec<f64>,
    sampling_rate: f64,
    config: Option<PpgPeakDetectionConfig>,
) -> Result<Vec<usize>, SignalError> {
    let nd = Array1::from_vec(signal);
    if let Some(cfg) = config {
        let core_cfg = CorePpgPeakDetectionConfig::from(cfg);
        let indices = core_ppg_findpeaks_config(&nd, sampling_rate, &core_cfg)?;
        Ok(indices)
    } else {
        let mask = core_ppg_findpeaks(&nd, sampling_rate)?;
        let indices = mask
            .iter()
            .enumerate()
            .filter_map(|(idx, &is_peak)| if is_peak { Some(idx) } else { None })
            .collect();
        Ok(indices)
    }
}

pub fn process_ppg_findpeaks_mask(
    signal: Vec<f64>,
    sampling_rate: f64,
) -> Result<Vec<u8>, SignalError> {
    let nd = Array1::from_vec(signal);
    let cfg = CorePpgPeakDetectionConfig::default();
    let mask = core_ppg_findpeaks_mask(&nd, sampling_rate, &cfg)?;
    let byte_mask: Vec<u8> = mask.iter().map(|&b| if b { 1 } else { 0 }).collect();
    Ok(byte_mask)
}
