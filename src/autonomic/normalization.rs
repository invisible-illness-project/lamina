use crate::autonomic::config::{FeatureDirection, NormalizationConfig, NormalizationMethod};
use crate::error::{Result, SignalError};
use crate::features::MultimodalFeatureVector;

/// Statistical summary for a single physiological feature computed over baseline windows.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BaselineFeatureStats {
    /// Population mean ($\mu$)
    pub mean: Option<f64>,
    /// Population standard deviation ($\sigma = \sqrt{\frac{1}{N}\sum (x_i - \mu)^2}$)
    pub std: Option<f64>,
    /// Sample median
    pub median: Option<f64>,
    /// Median Absolute Deviation (MAD)
    pub mad: Option<f64>,
    /// Number of valid baseline window samples contributing to statistics
    pub sample_count: usize,
    /// Validity flag indicating whether sample count $\ge \text{min\_baseline\_samples}$
    pub is_valid: bool,
}

impl BaselineFeatureStats {
    /// Compute baseline feature statistics from a slice of observed feature values.
    ///
    /// # Population Standard Deviation Semantics
    /// Calculates the **population standard deviation** ($\sigma = \sqrt{\frac{1}{N}\sum (x_i - \mu)^2}$),
    /// treating baseline feature windows as the empirical reference population.
    pub fn from_samples(samples: &[f64], config: &NormalizationConfig) -> Self {
        let valid_samples: Vec<f64> = samples.iter().copied().filter(|v| v.is_finite()).collect();
        let n = valid_samples.len();

        if n < config.min_baseline_samples {
            return Self {
                sample_count: n,
                is_valid: false,
                ..Self::default()
            };
        }

        // Population Mean & Population Standard Deviation (\sigma)
        let mean = valid_samples.iter().sum::<f64>() / n as f64;
        let variance = valid_samples
            .iter()
            .map(|x| (x - mean).powi(2))
            .sum::<f64>()
            / n as f64;
        let std = variance.sqrt();

        // Median & MAD
        let mut sorted = valid_samples.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let median = if n % 2 == 1 {
            sorted[n / 2]
        } else {
            0.5 * (sorted[n / 2 - 1] + sorted[n / 2])
        };

        let mut abs_devs: Vec<f64> = sorted.iter().map(|x| (x - median).abs()).collect();
        abs_devs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let mad = if n % 2 == 1 {
            abs_devs[n / 2]
        } else {
            0.5 * (abs_devs[n / 2 - 1] + abs_devs[n / 2])
        };

        Self {
            mean: Some(mean),
            std: Some(std),
            median: Some(median),
            mad: Some(mad),
            sample_count: n,
            is_valid: true,
        }
    }
}

/// Baseline physiological model fitted over reference feature vectors.
///
/// # Baseline Eligibility Assumption
/// Baseline fitting assumes that the caller-supplied reference feature vectors represent the intended
/// reference physiological context. Lamina does not independently classify or confirm whether a window is resting.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AutonomicBaseline {
    /// Baseline Heart Rate statistics (BPM)
    pub hr_bpm_stats: BaselineFeatureStats,
    /// Baseline SDNN statistics (ms)
    pub sdnn_ms_stats: BaselineFeatureStats,
    /// Baseline RMSSD statistics (ms)
    pub rmssd_ms_stats: BaselineFeatureStats,
    /// Baseline EDA Tonic Skin Conductance Level statistics ($\mu\text{S}$)
    pub eda_tonic_stats: BaselineFeatureStats,
    /// Baseline EDA Phasic statistics ($\mu\text{S}$)
    pub eda_phasic_stats: BaselineFeatureStats,
    /// Baseline SCR event rate statistics (events/min)
    pub scr_rate_stats: BaselineFeatureStats,
    /// Baseline Respiratory Rate statistics (BPM)
    pub rsp_rate_stats: BaselineFeatureStats,
    /// Baseline Breath Cycle Height Amplitude statistics
    pub rsp_amplitude_stats: BaselineFeatureStats,
    /// Baseline Respiratory Rate Standard Deviation statistics (BPM)
    pub rsp_std_stats: BaselineFeatureStats,
    /// Baseline RespHRV / RSA amplitude statistics (BPM)
    pub rsa_bpm_stats: BaselineFeatureStats,
    /// Baseline Cardiorespiratory Phase Concentration statistics ($R \in [0, 1]$)
    pub phase_coupling_stats: BaselineFeatureStats,
    /// Baseline ECG-PPG Pulse Delay statistics (seconds)
    pub pulse_delay_stats: BaselineFeatureStats,
}

