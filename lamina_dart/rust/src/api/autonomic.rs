use crate::api::error::SignalError;
use lamina::autonomic::config::AutonomicEstimatorConfig as CoreAutonomicEstimatorConfig;
use lamina::autonomic::estimator::AutonomicEstimator as CoreAutonomicEstimator;
use lamina::autonomic::normalization::AutonomicBaseline as CoreAutonomicBaseline;
use lamina::autonomic::state::{
    AutonomicState as CoreAutonomicState, CardiacState as CoreCardiacState,
    CouplingState as CoreCouplingState, ElectrodermalState as CoreElectrodermalState,
    RespiratoryState as CoreRespiratoryState,
};
use lamina::features::cardiac::CardiacFeatures as CoreCardiacFeatures;
use lamina::features::coupling::CouplingFeatures as CoreCouplingFeatures;
use lamina::features::eda::EdaFeatures as CoreEdaFeatures;
use lamina::features::quality::{
    FeatureCoverage as CoreFeatureCoverage, FeatureQuality as CoreFeatureQuality,
};
use lamina::features::respiration::RespirationFeatures as CoreRespirationFeatures;
use lamina::features::window::FeatureWindow as CoreFeatureWindow;
use lamina::features::MultimodalFeatureVector as CoreMultimodalFeatureVector;

#[derive(Debug, Clone, PartialEq)]
pub struct CardiacState {
    pub variability_index: Option<f64>,
    pub heart_rate_index: Option<f64>,
    pub recovery_evidence: Option<f64>,
    pub beat_count: usize,
}

impl From<CoreCardiacState> for CardiacState {
    fn from(c: CoreCardiacState) -> Self {
        Self {
            variability_index: c.variability_index,
            heart_rate_index: c.heart_rate_index,
            recovery_evidence: c.recovery_evidence,
            beat_count: c.beat_count,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ElectrodermalState {
    pub tonic_level_index: Option<f64>,
    pub phasic_activation_index: Option<f64>,
    pub scr_rate_index: Option<f64>,
    pub scr_count: usize,
}

impl From<CoreElectrodermalState> for ElectrodermalState {
    fn from(e: CoreElectrodermalState) -> Self {
        Self {
            tonic_level_index: e.tonic_level_index,
            phasic_activation_index: e.phasic_activation_index,
            scr_rate_index: e.scr_rate_index,
            scr_count: e.scr_count,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RespiratoryState {
    pub rate_index: Option<f64>,
    pub amplitude_index: Option<f64>,
    pub regularity_index: Option<f64>,
    pub cycle_count: usize,
}

impl From<CoreRespiratoryState> for RespiratoryState {
    fn from(r: CoreRespiratoryState) -> Self {
        Self {
            rate_index: r.rate_index,
            amplitude_index: r.amplitude_index,
            regularity_index: r.regularity_index,
            cycle_count: r.cycle_count,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CouplingState {
    pub resphr_coupling_index: Option<f64>,
    pub phase_coupling_index: Option<f64>,
    pub pulse_delay_index: Option<f64>,
    pub association_count: usize,
}

impl From<CoreCouplingState> for CouplingState {
    fn from(c: CoreCouplingState) -> Self {
        Self {
            resphr_coupling_index: c.resphr_coupling_index,
            phase_coupling_index: c.phase_coupling_index,
            pulse_delay_index: c.pulse_delay_index,
            association_count: c.association_count,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AutonomicState {
    pub timestamp: f64,
    pub duration_sec: f64,
    pub cardiac: CardiacState,
    pub electrodermal: ElectrodermalState,
    pub respiratory: RespiratoryState,
    pub coupling: CouplingState,
    pub activation_score: Option<f64>,
    pub regulation_score: Option<f64>,
}

impl From<CoreAutonomicState> for AutonomicState {
    fn from(s: CoreAutonomicState) -> Self {
        Self {
            timestamp: s.timestamp,
            duration_sec: s.duration_sec,
            cardiac: CardiacState::from(s.cardiac),
            electrodermal: ElectrodermalState::from(s.electrodermal),
            respiratory: RespiratoryState::from(s.respiratory),
            coupling: CouplingState::from(s.coupling),
            activation_score: s.activation_score,
            regulation_score: s.regulation_score,
        }
    }
}

pub struct AutonomicEstimator {
    inner: CoreAutonomicEstimator,
}

impl AutonomicEstimator {
    pub fn new() -> Self {
        Self {
            inner: CoreAutonomicEstimator::new(CoreAutonomicEstimatorConfig::default()),
        }
    }

    pub fn estimate_from_vector(&self) -> Result<AutonomicState, SignalError> {
        let fv = CoreMultimodalFeatureVector {
            window: CoreFeatureWindow {
                start_time_sec: 0.0,
                end_time_sec: 1.0,
                duration_sec: 1.0,
            },
            cardiac: CoreCardiacFeatures::empty(),
            eda: CoreEdaFeatures {
                mean_tonic_us: None,
                median_tonic_us: None,
                tonic_std_us: None,
                mean_phasic_us: None,
                phasic_std_us: None,
                scr_count: 0,
                scr_rate_per_min: None,
                mean_scr_amplitude_us: None,
                median_scr_amplitude_us: None,
                mean_scr_rise_time_sec: None,
            },
            respiration: CoreRespirationFeatures {
                mean_rate_bpm: None,
                median_rate_bpm: None,
                rate_std_bpm: None,
                mean_cycle_duration_sec: None,
                cycle_count: 0,
                mean_amplitude: None,
                amplitude_std: None,
            },
            coupling: CoreCouplingFeatures {
                rsa_amplitude_bpm: None,
                rsa_amplitude_rr_sec: None,
                cardiac_respiratory_concentration: None,
                cardiac_respiratory_mean_phase: None,
                mean_pulse_delay_sec: None,
                pulse_delay_std_sec: None,
                scr_cardiac_association_count: 0,
            },
            quality: CoreFeatureQuality {
                coverage: 1.0,
                modality_coverage: CoreFeatureCoverage {
                    overall: 1.0,
                    ecg: Some(1.0),
                    ppg: Some(1.0),
                    eda: Some(1.0),
                    rsp: Some(1.0),
                },
                cardiac_valid: true,
                eda_valid: true,
                respiration_valid: true,
                coupling_valid: true,
                usable_feature_count: 0,
                total_feature_count: 20,
                issues: vec![],
            },
        };
        let baseline = CoreAutonomicBaseline::default();
        let state = self.inner.estimate(&fv, &baseline)?;
        Ok(AutonomicState::from(state))
    }
}

impl Default for AutonomicEstimator {
    fn default() -> Self {
        Self::new()
    }
}
