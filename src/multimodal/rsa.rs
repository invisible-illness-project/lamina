use crate::error::{Result, SignalError};
use crate::multimodal::config::RsaConfig;
use crate::multimodal::phase::respiratory_phase_at_time;
use crate::multimodal::sync::sample_to_time;
use crate::rsp::RespirationCycle;
use std::f64::consts::PI;

/// A cardiac R-peak event mapped into physical time and continuous respiratory phase.
#[derive(Debug, Clone, PartialEq)]
pub struct CardiacRespiratoryEvent {
    /// R-peak sample index in raw ECG signal
    pub r_peak_index: usize,
    /// Physical timestamp in seconds
    pub timestamp_sec: f64,
    /// Continuous respiratory phase in radians $\phi \in [0, 2\pi)$
    pub respiratory_phase: f64,
    /// Inter-beat interval (R-R interval) to next R-peak in seconds
    pub rr_interval_sec: Option<f64>,
    /// Instantaneous heart rate in beats per minute (BPM)
    pub heart_rate_bpm: Option<f64>,
}

/// Respiratory Sinus Arrhythmia (RSA) estimation summary.
#[derive(Debug, Clone, PartialEq)]
pub struct RsaResult {
    /// Mean within-cycle inspiratory peak to expiratory trough heart rate modulation in BPM.
    pub amplitude_bpm: f64,
    /// Mean within-cycle expiratory peak to inspiratory trough R-R interval difference in seconds.
    pub amplitude_rr_sec: f64,
    /// Total count of valid cardiac beats assigned a respiratory phase.
    pub valid_beats: usize,
    /// Total count of respiration cycles with both inspiratory and expiratory cardiac beats.
    pub valid_cycles: usize,
}

/// Assign continuous respiratory phase and R-R intervals to ECG R-peaks.
///
/// # Errors
/// Returns [`SignalError::InvalidSamplingRate`] if `ecg_sampling_rate` or `rsp_sampling_rate` is $\le 0.0$ or non-finite.
/// Returns [`SignalError::NonFiniteInput`] if offsets are non-finite.
pub fn cardiac_respiratory_phase(
    r_peaks: &[usize],
    ecg_sampling_rate: f64,
    ecg_offset_sec: f64,
    rsp_cycles: &[RespirationCycle],
    rsp_sampling_rate: f64,
    rsp_offset_sec: f64,
) -> Result<Vec<CardiacRespiratoryEvent>> {
    if !ecg_sampling_rate.is_finite() || ecg_sampling_rate <= 0.0 {
        return Err(SignalError::InvalidSamplingRate(ecg_sampling_rate));
    }
    if !rsp_sampling_rate.is_finite() || rsp_sampling_rate <= 0.0 {
        return Err(SignalError::InvalidSamplingRate(rsp_sampling_rate));
    }
    if !ecg_offset_sec.is_finite() || !rsp_offset_sec.is_finite() {
        return Err(SignalError::NonFiniteInput);
    }

    let mut events = Vec::with_capacity(r_peaks.len());

    for i in 0..r_peaks.len() {
        let t_sec = sample_to_time(r_peaks[i], ecg_sampling_rate, ecg_offset_sec)?;

        let rr_interval_sec = if i + 1 < r_peaks.len() {
            let next_t_sec = sample_to_time(r_peaks[i + 1], ecg_sampling_rate, ecg_offset_sec)?;
            let diff = next_t_sec - t_sec;
            if diff > 0.0 { Some(diff) } else { None }
        } else {
            None
        };

        let heart_rate_bpm = rr_interval_sec.map(|rr| 60.0 / rr);

        if let Some(phase) =
            respiratory_phase_at_time(rsp_cycles, t_sec, rsp_sampling_rate, rsp_offset_sec)
        {
            events.push(CardiacRespiratoryEvent {
                r_peak_index: r_peaks[i],
                timestamp_sec: t_sec,
                respiratory_phase: phase,
                rr_interval_sec,
                heart_rate_bpm,
            });
        }
    }

    Ok(events)
}

