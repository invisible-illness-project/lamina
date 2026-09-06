use crate::api::error::SignalError;
use crate::api::rsp::RespirationCycle;
use lamina::multimodal::config::{PulseTimingConfig as CorePulseTimingConfig, RsaConfig as CoreRsaConfig};
use lamina::multimodal::coupling::{
    cardiorespiratory_phase_coupling as core_phase_coupling, PhaseCouplingResult as CorePhaseCouplingResult,
};
use lamina::multimodal::ecg_ppg::{
    ecg_ppg_timing_config as core_ecg_ppg_timing_config, PulseTimingResult as CorePulseTimingResult,
};
use lamina::multimodal::rsa::{
    rsa_config as core_rsa_config, CardiacRespiratoryEvent as CoreCardiacRespiratoryEvent,
    RsaResult as CoreRsaResult,
};
use lamina::rsp::RespirationCycle as CoreRespirationCycle;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct PulseTimingConfig {
    pub min_delay_sec: Option<f64>,
    pub max_delay_sec: Option<f64>,
}

impl From<PulseTimingConfig> for CorePulseTimingConfig {
    fn from(c: PulseTimingConfig) -> Self {
        Self {
            min_delay_sec: c.min_delay_sec,
            max_delay_sec: c.max_delay_sec,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PulseTimingResult {
    pub ecg_peak_index: usize,
    pub ppg_peak_index: usize,
    pub ecg_timestamp_sec: f64,
    pub ppg_timestamp_sec: f64,
    pub pulse_delay_sec: f64,
}

impl From<CorePulseTimingResult> for PulseTimingResult {
    fn from(r: CorePulseTimingResult) -> Self {
        Self {
            ecg_peak_index: r.ecg_peak_index,
            ppg_peak_index: r.ppg_peak_index,
            ecg_timestamp_sec: r.ecg_timestamp_sec,
            ppg_timestamp_sec: r.ppg_timestamp_sec,
            pulse_delay_sec: r.pulse_delay_sec,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PhaseCouplingResult {
    pub mean_phase_rad: f64,
    pub vector_length: f64,
    pub sample_count: usize,
}

impl From<CorePhaseCouplingResult> for PhaseCouplingResult {
    fn from(r: CorePhaseCouplingResult) -> Self {
        Self {
            mean_phase_rad: r.mean_phase,
            vector_length: r.concentration,
            sample_count: r.sample_count,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CardiacRespiratoryEvent {
    pub r_peak_index: usize,
    pub timestamp_sec: f64,
    pub respiratory_phase: f64,
    pub rr_interval_sec: Option<f64>,
    pub heart_rate_bpm: Option<f64>,
}

impl From<CoreCardiacRespiratoryEvent> for CardiacRespiratoryEvent {
    fn from(e: CoreCardiacRespiratoryEvent) -> Self {
        Self {
            r_peak_index: e.r_peak_index,
            timestamp_sec: e.timestamp_sec,
            respiratory_phase: e.respiratory_phase,
            rr_interval_sec: e.rr_interval_sec,
            heart_rate_bpm: e.heart_rate_bpm,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RsaResult {
    pub amplitude_bpm: f64,
    pub amplitude_rr_sec: f64,
    pub valid_beats: usize,
    pub valid_cycles: usize,
}

impl From<CoreRsaResult> for RsaResult {
    fn from(r: CoreRsaResult) -> Self {
        Self {
            amplitude_bpm: r.amplitude_bpm,
            amplitude_rr_sec: r.amplitude_rr_sec,
            valid_beats: r.valid_beats,
            valid_cycles: r.valid_cycles,
        }
    }
}

pub fn compute_ecg_ppg_timing(
    ecg_peaks: Vec<usize>,
    ecg_sampling_rate: f64,
    ecg_offset_sec: f64,
    ppg_peaks: Vec<usize>,
    ppg_sampling_rate: f64,
    ppg_offset_sec: f64,
    config: Option<PulseTimingConfig>,
) -> Result<Vec<PulseTimingResult>, SignalError> {
    let cfg = config.map(CorePulseTimingConfig::from).unwrap_or_default();
    let results = core_ecg_ppg_timing_config(
        &ecg_peaks,
        ecg_sampling_rate,
        ecg_offset_sec,
        &ppg_peaks,
        ppg_sampling_rate,
        ppg_offset_sec,
        &cfg,
    )?;
    Ok(results.into_iter().map(PulseTimingResult::from).collect())
}

pub fn compute_cardiorespiratory_phase_coupling(
    phases: Vec<f64>,
) -> Result<PhaseCouplingResult, SignalError> {
    let res = core_phase_coupling(&phases)?;
    Ok(PhaseCouplingResult::from(res))
}

pub fn compute_rsa(
    r_peaks: Vec<usize>,
    ecg_sampling_rate: f64,
    ecg_offset_sec: f64,
    rsp_cycles: Vec<RespirationCycle>,
    rsp_sampling_rate: f64,
    rsp_offset_sec: f64,
) -> Result<RsaResult, SignalError> {
    let core_cycles: Vec<CoreRespirationCycle> = rsp_cycles
        .into_iter()
        .map(|c| CoreRespirationCycle {
            inspiration_index: c.inspiration_index,
            expiration_index: c.expiration_index,
            next_inspiration_index: c.next_inspiration_index,
            duration_sec: c.duration_sec,
            respiratory_rate_bpm: c.respiratory_rate_bpm,
            amplitude: c.amplitude,
        })
        .collect();
    let cfg = CoreRsaConfig::default();
    let res = core_rsa_config(
        &r_peaks,
        ecg_sampling_rate,
        ecg_offset_sec,
        &core_cycles,
        rsp_sampling_rate,
        rsp_offset_sec,
        &cfg,
    )?;
    Ok(RsaResult::from(res))
}
