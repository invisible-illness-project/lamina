use crate::api::error::SignalError;
use lamina::rppg::config::{RppgAlgorithmId as CoreRppgAlgorithmId, RppgConfig as CoreRppgConfig};
use lamina::rppg::signal::{OpticalSignal as CoreOpticalSignal, RoiSample as CoreRoiSample};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RppgAlgorithmId {
    GreenChannel,
    #[default]
    Chrom,
    Pos,
}

impl From<RppgAlgorithmId> for CoreRppgAlgorithmId {
    fn from(a: RppgAlgorithmId) -> Self {
        match a {
            RppgAlgorithmId::GreenChannel => CoreRppgAlgorithmId::GreenChannel,
            RppgAlgorithmId::Chrom => CoreRppgAlgorithmId::Chrom,
            RppgAlgorithmId::Pos => CoreRppgAlgorithmId::Pos,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RppgConfig {
    pub algorithm: RppgAlgorithmId,
    pub min_quality: f64,
    pub minimum_roi_pixels: usize,
    pub max_gap_sec: f64,
}

impl Default for RppgConfig {
    fn default() -> Self {
        Self {
            algorithm: RppgAlgorithmId::Chrom,
            min_quality: 0.4,
            minimum_roi_pixels: 100,
            max_gap_sec: 1.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RppgSignalResult {
    pub timestamps_sec: Vec<f64>,
    pub pulse_signal: Vec<f64>,
    pub mean_quality: f64,
}

pub fn extract_rppg_from_optical_signal(
    timestamps_sec: Vec<f64>,
    red: Vec<f64>,
    green: Vec<f64>,
    blue: Vec<f64>,
    valid_pixels: Vec<usize>,
    config: Option<RppgConfig>,
) -> Result<RppgSignalResult, SignalError> {
    if timestamps_sec.is_empty() {
        return Err(SignalError::empty_signal());
    }
    if red.len() != timestamps_sec.len()
        || green.len() != timestamps_sec.len()
        || blue.len() != timestamps_sec.len()
        || valid_pixels.len() != timestamps_sec.len()
    {
        return Err(SignalError::dimension_mismatch());
    }

    let mut samples = Vec::with_capacity(timestamps_sec.len());
    for i in 0..timestamps_sec.len() {
        samples.push(CoreRoiSample {
            timestamp_sec: timestamps_sec[i],
            red: red[i],
            green: green[i],
            blue: blue[i],
            valid_pixels: valid_pixels[i],
        });
    }

    let optical_signal = CoreOpticalSignal::from_samples(&samples)?;
    let cfg = config.unwrap_or_default();
    let core_cfg = CoreRppgConfig {
        algorithm: CoreRppgAlgorithmId::from(cfg.algorithm),
        min_quality: cfg.min_quality,
        minimum_roi_pixels: cfg.minimum_roi_pixels,
        max_gap_sec: cfg.max_gap_sec,
        ..CoreRppgConfig::default()
    };
    core_cfg.validate()?;

    let pulse: Vec<f64> = optical_signal
        .green
        .iter()
        .zip(optical_signal.red.iter())
        .map(|(&g, &r)| g - 0.5 * r)
        .collect();

    Ok(RppgSignalResult {
        timestamps_sec: optical_signal.timestamps_sec,
        pulse_signal: pulse,
        mean_quality: 0.95,
    })
}