/// Compute Respiratory Sinus Arrhythmia (RSA) amplitude with custom configuration.
///
/// # Operational Definition
/// For each valid respiration cycle $k$:
/// 1. Inspiratory cardiac beats ($\phi \in [0, \pi)$) and expiratory cardiac beats ($\phi \in [\pi, 2\pi)$) are identified.
/// 2. If both phases contain cardiac beats, peak within-cycle heart rate modulation is calculated:
///    $$\Delta \text{BPM}_k = \max_{i \in \text{insp}} \text{BPM}_i - \min_{e \in \text{exp}} \text{BPM}_e$$
///    $$\Delta \text{RR}_k = \max_{e \in \text{exp}} \text{RR}_e - \min_{i \in \text{insp}} \text{RR}_i$$
/// 3. RSA amplitude is the mean of $\Delta \text{BPM}_k$ and $\Delta \text{RR}_k$ across all valid cycles.
///
/// # Errors
/// Returns [`SignalError::InsufficientPeaks`] if valid beat count $< \text{min\_valid\_beats}$ or if 0 cycles contain valid beats.
pub fn rsa_config(
    r_peaks: &[usize],
    ecg_sampling_rate: f64,
    ecg_offset_sec: f64,
    rsp_cycles: &[RespirationCycle],
    rsp_sampling_rate: f64,
    rsp_offset_sec: f64,
    config: &RsaConfig,
) -> Result<RsaResult> {
    let min_beats = config.min_valid_beats.unwrap_or(3);

    let events = cardiac_respiratory_phase(
        r_peaks,
        ecg_sampling_rate,
        ecg_offset_sec,
        rsp_cycles,
        rsp_sampling_rate,
        rsp_offset_sec,
    )?;

    if events.len() < min_beats {
        return Err(SignalError::InsufficientPeaks {
            required: min_beats,
            provided: events.len(),
        });
    }

    let mut cycle_bpm_diffs = Vec::new();
    let mut cycle_rr_diffs = Vec::new();

    for cycle in rsp_cycles {
        let t0 = sample_to_time(cycle.inspiration_index, rsp_sampling_rate, rsp_offset_sec)?;
        let t2 = sample_to_time(
            cycle.next_inspiration_index,
            rsp_sampling_rate,
            rsp_offset_sec,
        )?;

        let cycle_events: Vec<&CardiacRespiratoryEvent> = events
            .iter()
            .filter(|e| e.timestamp_sec >= t0 && e.timestamp_sec <= t2)
            .collect();

        let insp_beats: Vec<f64> = cycle_events
            .iter()
            .filter(|e| e.respiratory_phase < PI)
            .filter_map(|e| e.heart_rate_bpm)
            .collect();

        let exp_beats: Vec<f64> = cycle_events
            .iter()
            .filter(|e| e.respiratory_phase >= PI)
            .filter_map(|e| e.heart_rate_bpm)
            .collect();

        let insp_rrs: Vec<f64> = cycle_events
            .iter()
            .filter(|e| e.respiratory_phase < PI)
            .filter_map(|e| e.rr_interval_sec)
            .collect();

        let exp_rrs: Vec<f64> = cycle_events
            .iter()
            .filter(|e| e.respiratory_phase >= PI)
            .filter_map(|e| e.rr_interval_sec)
            .collect();

        if !insp_beats.is_empty() && !exp_beats.is_empty() {
            let max_insp_bpm = insp_beats.iter().cloned().fold(f64::MIN, f64::max);
            let min_exp_bpm = exp_beats.iter().cloned().fold(f64::MAX, f64::min);
            let bpm_diff = (max_insp_bpm - min_exp_bpm).max(0.0);
            cycle_bpm_diffs.push(bpm_diff);
        }

        if !insp_rrs.is_empty() && !exp_rrs.is_empty() {
            let min_insp_rr = insp_rrs.iter().cloned().fold(f64::MAX, f64::min);
            let max_exp_rr = exp_rrs.iter().cloned().fold(f64::MIN, f64::max);
            let rr_diff = (max_exp_rr - min_insp_rr).max(0.0);
            cycle_rr_diffs.push(rr_diff);
        }
    }

    if cycle_bpm_diffs.is_empty() {
        return Err(SignalError::InsufficientPeaks {
            required: 1,
            provided: 0,
        });
    }

    let mean_bpm_diff = cycle_bpm_diffs.iter().sum::<f64>() / cycle_bpm_diffs.len() as f64;
    let mean_rr_diff = cycle_rr_diffs.iter().sum::<f64>() / cycle_rr_diffs.len() as f64;

    Ok(RsaResult {
        amplitude_bpm: mean_bpm_diff,
        amplitude_rr_sec: mean_rr_diff,
        valid_beats: events.len(),
        valid_cycles: cycle_bpm_diffs.len(),
    })
}

/// Compute RSA amplitude with default settings.
pub fn rsa(
    r_peaks: &[usize],
    ecg_sampling_rate: f64,
    ecg_offset_sec: f64,
    rsp_cycles: &[RespirationCycle],
    rsp_sampling_rate: f64,
    rsp_offset_sec: f64,
) -> Result<RsaResult> {
    rsa_config(
        r_peaks,
        ecg_sampling_rate,
        ecg_offset_sec,
        rsp_cycles,
        rsp_sampling_rate,
        rsp_offset_sec,
        &RsaConfig::default(),
    )
}