impl AutonomicBaseline {
    /// Fit an [`AutonomicBaseline`] model from a sequence of windowed feature vectors.
    pub fn fit(
        feature_series: &[MultimodalFeatureVector],
        config: &NormalizationConfig,
    ) -> Result<Self> {
        config.validate()?;
        if feature_series.is_empty() {
            return Err(SignalError::InsufficientSamples {
                required: config.min_baseline_samples,
                provided: 0,
            });
        }

        let extract_values = |extractor: fn(&MultimodalFeatureVector) -> Option<f64>| -> Vec<f64> {
            feature_series.iter().filter_map(extractor).collect()
        };

        Ok(Self {
            hr_bpm_stats: BaselineFeatureStats::from_samples(
                &extract_values(|fv| fv.cardiac.mean_hr_bpm),
                config,
            ),
            sdnn_ms_stats: BaselineFeatureStats::from_samples(
                &extract_values(|fv| fv.cardiac.sdnn_ms),
                config,
            ),
            rmssd_ms_stats: BaselineFeatureStats::from_samples(
                &extract_values(|fv| fv.cardiac.rmssd_ms),
                config,
            ),
            eda_tonic_stats: BaselineFeatureStats::from_samples(
                &extract_values(|fv| fv.eda.mean_tonic_us),
                config,
            ),
            eda_phasic_stats: BaselineFeatureStats::from_samples(
                &extract_values(|fv| fv.eda.mean_phasic_us),
                config,
            ),
            scr_rate_stats: BaselineFeatureStats::from_samples(
                &extract_values(|fv| fv.eda.scr_rate_per_min),
                config,
            ),
            rsp_rate_stats: BaselineFeatureStats::from_samples(
                &extract_values(|fv| fv.respiration.mean_rate_bpm),
                config,
            ),
            rsp_amplitude_stats: BaselineFeatureStats::from_samples(
                &extract_values(|fv| fv.respiration.mean_amplitude),
                config,
            ),
            rsp_std_stats: BaselineFeatureStats::from_samples(
                &extract_values(|fv| fv.respiration.rate_std_bpm),
                config,
            ),
            rsa_bpm_stats: BaselineFeatureStats::from_samples(
                &extract_values(|fv| fv.coupling.rsa_amplitude_bpm),
                config,
            ),
            phase_coupling_stats: BaselineFeatureStats::from_samples(
                &extract_values(|fv| fv.coupling.cardiac_respiratory_concentration),
                config,
            ),
            pulse_delay_stats: BaselineFeatureStats::from_samples(
                &extract_values(|fv| fv.coupling.mean_pulse_delay_sec),
                config,
            ),
        })
    }

    /// Normalize an observed feature value against baseline statistics, applying directional orientation and hyperbolic tangent clipping into $[-1.0, 1.0]$.
    ///
    /// # Zero-Variance Policy
    /// If baseline scale ($\sigma$ or $c \cdot \text{MAD}$) $\le \text{min\_scale}$:
    /// - If $|x - \text{location}| \le \text{min\_scale}$, returns `Some(0.0)` (exact baseline location match).
    /// - If $|x - \text{location}| > \text{min\_scale}$, returns `None` (z-score unavailable when scale lacks variation).
    pub fn normalize_feature(
        value: Option<f64>,
        stats: &BaselineFeatureStats,
        config: &NormalizationConfig,
        direction: FeatureDirection,
    ) -> Option<f64> {
        let x = value?;
        if !stats.is_valid || !x.is_finite() {
            return None;
        }

        let (location, scale) = match config.method {
            NormalizationMethod::ZScore => (stats.mean?, stats.std?),
            NormalizationMethod::RobustMedianMad => {
                (stats.median?, config.mad_multiplier * stats.mad?)
            }
        };

        if scale <= config.min_scale {
            if (x - location).abs() <= config.min_scale {
                return Some(0.0);
            } else {
                return None;
            }
        }

        let z = (x - location) / scale;

        let z_directed = match direction {
            FeatureDirection::Positive => z,
            FeatureDirection::Negative => -z,
        };

        let score = (z_directed / config.bounded_scale).tanh();
        if score.is_finite() { Some(score) } else { None }
    }
}
