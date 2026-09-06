use crate::autonomic::confidence::StateConfidence;
use crate::autonomic::config::{AutonomicEstimatorConfig, FeatureDirection};
use crate::autonomic::normalization::AutonomicBaseline;
use crate::autonomic::state::{
    AutonomicState, AutonomicStateSeries, CardiacState, CouplingState, ElectrodermalState,
    RespiratoryState,
};
use crate::error::{Result, SignalError};
use crate::features::MultimodalFeatureVector;

/// Deterministic multimodal physiological state estimator.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AutonomicEstimator {
    /// Estimator configuration
    pub config: AutonomicEstimatorConfig,
}

impl AutonomicEstimator {
    /// Construct a new [`AutonomicEstimator`] with the specified configuration.
    pub fn new(config: AutonomicEstimatorConfig) -> Self {
        Self { config }
    }

    /// Estimate physiological state for a single feature window against a fitted baseline.
    pub fn estimate(
        &self,
        fv: &MultimodalFeatureVector,
        baseline: &AutonomicBaseline,
    ) -> Result<AutonomicState> {
        self.config.validate()?;

        // 1. Cardiac State Evidence
        let cardiac_cov = fv
            .quality
            .modality_coverage
            .ecg
            .unwrap_or(fv.quality.coverage);
        let cardiac_valid = if self.config.quality.require_cardiac_validity {
            fv.quality.cardiac_valid && cardiac_cov >= self.config.quality.min_coverage
        } else {
            cardiac_cov >= self.config.quality.min_coverage
        };

        let hr_index = if cardiac_valid {
            AutonomicBaseline::normalize_feature(
                fv.cardiac.mean_hr_bpm,
                &baseline.hr_bpm_stats,
                &self.config.normalization,
                FeatureDirection::Positive,
            )
        } else {
            None
        };

        let variability_index = if cardiac_valid {
            // SDNN preferred when valid, with RMSSD used as fallback
            let var_val = fv.cardiac.sdnn_ms.or(fv.cardiac.rmssd_ms);
            let var_stats = if fv.cardiac.sdnn_ms.is_some() {
                &baseline.sdnn_ms_stats
            } else {
                &baseline.rmssd_ms_stats
            };
            AutonomicBaseline::normalize_feature(
                var_val,
                var_stats,
                &self.config.normalization,
                FeatureDirection::Positive,
            )
        } else {
            None
        };

        // Engineered cardiac recovery evidence index: signed weighted average over available positive-weight contributors
        let w_var = self.config.recovery.variability_weight;
        let w_hr = self.config.recovery.heart_rate_weight;

        let mut recovery_num = 0.0;
        let mut recovery_denom = 0.0;

        if let Some(v) = variability_index.filter(|_| w_var > 0.0) {
            recovery_num += w_var * v;
            recovery_denom += w_var;
        }

        if let Some(h) = hr_index.filter(|_| w_hr > 0.0) {
            recovery_num += w_hr * (-h);
            recovery_denom += w_hr;
        }

        let recovery_evidence = if recovery_denom > 0.0 {
            Some((recovery_num / recovery_denom).clamp(-1.0, 1.0))
        } else {
            None
        };

        let cardiac = CardiacState {
            variability_index,
            heart_rate_index: hr_index,
            recovery_evidence,
            beat_count: fv.cardiac.beat_count,
        };

        // 2. Electrodermal State Evidence
        let eda_cov = fv
            .quality
            .modality_coverage
            .eda
            .unwrap_or(fv.quality.coverage);
        let eda_valid = eda_cov >= self.config.quality.min_coverage;

        let tonic_level_index = if eda_valid {
            AutonomicBaseline::normalize_feature(
                fv.eda.mean_tonic_us,
                &baseline.eda_tonic_stats,
                &self.config.normalization,
                FeatureDirection::Positive,
            )
        } else {
            None
        };

        let phasic_activation_index = if eda_valid {
            AutonomicBaseline::normalize_feature(
                fv.eda.mean_phasic_us,
                &baseline.eda_phasic_stats,
                &self.config.normalization,
                FeatureDirection::Positive,
            )
        } else {
            None
        };

        let scr_rate_index = if eda_valid {
            AutonomicBaseline::normalize_feature(
                fv.eda.scr_rate_per_min,
                &baseline.scr_rate_stats,
                &self.config.normalization,
                FeatureDirection::Positive,
            )
        } else {
            None
        };

        let electrodermal = ElectrodermalState {
            tonic_level_index,
            phasic_activation_index,
            scr_rate_index,
            scr_count: fv.eda.scr_count,
        };

        // 3. Respiratory State Evidence
        let rsp_cov = fv
            .quality
            .modality_coverage
            .rsp
            .unwrap_or(fv.quality.coverage);
        let rsp_valid = fv.quality.respiration_valid && rsp_cov >= self.config.quality.min_coverage;

        let rate_index = if rsp_valid {
            AutonomicBaseline::normalize_feature(
                fv.respiration.mean_rate_bpm,
                &baseline.rsp_rate_stats,
                &self.config.normalization,
                FeatureDirection::Positive,
            )
        } else {
            None
        };

        let amplitude_index = if rsp_valid {
            AutonomicBaseline::normalize_feature(
                fv.respiration.mean_amplitude,
                &baseline.rsp_amplitude_stats,
                &self.config.normalization,
                FeatureDirection::Positive,
            )
        } else {
            None
        };

        let regularity_index = if rsp_valid {
            AutonomicBaseline::normalize_feature(
                fv.respiration.rate_std_bpm,
                &baseline.rsp_std_stats,
                &self.config.normalization,
                FeatureDirection::Negative,
            )
        } else {
            None
        };

        let respiratory = RespiratoryState {
            rate_index,
            amplitude_index,
            regularity_index,
            cycle_count: fv.respiration.cycle_count,
        };

        // 4. Coupling State Evidence
        // Per Buron & Menuet (2026) and Gevonden et al. (2025): RespHRV strictly requires valid direct respiration context.
        let resphr_valid = if self.config.quality.require_respiration_for_resphrv {
            rsp_valid && fv.respiration.mean_rate_bpm.is_some()
        } else {
            true
        };

        let resphr_coupling_index = if resphr_valid {
            AutonomicBaseline::normalize_feature(
                fv.coupling.rsa_amplitude_bpm,
                &baseline.rsa_bpm_stats,
                &self.config.normalization,
                FeatureDirection::Positive,
            )
        } else {
            None
        };

        let phase_coupling_index = AutonomicBaseline::normalize_feature(
            fv.coupling.cardiac_respiratory_concentration,
            &baseline.phase_coupling_stats,
            &self.config.normalization,
            FeatureDirection::Positive,
        );

        let pulse_delay_index = AutonomicBaseline::normalize_feature(
            fv.coupling.mean_pulse_delay_sec,
            &baseline.pulse_delay_stats,
            &self.config.normalization,
            FeatureDirection::Positive,
        );

        let coupling = CouplingState {
            resphr_coupling_index,
            phase_coupling_index,
            pulse_delay_index,
            association_count: fv.coupling.scr_cardiac_association_count,
        };

        // 5. Multimodal Physiological Activation Evidence Index
        let mut act_sum = 0.0;
        let mut act_weight_sum = 0.0;

        if let Some(x) = hr_index {
            act_sum += self.config.activation.hr_weight * x;
            act_weight_sum += self.config.activation.hr_weight;
        }
        if let Some(x) = phasic_activation_index {
            act_sum += self.config.activation.eda_phasic_weight * x;
            act_weight_sum += self.config.activation.eda_phasic_weight;
        }
        if let Some(x) = scr_rate_index {
            act_sum += self.config.activation.scr_rate_weight * x;
            act_weight_sum += self.config.activation.scr_rate_weight;
        }
        if let Some(x) = rate_index {
            act_sum += self.config.activation.rsp_rate_weight * x;
            act_weight_sum += self.config.activation.rsp_rate_weight;
        }

        let activation_score = if act_weight_sum > 0.0 {
            Some((act_sum / act_weight_sum).clamp(-1.0, 1.0))
        } else {
            None
        };

        // 6. Multimodal Cardiorespiratory Regulation & Coupling Evidence Index
        let mut reg_sum = 0.0;
        let mut reg_weight_sum = 0.0;

        if let Some(x) = variability_index {
            reg_sum += self.config.regulation.cardiac_variability_weight * x;
            reg_weight_sum += self.config.regulation.cardiac_variability_weight;
        }
        if let Some(x) = resphr_coupling_index {
            reg_sum += self.config.regulation.resphr_coupling_weight * x;
            reg_weight_sum += self.config.regulation.resphr_coupling_weight;
        }
        if let Some(x) = phase_coupling_index {
            reg_sum += self.config.regulation.phase_coupling_weight * x;
            reg_weight_sum += self.config.regulation.phase_coupling_weight;
        }

        let regulation_score = if reg_weight_sum > 0.0 {
            Some((reg_sum / reg_weight_sum).clamp(-1.0, 1.0))
        } else {
            None
        };

        // 7. Multi-tiered State Confidence
        let confidence = StateConfidence::compute(fv, &self.config.quality);

        Ok(AutonomicState {
            timestamp: fv.window.start_time_sec,
            duration_sec: fv.window.duration_sec,
            cardiac,
            electrodermal,
            respiratory,
            coupling,
            activation_score,
            regulation_score,
            confidence,
        })
    }

