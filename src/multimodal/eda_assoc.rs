use crate::eda::ScrEvent;
use crate::error::{Result, SignalError};
use crate::multimodal::phase::respiratory_phase_at_time;
use crate::multimodal::sync::sample_to_time;
use crate::rsp::RespirationCycle;

/// Temporal association between a Skin Conductance Response (SCR) event and cardiorespiratory states.
#[derive(Debug, Clone, PartialEq)]
pub struct ScrCardiorespiratoryAssociation {
    /// SCR peak sample index
    pub scr_peak_index: usize,
    /// Physical timestamp of SCR peak in seconds
    pub scr_peak_time_sec: f64,
    /// SCR amplitude in microsiemens ($\mu\text{S}$)
    pub scr_amplitude: f64,
    /// Respiratory phase at SCR peak timestamp $\phi \in [0, 2\pi)$, if within a valid cycle
    pub respiratory_phase_rad: Option<f64>,
    /// Physical timestamp of nearest ECG R-peak in seconds
    pub nearest_r_peak_time_sec: Option<f64>,
    /// Time offset $t_{\text{scr}} - t_{\text{ecg}}$ relative to nearest R-peak in seconds
    pub cardiac_delay_sec: Option<f64>,
}

/// Link SCR events with nearest ECG R-peaks and current respiratory phase.
///
/// # Non-Causal Temporal Association
/// This function computes observational temporal proximity and phase relationships without
/// asserting causal physiological relationships.
///
/// # Errors
/// Returns [`SignalError::InvalidSamplingRate`] if any sampling rate is $\le 0.0$ or non-finite.
/// Returns [`SignalError::NonFiniteInput`] if offsets are non-finite.
#[allow(clippy::too_many_arguments)]
pub fn eda_cardiorespiratory_association(
    scr_events: &[ScrEvent],
    eda_sampling_rate: f64,
    eda_offset_sec: f64,
    r_peaks: &[usize],
    ecg_sampling_rate: f64,
    ecg_offset_sec: f64,
    rsp_cycles: &[RespirationCycle],
    rsp_sampling_rate: f64,
    rsp_offset_sec: f64,
) -> Result<Vec<ScrCardiorespiratoryAssociation>> {
    if !eda_sampling_rate.is_finite() || eda_sampling_rate <= 0.0 {
        return Err(SignalError::InvalidSamplingRate(eda_sampling_rate));
    }
    if !ecg_sampling_rate.is_finite() || ecg_sampling_rate <= 0.0 {
        return Err(SignalError::InvalidSamplingRate(ecg_sampling_rate));
    }
    if !rsp_sampling_rate.is_finite() || rsp_sampling_rate <= 0.0 {
        return Err(SignalError::InvalidSamplingRate(rsp_sampling_rate));
    }
    if !eda_offset_sec.is_finite() || !ecg_offset_sec.is_finite() || !rsp_offset_sec.is_finite() {
        return Err(SignalError::NonFiniteInput);
    }

    let mut ecg_times = Vec::with_capacity(r_peaks.len());
    for &idx in r_peaks {
        let t = sample_to_time(idx, ecg_sampling_rate, ecg_offset_sec)?;
        ecg_times.push(t);
    }

    let mut associations = Vec::with_capacity(scr_events.len());

    for event in scr_events {
        let t_scr = sample_to_time(event.peak_index, eda_sampling_rate, eda_offset_sec)?;
        let phase = respiratory_phase_at_time(rsp_cycles, t_scr, rsp_sampling_rate, rsp_offset_sec);

        let (nearest_t_ecg, cardiac_delay) = if !ecg_times.is_empty() {
            let mut min_diff = f64::MAX;
            let mut best_t = 0.0;
            for &t_ecg in &ecg_times {
                let diff = (t_scr - t_ecg).abs();
                if diff < min_diff {
                    min_diff = diff;
                    best_t = t_ecg;
                }
            }
            (Some(best_t), Some(t_scr - best_t))
        } else {
            (None, None)
        };

        associations.push(ScrCardiorespiratoryAssociation {
            scr_peak_index: event.peak_index,
            scr_peak_time_sec: t_scr,
            scr_amplitude: event.amplitude,
            respiratory_phase_rad: phase,
            nearest_r_peak_time_sec: nearest_t_ecg,
            cardiac_delay_sec: cardiac_delay,
        });
    }

    Ok(associations)
}
