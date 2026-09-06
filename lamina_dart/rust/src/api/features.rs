use crate::api::eda::EdaPeakEvent;
use crate::api::error::SignalError;
use crate::api::rsp::RespirationCycle;
use lamina::eda::ScrEvent as CoreScrEvent;
use lamina::features::cardiac::{cardiac_features as core_cardiac_features, CardiacFeatures as CoreCardiacFeatures};
use lamina::features::eda::{eda_features as core_eda_features, EdaFeatures as CoreEdaFeatures};
use lamina::features::respiration::{respiration_features as core_respiration_features, RespirationFeatures as CoreRespirationFeatures};
use lamina::features::window::FeatureWindow as CoreFeatureWindow;
use lamina::rsp::RespirationCycle as CoreRespirationCycle;
use ndarray::Array1;

#[derive(Debug, Clone, PartialEq)]
pub struct FeatureWindow {
    pub start_time_sec: f64,
    pub end_time_sec: f64,
}

impl From<FeatureWindow> for CoreFeatureWindow {
    fn from(w: FeatureWindow) -> Self {
        Self {
            start_time_sec: w.start_time_sec,
            end_time_sec: w.end_time_sec,
            duration_sec: w.end_time_sec - w.start_time_sec,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CardiacFeatures {
    pub mean_hr_bpm: Option<f64>,
    pub median_hr_bpm: Option<f64>,
    pub sdnn_ms: Option<f64>,
    pub rmssd_ms: Option<f64>,
    pub pnn50: Option<f64>,
    pub rr_mean_ms: Option<f64>,
    pub rr_std_ms: Option<f64>,
    pub beat_count: usize,
}

impl From<CoreCardiacFeatures> for CardiacFeatures {
    fn from(c: CoreCardiacFeatures) -> Self {
        Self {
            mean_hr_bpm: c.mean_hr_bpm,
            median_hr_bpm: c.median_hr_bpm,
            sdnn_ms: c.sdnn_ms,
            rmssd_ms: c.rmssd_ms,
            pnn50: c.pnn50,
            rr_mean_ms: c.rr_mean_ms,
            rr_std_ms: c.rr_std_ms,
            beat_count: c.beat_count,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EdaFeatures {
    pub mean_tonic_us: Option<f64>,
    pub median_tonic_us: Option<f64>,
    pub tonic_std_us: Option<f64>,
    pub mean_phasic_us: Option<f64>,
    pub phasic_std_us: Option<f64>,
    pub scr_count: usize,
    pub scr_rate_per_min: Option<f64>,
    pub mean_scr_amplitude_us: Option<f64>,
    pub median_scr_amplitude_us: Option<f64>,
    pub mean_scr_rise_time_sec: Option<f64>,
}

impl From<CoreEdaFeatures> for EdaFeatures {
    fn from(e: CoreEdaFeatures) -> Self {
        Self {
            mean_tonic_us: e.mean_tonic_us,
            median_tonic_us: e.median_tonic_us,
            tonic_std_us: e.tonic_std_us,
            mean_phasic_us: e.mean_phasic_us,
            phasic_std_us: e.phasic_std_us,
            scr_count: e.scr_count,
            scr_rate_per_min: e.scr_rate_per_min,
            mean_scr_amplitude_us: e.mean_scr_amplitude_us,
            median_scr_amplitude_us: e.median_scr_amplitude_us,
            mean_scr_rise_time_sec: e.mean_scr_rise_time_sec,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RespirationFeatures {
    pub mean_rate_bpm: Option<f64>,
    pub median_rate_bpm: Option<f64>,
    pub rate_std_bpm: Option<f64>,
    pub mean_cycle_duration_sec: Option<f64>,
    pub cycle_count: usize,
    pub mean_amplitude: Option<f64>,
    pub amplitude_std: Option<f64>,
}

impl From<CoreRespirationFeatures> for RespirationFeatures {
    fn from(r: CoreRespirationFeatures) -> Self {
        Self {
            mean_rate_bpm: r.mean_rate_bpm,
            median_rate_bpm: r.median_rate_bpm,
            rate_std_bpm: r.rate_std_bpm,
            mean_cycle_duration_sec: r.mean_cycle_duration_sec,
            cycle_count: r.cycle_count,
            mean_amplitude: r.mean_amplitude,
            amplitude_std: r.amplitude_std,
        }
    }
}

pub fn extract_cardiac_features(
    r_peaks: Vec<usize>,
    sampling_rate: f64,
    offset_sec: f64,
    window: FeatureWindow,
) -> Result<CardiacFeatures, SignalError> {
    let core_win = CoreFeatureWindow::from(window);
    let res = core_cardiac_features(&r_peaks, sampling_rate, offset_sec, &core_win)?;
    Ok(CardiacFeatures::from(res))
}

pub fn extract_eda_features(
    tonic: Vec<f64>,
    phasic: Vec<f64>,
    events: Vec<EdaPeakEvent>,
    sampling_rate: f64,
    offset_sec: f64,
    window: FeatureWindow,
) -> Result<EdaFeatures, SignalError> {
    let nd_tonic = Array1::from_vec(tonic);
    let nd_phasic = Array1::from_vec(phasic);
    let core_events: Vec<CoreScrEvent> = events
        .into_iter()
        .map(|e| CoreScrEvent {
            onset_index: e.onset_index,
            peak_index: e.peak_index,
            amplitude: e.amplitude,
            rise_time_sec: e.rise_time_sec,
        })
        .collect();
    let core_win = CoreFeatureWindow::from(window);
    let res = core_eda_features(
        &nd_tonic,
        &nd_phasic,
        &core_events,
        sampling_rate,
        offset_sec,
        &core_win,
    )?;
    Ok(EdaFeatures::from(res))
}

pub fn extract_respiration_features(
    cycles: Vec<RespirationCycle>,
    sampling_rate: f64,
    offset_sec: f64,
    window: FeatureWindow,
) -> Result<RespirationFeatures, SignalError> {
    let core_cycles: Vec<CoreRespirationCycle> = cycles
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
    let core_win = CoreFeatureWindow::from(window);
    let res = core_respiration_features(&core_cycles, sampling_rate, offset_sec, &core_win)?;
    Ok(RespirationFeatures::from(res))
}
