/// Configuration for physiological feature window generation.
#[derive(Debug, Clone, PartialEq)]
pub struct WindowConfig {
    /// Window duration in seconds (default: 60.0 s).
    pub window_duration_sec: f64,
    /// Step size between consecutive windows in seconds (default: 30.0 s).
    pub step_sec: f64,
    /// Minimum required recording coverage ratio in $[0.0, 1.0]$ (default: 0.80).
    pub min_coverage: f64,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            window_duration_sec: 60.0,
            step_sec: 30.0,
            min_coverage: 0.80,
        }
    }
}

/// Configuration for multimodal feature extraction.
#[derive(Debug, Clone, PartialEq)]
pub struct FeatureConfig {
    /// Window generation settings
    pub window: WindowConfig,
    /// Minimum cardiac beat count required for valid cardiac features (default: 10).
    pub min_beats: usize,
    /// Minimum respiration cycles required for valid respiration features (default: 3).
    pub min_respiration_cycles: usize,
    /// Minimum SCR events required for valid EDA feature calculations (default: 0).
    pub min_scr_events: usize,
    /// Require valid cardiac modality features for non-empty feature vector (default: false).
    pub require_cardiac: bool,
    /// Require valid respiration modality features for non-empty feature vector (default: false).
    pub require_respiration: bool,
    /// Require valid EDA modality features for non-empty feature vector (default: false).
    pub require_eda: bool,
}

impl Default for FeatureConfig {
    fn default() -> Self {
        Self {
            window: WindowConfig::default(),
            min_beats: 10,
            min_respiration_cycles: 3,
            min_scr_events: 0,
            require_cardiac: false,
            require_respiration: false,
            require_eda: false,
        }
    }
}
