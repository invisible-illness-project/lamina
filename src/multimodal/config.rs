/// Configuration for Respiratory Sinus Arrhythmia (RSA) estimation.
#[derive(Debug, Clone, PartialEq)]
pub struct RsaConfig {
    /// Minimum required valid beats within respiratory cycles to compute RSA.
    pub min_valid_beats: Option<usize>,
}

impl Default for RsaConfig {
    fn default() -> Self {
        Self {
            min_valid_beats: Some(3),
        }
    }
}

/// Configuration for ECG-to-PPG pulse delay timing.
#[derive(Debug, Clone, PartialEq)]
pub struct PulseTimingConfig {
    /// Minimum allowable pulse delay in seconds (default: 0.10 s = 100 ms).
    pub min_delay_sec: Option<f64>,
    /// Maximum allowable pulse delay in seconds (default: 0.60 s = 600 ms).
    pub max_delay_sec: Option<f64>,
}

impl Default for PulseTimingConfig {
    fn default() -> Self {
        Self {
            min_delay_sec: Some(0.10),
            max_delay_sec: Some(0.60),
        }
    }
}

/// Consolidated configuration for multimodal physiological processing.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MultimodalConfig {
    /// RSA configuration settings
    pub rsa: RsaConfig,
    /// ECG-PPG pulse timing configuration settings
    pub pulse_timing: PulseTimingConfig,
}
