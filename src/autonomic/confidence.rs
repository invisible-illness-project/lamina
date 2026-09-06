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
    /// Compute multi-tiered state confidence metrics from a windowed feature vector.
    pub fn compute(fv: &MultimodalFeatureVector) -> Self {
        // 1. Cardiac Confidence
        let cardiac = if fv.cardiac.mean_hr_bpm.is_some() {
            let cov = fv
                .quality
                .modality_coverage
                .ecg
                .unwrap_or(fv.quality.coverage)
                .clamp(0.0, 1.0);
            let beat_factor = (fv.cardiac.beat_count as f64 / 10.0).clamp(0.2, 1.0);
            let valid_factor = if fv.quality.cardiac_valid { 1.0 } else { 0.5 };
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
            let valid_factor = if fv.quality.eda_valid { 1.0 } else { 0.5 };
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
            let cycle_factor = (fv.respiration.cycle_count as f64 / 4.0).clamp(0.2, 1.0);
            let valid_factor = if fv.quality.respiration_valid {
                1.0
            } else {
                0.5
            };
            Some((cov * cycle_factor * valid_factor).clamp(0.0, 1.0))
        } else {
            None
        };

        // 4. Coupling Confidence
        // Per Buron & Menuet (2026) and Gevonden et al. (2025): RespHRV requires direct valid respiration.
        let coupling = if fv.quality.coupling_valid {
            let mut contributors = Vec::new();
            if fv.coupling.mean_pulse_delay_sec.is_some() {
                contributors.push(0.9);
            }
            if fv.coupling.cardiac_respiratory_concentration.is_some() {
                contributors.push(0.95);
            }
            if fv.coupling.rsa_amplitude_bpm.is_some() {
                if fv.quality.respiration_valid {
                    contributors.push(1.0);
                } else {
                    // Respiration missing/invalid degrades RespHRV contribution
                    contributors.push(0.3);
                }
            }

            if !contributors.is_empty() {
                let mean_conf = contributors.iter().sum::<f64>() / contributors.len() as f64;
                Some(mean_conf.clamp(0.0, 1.0))
            } else {
                None
            }
        } else if fv.coupling.rsa_amplitude_bpm.is_some()
            || fv.coupling.mean_pulse_delay_sec.is_some()
        {
            Some(0.4)
        } else {
            None
        };

        // 5. Overall Confidence (Mean of non-None modality confidences)
        let present_confidences: Vec<f64> = [cardiac, electrodermal, respiratory, coupling]
            .into_iter()
            .flatten()
            .collect();

        let overall = if !present_confidences.is_empty() {
            Some(
                (present_confidences.iter().sum::<f64>() / present_confidences.len() as f64)
                    .clamp(0.0, 1.0),
            )
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
