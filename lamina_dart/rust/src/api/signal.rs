use crate::api::error::SignalError;
use lamina::signal::filter::signal_filter;
use lamina::signal::peaks::{signal_findpeaks, signal_findpeaks_config, PeakDetectionConfig as CorePeakDetectionConfig};
use lamina::signal::smooth::signal_smooth_moving_average;
use ndarray::Array1;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct PeakDetectionConfig {
    pub min_height: Option<f64>,
    pub min_distance: Option<usize>,
    pub min_prominence: Option<f64>,
    pub min_width: Option<usize>,
    pub threshold: Option<f64>,
}

impl From<PeakDetectionConfig> for CorePeakDetectionConfig {
    fn from(c: PeakDetectionConfig) -> Self {
        Self {
            min_height: c.min_height,
            min_distance: c.min_distance,
            min_prominence: c.min_prominence,
            min_width: c.min_width,
            threshold: c.threshold,
        }
    }
}

pub fn process_signal_smooth_moving_average(
    signal: Vec<f64>,
    window_size: usize,
) -> Result<Vec<f64>, SignalError> {
    let nd = Array1::from_vec(signal);
    let out = signal_smooth_moving_average(&nd, window_size)?;
    Ok(out.into_raw_vec_and_offset().0)
}

pub fn process_signal_filter(
    signal: Vec<f64>,
    sampling_rate: f64,
    lowcut: Option<f64>,
    highcut: Option<f64>,
    order: usize,
) -> Result<Vec<f64>, SignalError> {
    let nd = Array1::from_vec(signal);
    let out = signal_filter(&nd, sampling_rate, lowcut, highcut, order)?;
    Ok(out.into_raw_vec_and_offset().0)
}

pub fn process_signal_findpeaks(signal: Vec<f64>) -> Result<Vec<usize>, SignalError> {
    let nd = Array1::from_vec(signal);
    let mask = signal_findpeaks(&nd)?;
    let indices: Vec<usize> = mask
        .iter()
        .enumerate()
        .filter_map(|(idx, &is_peak)| if is_peak { Some(idx) } else { None })
        .collect();
    Ok(indices)
}

pub fn process_signal_findpeaks_config(
    signal: Vec<f64>,
    config: PeakDetectionConfig,
) -> Result<Vec<usize>, SignalError> {
    let nd = Array1::from_vec(signal);
    let core_cfg = CorePeakDetectionConfig::from(config);
    let indices = signal_findpeaks_config(&nd, &core_cfg)?;
    Ok(indices)
}
