use crate::error::{Result, SignalError};

/// Normalization method for physiological baseline scaling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NormalizationMethod {
    /// Standard z-score normalization ($z = \frac{x - \mu}{\sigma}$)
    #[default]
    ZScore,
    /// Robust median / Median Absolute Deviation (MAD) normalization ($z = \frac{x - \text{median}}{c \cdot \text{MAD}}$)
    RobustMedianMad,
}

/// Direction of feature change relative to physiological state evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FeatureDirection {
    /// Increasing feature magnitude increases evidence index
    #[default]
    Positive,
    /// Increasing feature magnitude decreases evidence index
    Negative,
}

/// Configuration for baseline normalization and bounded scaling.
#[derive(Debug, Clone, PartialEq)]
pub struct NormalizationConfig {
    /// Active normalization method (ZScore or RobustMedianMad)
    pub method: NormalizationMethod,
    /// Minimum required baseline feature window samples
    pub min_baseline_samples: usize,
    /// Hyperbolic tangent scaling parameter $s > 0$ for bounded transform $\tanh(z / s)$
    pub bounded_scale: f64,
    /// Numerical scale floor threshold to detect unusable baseline scale ($\sigma \le \text{min\_scale}$)
    pub min_scale: f64,
    /// Consistency constant multiplier for MAD (default $c = 1.4826$ for normal equivalence)
    pub mad_multiplier: f64,
}

impl Default for NormalizationConfig {
    fn default() -> Self {
        Self {
            method: NormalizationMethod::ZScore,
            min_baseline_samples: 3,
            bounded_scale: 2.0,
            min_scale: 1e-6,
            mad_multiplier: 1.4826,
        }
    }
}

impl NormalizationConfig {
    /// Validate normalization configuration.
    pub fn validate(&self) -> Result<()> {
        if self.min_baseline_samples == 0 {
            return Err(SignalError::InvalidWindowSize(0));
        }
        if !self.bounded_scale.is_finite() || self.bounded_scale <= 0.0 {
            return Err(SignalError::NonFiniteInput);
        }
        if !self.min_scale.is_finite() || self.min_scale <= 0.0 {
            return Err(SignalError::NonFiniteInput);
        }
        if !self.mad_multiplier.is_finite() || self.mad_multiplier <= 0.0 {
            return Err(SignalError::NonFiniteInput);
        }
        Ok(())
    }
}

/// Configuration weights for Physiological Activation Evidence Index.
#[derive(Debug, Clone, PartialEq)]
pub struct ActivationWeights {
    /// Weight for normalized Heart Rate
    pub hr_weight: f64,
    /// Weight for normalized EDA Phasic SCL
    pub eda_phasic_weight: f64,
    /// Weight for normalized SCR Event Rate
    pub scr_rate_weight: f64,
    /// Weight for normalized Respiratory Rate
    pub rsp_rate_weight: f64,
}

impl Default for ActivationWeights {
    fn default() -> Self {
        Self {
            hr_weight: 1.0,
            eda_phasic_weight: 1.0,
            scr_rate_weight: 1.0,
            rsp_rate_weight: 0.5,
        }
    }
}

/// Configuration weights for Cardiorespiratory Regulation & Coupling Evidence Index.
#[derive(Debug, Clone, PartialEq)]
pub struct RegulationWeights {
    /// Weight for normalized Cardiac Variability evidence (SDNN / RMSSD)
    pub cardiac_variability_weight: f64,
    /// Weight for normalized RespHRV / RSA coupling amplitude
    pub resphr_coupling_weight: f64,
    /// Weight for normalized Cardiorespiratory Phase Concentration
    pub phase_coupling_weight: f64,
}

impl Default for RegulationWeights {
    fn default() -> Self {
        Self {
            cardiac_variability_weight: 1.0,
            resphr_coupling_weight: 1.0,
            phase_coupling_weight: 1.0,
        }
    }
}

/// Configuration parameters for engineered cardiac recovery evidence index.
#[derive(Debug, Clone, PartialEq)]
pub struct RecoveryConfig {
    /// Weight for normalized cardiac variability index
    pub variability_weight: f64,
    /// Weight for normalized heart rate index
    pub heart_rate_weight: f64,
}

impl Default for RecoveryConfig {
    fn default() -> Self {
        Self {
            variability_weight: 1.0,
            heart_rate_weight: 1.0,
        }
    }
}