    /// Estimate a state trajectory series across sequential feature windows, performing temporal consistency validation and preserving raw states.
    pub fn estimate_series(
        &self,
        feature_series: &[MultimodalFeatureVector],
        baseline: &AutonomicBaseline,
    ) -> Result<AutonomicStateSeries> {
        self.config.validate()?;
        if feature_series.is_empty() {
            return Ok(AutonomicStateSeries {
                states: Vec::new(),
                smoothed_states: None,
                window_duration_sec: 0.0,
                step_sec: 0.0,
            });
        }

        // Validate temporal metadata structure across entire series
        let window_duration_sec = feature_series[0].window.duration_sec;
        if !window_duration_sec.is_finite() || window_duration_sec <= 0.0 {
            return Err(SignalError::InvalidWindowSize(0));
        }

        let mut prev_t = feature_series[0].window.start_time_sec;
        if !prev_t.is_finite() {
            return Err(SignalError::NonFiniteInput);
        }

        let expected_step = if feature_series.len() > 1 {
            let s = feature_series[1].window.start_time_sec - prev_t;
            if !s.is_finite() || s <= 0.0 {
                return Err(SignalError::InvalidWindowSize(0));
            }
            s
        } else {
            window_duration_sec
        };

        for fv in feature_series.iter().skip(1) {
            let t = fv.window.start_time_sec;
            let d = fv.window.duration_sec;

            if !t.is_finite() || !d.is_finite() || d <= 0.0 {
                return Err(SignalError::NonFiniteInput);
            }
            if t <= prev_t {
                return Err(SignalError::InvalidWindowSize(0));
            }
            let step = t - prev_t;
            if (step - expected_step).abs() > 1e-4 {
                return Err(SignalError::InvalidWindowSize(0));
            }
            if (d - window_duration_sec).abs() > 1e-4 {
                return Err(SignalError::InvalidWindowSize(0));
            }
            prev_t = t;
        }

        // Estimate raw states across feature series
        let mut raw_states = Vec::with_capacity(feature_series.len());
        for fv in feature_series {
            raw_states.push(self.estimate(fv, baseline)?);
        }

        // Compute smoothed states separately if EMA smoothing is configured
        let smoothed_states = if let Some(ref smooth_cfg) = self.config.smoothing {
            let alpha = smooth_cfg.alpha;
            let mut smoothed = raw_states.clone();

            let mut prev_act: Option<f64> = None;
            let mut prev_reg: Option<f64> = None;

            for state in smoothed.iter_mut() {
                if let Some(x) = state.activation_score {
                    let s = match prev_act {
                        Some(p) => alpha * x + (1.0 - alpha) * p,
                        None => x,
                    };
                    state.activation_score = Some(s.clamp(-1.0, 1.0));
                    prev_act = Some(s);
                }

                if let Some(x) = state.regulation_score {
                    let s = match prev_reg {
                        Some(p) => alpha * x + (1.0 - alpha) * p,
                        None => x,
                    };
                    state.regulation_score = Some(s.clamp(-1.0, 1.0));
                    prev_reg = Some(s);
                }
            }
            Some(smoothed)
        } else {
            None
        };

        Ok(AutonomicStateSeries {
            states: raw_states,
            smoothed_states,
            window_duration_sec,
            step_sec: expected_step,
        })
    }
}
