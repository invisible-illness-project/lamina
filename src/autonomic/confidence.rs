use crate::autonomic::config::QualityConfig;
use crate::features::MultimodalFeatureVector;

/// Multi-tiered physiological state confidence and evidence completeness summary.
///
/// All non-`None` confidence values represent evidence completeness and data quality in $[0.0, 1.0]$.
/// They represent evidence strength and temporal completeness, **not** uncalibrated statistical probabilities.
#[derive(Debug, Clone, PartialEq)]
pub struct StateConfidence {
    /// Overall composite state confidence index in $[0.0, 1.0]$
    pub overall: Option<f64>,
    /// Cardiac modality state confidence index in $[0.0, 1.0]$
    pub cardiac: Option<f64>,
    /// Electrodermal modality state confidence index in $[0.0, 1.0]$
    pub electrodermal: Option<f64>,
    /// Respiratory modality state confidence index in $[0.0, 1.0]$
    pub respiratory: Option<f64>,
    /// Cross-modal coupling state confidence index in $[0.0, 1.0]$
    pub coupling: Option<f64>,
}

impl StateConfidence {
    /// Compute multi-tiered state confidence metrics from a windowed feature vector using configurable quality settings.
    pub fn compute(fv: &MultimodalFeatureVector, config: &QualityConfig) -> Self {
        let conf_cfg = &config.confidence;

        // 1. Cardiac Confidence
        let cardiac = if fv.cardiac.mean_hr_bpm.is_some() {
            let cov = fv
                .quality
                .modality_coverage
                .ecg
                .unwrap_or(fv.quality.coverage)
                .clamp(0.0, 1.0);
            let beat_factor = (fv.cardiac.beat_count as f64 / conf_cfg.min_beats as f64)
                .clamp(conf_cfg.quality_factor, 1.0);
            let valid_factor = if fv.quality.cardiac_valid {
                1.0
            } else {
                conf_cfg.quality_factor
            };
            Some((cov * beat_factor * valid_factor).clamp(0.0, 1.0))
        } else {
            None
        };

        // 2. Electrodermal Confidence
        let electrodermal = if fv.eda.mean_tonic_us.is_some() {
            let cov = fv
                .quality
                .modality_coverage
                .eda
                .unwrap_or(fv.quality.coverage)
                .clamp(0.0, 1.0);
            let valid_factor = if fv.quality.eda_valid {
                1.0
            } else {
                conf_cfg.quality_factor
            };
            Some((cov * valid_factor).clamp(0.0, 1.0))
        } else {
            None
        };

        // 3. Respiratory Confidence
        let respiratory = if fv.respiration.mean_rate_bpm.is_some() {
            let cov = fv
                .quality
                .modality_coverage
                .rsp
                .unwrap_or(fv.quality.coverage)
                .clamp(0.0, 1.0);
            let cycle_factor = (fv.respiration.cycle_count as f64 / conf_cfg.min_cycles as f64)
                .clamp(conf_cfg.quality_factor, 1.0);
            let valid_factor = if fv.quality.respiration_valid {
                1.0
            } else {
                conf_cfg.quality_factor
            };
            Some((cov * cycle_factor * valid_factor).clamp(0.0, 1.0))
        } else {
            None
        };
        // 4. Coupling Confidence
        // Per Buron & Menuet (2026) and Gevonden et al. (2025): RespHRV requires valid respiration context.
        let coupling = if fv.quality.coupling_valid {
            let mut conf_sum = 0.0;
            let mut count = 0.0;

            if fv.coupling.mean_pulse_delay_sec.is_some() {
                conf_sum += 1.0;
                count += 1.0;
            }
            if fv.coupling.cardiac_respiratory_concentration.is_some() {
                conf_sum += 1.0;
                count += 1.0;
            }
            if fv.coupling.rsa_amplitude_bpm.is_some() {
                count += 1.0;
                if fv.quality.respiration_valid && fv.respiration.mean_rate_bpm.is_some() {
                    conf_sum += 1.0;
                }
            }

            if count > 0.0 {
                let cov = fv.quality.coverage.clamp(0.0, 1.0);
                Some(((conf_sum / count) * cov).clamp(0.0, 1.0))
            } else {
                None
            }
        } else {
            None
        };

        // 5. Overall Confidence (Weighted average across all configured modalities)
        let total_weight = conf_cfg.cardiac_weight
            + conf_cfg.eda_weight
            + conf_cfg.rsp_weight
            + conf_cfg.coupling_weight;

        let overall_sum = conf_cfg.cardiac_weight * cardiac.unwrap_or(0.0)
            + conf_cfg.eda_weight * electrodermal.unwrap_or(0.0)
            + conf_cfg.rsp_weight * respiratory.unwrap_or(0.0)
            + conf_cfg.coupling_weight * coupling.unwrap_or(0.0);

        let overall = if total_weight > 0.0 {
            Some((overall_sum / total_weight).clamp(0.0, 1.0))
        } else {
            None
        };

        Self {
            overall,
            cardiac,
            electrodermal,
            respiratory,
            coupling,
        }
    }
}