impl RecoveryConfig {
    /// Validate recovery evidence configuration parameters.
    pub fn validate(&self) -> Result<()> {
        if !self.variability_weight.is_finite() || self.variability_weight < 0.0 {
            return Err(SignalError::NonFiniteInput);
        }
        if !self.heart_rate_weight.is_finite() || self.heart_rate_weight < 0.0 {
            return Err(SignalError::NonFiniteInput);
        }
        if self.variability_weight + self.heart_rate_weight <= 0.0 {
            return Err(SignalError::NonFiniteInput);
        }
        Ok(())
    }
}

/// Configurable weight and threshold parameters for state evidence confidence.
#[derive(Debug, Clone, PartialEq)]
pub struct ConfidenceWeights {
    /// Minimum required beat count for full cardiac confidence scaling
    pub min_beats: usize,
    /// Minimum required breath cycle count for full respiration confidence scaling
    pub min_cycles: usize,
    /// Penalty scaling factor for failed quality flags ($0.0 < \text{factor} \le 1.0$)
    pub quality_factor: f64,
    /// Weight for cardiac confidence in overall confidence mean
    pub cardiac_weight: f64,
    /// Weight for electrodermal confidence in overall confidence mean
    pub eda_weight: f64,
    /// Weight for respiration confidence in overall confidence mean
    pub rsp_weight: f64,
    /// Weight for coupling confidence in overall confidence mean
    pub coupling_weight: f64,
}

impl Default for ConfidenceWeights {
    fn default() -> Self {
        Self {
            min_beats: 10,
            min_cycles: 4,
            quality_factor: 0.5,
            cardiac_weight: 1.0,
            eda_weight: 1.0,
            rsp_weight: 1.0,
            coupling_weight: 1.0,
        }
    }
}

/// Quality gating configuration for feature inclusion.
#[derive(Debug, Clone, PartialEq)]
pub struct QualityConfig {
    /// Minimum required temporal coverage fraction ($0.0 \le \text{coverage} \le 1.0$)
    pub min_coverage: f64,
    /// Require valid cardiac feature flag for cardiac state calculation
    pub require_cardiac_validity: bool,
    /// Require valid direct respiration signal for RespHRV inclusion
    pub require_respiration_for_resphrv: bool,
    /// Configurable confidence weighting and threshold parameters
    pub confidence: ConfidenceWeights,
}

impl Default for QualityConfig {
    fn default() -> Self {
        Self {
            min_coverage: 0.70,
            require_cardiac_validity: true,
            require_respiration_for_resphrv: true,
            confidence: ConfidenceWeights::default(),
        }
    }
}

/// First-order Exponential Moving Average (EMA) temporal smoothing configuration.
#[derive(Debug, Clone, PartialEq)]
pub struct SmoothingConfig {
    /// Smoothing coefficient $\alpha \in (0.0, 1.0]$ ($s_t = \alpha x_t + (1 - \alpha) s_{t-1}$)
    pub alpha: f64,
}

impl Default for SmoothingConfig {
    fn default() -> Self {
        Self { alpha: 0.3 }
    }
}

impl SmoothingConfig {
    /// Validate smoothing configuration.
    pub fn validate(&self) -> Result<()> {
        if !self.alpha.is_finite() || self.alpha <= 0.0 || self.alpha > 1.0 {
            return Err(SignalError::NonFiniteInput);
        }
        Ok(())
    }
}

/// Master configuration for the autonomic state estimator.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AutonomicEstimatorConfig {
    /// Normalization & baseline configuration
    pub normalization: NormalizationConfig,
    /// Activation index weights
    pub activation: ActivationWeights,
    /// Regulation index weights
    pub regulation: RegulationWeights,
    /// Cardiac recovery evidence index weights
    pub recovery: RecoveryConfig,
    /// Quality gating rules
    pub quality: QualityConfig,
    /// Optional temporal smoothing configuration
    pub smoothing: Option<SmoothingConfig>,
}

impl AutonomicEstimatorConfig {
    /// Validate all nested estimator configurations.
    pub fn validate(&self) -> Result<()> {
        self.normalization.validate()?;
        self.recovery.validate()?;
        if let Some(ref s) = self.smoothing {
            s.validate()?;
        }
        Ok(())
    }
}
