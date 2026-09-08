use crate::error::{Result, SignalError};

/// Identifiers for supported remote photoplethysmography (rPPG) pulse extraction algorithms.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RppgAlgorithmId {
    /// Green-channel intensity baseline method
    GreenChannel,
    /// Chrominance-based method (de Haan & Jeanne, 2013)
    #[default]
    Chrom,
    /// Plane-Orthogonal-to-Skin method (Wang et al., 2017)
    Pos,
}

/// Configuration parameters for temporal windowed rPPG processing.
#[derive(Debug, Clone, PartialEq)]
pub struct RppgWindowConfig {
    /// Processing window duration in seconds (default 3.0 s)
    pub window_sec: f64,
    /// Processing window step / hop size in seconds (default 0.5 s)
    pub step_sec: f64,
    /// Minimum required window duration coverage fraction in $(0.0, 1.0]$ (default 0.8 = 80%)
    pub min_window_fraction: f64,
}

impl Default for RppgWindowConfig {
    fn default() -> Self {
        Self {
            window_sec: 3.0,
            step_sec: 0.5,
            min_window_fraction: 0.8,
        }
    }
}

impl RppgWindowConfig {
    /// Validate temporal window parameters.
    pub fn validate(&self) -> Result<()> {
        if !self.window_sec.is_finite() || self.window_sec <= 0.0 {
            return Err(SignalError::InvalidWindowSize(0));
        }
        if !self.step_sec.is_finite() || self.step_sec <= 0.0 {
            return Err(SignalError::InvalidWindowSize(0));
        }
        if self.step_sec > self.window_sec {
            return Err(SignalError::InvalidWindowSize(0));
        }
        if !self.min_window_fraction.is_finite()
            || self.min_window_fraction <= 0.0
            || self.min_window_fraction > 1.0
        {
            return Err(SignalError::NonFiniteInput);
        }
        Ok(())
    }
}

/// Configuration for optical signal preprocessing and detrending.
#[derive(Debug, Clone, PartialEq)]
pub struct RppgPreprocessingConfig {
    /// Apply channel-wise mean normalization ($C / \mu_C - 1$)
    pub normalize_channels: bool,
    /// Apply temporal trend removal
    pub detrend: bool,
}

impl Default for RppgPreprocessingConfig {
    fn default() -> Self {
        Self {
            normalize_channels: true,
            detrend: true,
        }
    }
}

/// Signal polarity convention for optical surrogates and blood volume pulse (BVP) waveforms.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SignalPolarity {
    /// Normal optical surrogate (pass-through uninverted projection).
    /// Assumes input is already normalized to the expected positive BVP peak expansion convention. No inversion occurs.
    #[default]
    Normal,
    /// Inverted optical surrogate (explicitly negated waveform).
    /// Assumes input uses raw optical absorption convention (where systolic blood expansion drops reflected light intensity).
    /// This is the normative production default for raw optical signal processing.
    Inverted,
    /// Auto-detect polarity using a directional skewness heuristic (`skew < -0.3`).
    ///
    /// # Operational Contract & Limitations
    /// `AutoDetect` is an experimental statistical heuristic fallback. If sample skewness satisfies $\gamma_1 < -0.3$
    /// (indicating strong downward excursion dominance, e.g. raw light intensity absorption drops), `AutoDetect` negates
    /// the signal. If $\gamma_1 \ge -0.3$, or if signal variance is low ($\sigma \le 10^{-6}$), `AutoDetect` leaves the waveform unchanged.
    ///
    /// Statistical skewness alone cannot establish physical sensor optical orientation with certainty. For robust production processing,
    /// explicit [`SignalPolarity::Inverted`] remains the recommended production default.
    AutoDetect,
}

/// Master configuration for the rPPG pulse signal extraction pipeline.
#[derive(Debug, Clone, PartialEq)]
pub struct RppgConfig {
    /// Selected rPPG extraction algorithm
    pub algorithm: RppgAlgorithmId,
    /// Minimum required composite quality threshold in $[0.0, 1.0]$ (default 0.4)
    pub min_quality: f64,
    /// Minimum required valid ROI pixel count (default 100)
    pub minimum_roi_pixels: usize,
    /// Maximum allowed gap in seconds for uniform temporal resampling (default 1.0 s)
    pub max_gap_sec: f64,
    /// Temporal windowing parameters
    pub window: RppgWindowConfig,
    /// Optical signal preprocessing settings
    pub preprocessing: RppgPreprocessingConfig,
    /// Frequency band $[f_{\text{low}}, f_{\text{high}}]$ in Hz for pulse signal filtering (default $[0.75, 2.5]$ Hz for $45\text{--}150$ BPM)
    pub signal_band_hz: (f64, f64),
    /// Pulse-phase polarity convention for downstream BVP conversion
    pub polarity: SignalPolarity,
}

impl Default for RppgConfig {
    fn default() -> Self {
        Self {
            algorithm: RppgAlgorithmId::default(),
            min_quality: 0.4,
            minimum_roi_pixels: 100,
            max_gap_sec: 1.0,
            window: RppgWindowConfig::default(),
            preprocessing: RppgPreprocessingConfig::default(),
            signal_band_hz: (0.75, 2.5),
            polarity: SignalPolarity::Inverted,
        }
    }
}

impl RppgConfig {
    /// Set signal polarity convention.
    pub fn with_polarity(mut self, polarity: SignalPolarity) -> Self {
        self.polarity = polarity;
        self
    }

    /// Validate all rPPG configuration fields.
    pub fn validate(&self) -> Result<()> {
        if !self.min_quality.is_finite() || !(0.0..=1.0).contains(&self.min_quality) {
            return Err(SignalError::NonFiniteInput);
        }
        if self.minimum_roi_pixels == 0 {
            return Err(SignalError::InvalidWindowSize(0));
        }
        if !self.max_gap_sec.is_finite() || self.max_gap_sec <= 0.0 {
            return Err(SignalError::NonFiniteInput);
        }
        self.window.validate()?;
        let (f_low, f_high) = self.signal_band_hz;
        if !f_low.is_finite() || !f_high.is_finite() || f_low <= 0.0 || f_low >= f_high {
            return Err(SignalError::InvalidCutoffFrequency(
                "Invalid rPPG signal frequency band".to_string(),
            ));
        }
        Ok(())
    }
}
