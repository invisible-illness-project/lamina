use crate::eda::ScrEvent;
use crate::error::Result;
use crate::features::window::FeatureWindow;
use crate::multimodal::coupling::cardiorespiratory_phase_coupling;
use crate::multimodal::ecg_ppg::ecg_ppg_timing;
use crate::multimodal::eda_assoc::eda_cardiorespiratory_association;
use crate::multimodal::rsa::{cardiac_respiratory_phase, rsa};
use crate::multimodal::sync::sample_to_time;
use crate::rsp::RespirationCycle;

/// Multimodal physiological coupling features extracted over a time window.
#[derive(Debug, Clone, PartialEq)]
pub struct CouplingFeatures {
    /// Mean RSA amplitude in heart rate modulation (BPM)
    pub rsa_amplitude_bpm: Option<f64>,
    /// Mean RSA amplitude in inter-beat interval modulation (seconds)
    pub rsa_amplitude_rr_sec: Option<f64>,
    /// Cardiorespiratory phase coupling concentration ($R \in [0.0, 1.0]$)
    pub cardiac_respiratory_concentration: Option<f64>,
    /// Cardiorespiratory circular mean phase in radians ($\bar{\phi} \in [0, 2\pi)$)
    pub cardiac_respiratory_mean_phase: Option<f64>,
    /// Mean ECG-to-PPG pulse delay in seconds
    pub mean_pulse_delay_sec: Option<f64>,
    /// Standard deviation of ECG-to-PPG pulse delay in seconds
    pub pulse_delay_std_sec: Option<f64>,
    /// Count of SCR events associated with cardiac/respiratory state within the window
    pub scr_cardiac_association_count: usize,
}

/// Extract multimodal coupling features across available physiological modalities for a feature window.
#[allow(clippy::too_many_arguments)]
pub fn coupling_features(
    r_peaks: Option<&[usize]>,
    ecg_fs: f64,
    ecg_off: f64,
    ppg_peaks: Option<&[usize]>,
    ppg_fs: f64,
    ppg_off: f64,
    rsp_cycles: Option<&[RespirationCycle]>,
    rsp_fs: f64,
    rsp_off: f64,
    scr_events: Option<&[ScrEvent]>,
    eda_fs: f64,
    eda_off: f64,
    window: &FeatureWindow,
) -> Result<CouplingFeatures> {
    // Intersect events with window
    let win_r_peaks: Vec<usize> = if let Some(peaks) = r_peaks {
        peaks
            .iter()
            .copied()
            .filter(|&idx| {
                if let Ok(t) = sample_to_time(idx, ecg_fs, ecg_off) {
                    t >= window.start_time_sec && t < window.end_time_sec
                } else {
                    false
                }
            })
            .collect()
    } else {
        Vec::new()
    };

    let win_ppg_peaks: Vec<usize> = if let Some(peaks) = ppg_peaks {
        peaks
            .iter()
            .copied()
            .filter(|&idx| {
                if let Ok(t) = sample_to_time(idx, ppg_fs, ppg_off) {
                    t >= window.start_time_sec && t < window.end_time_sec
                } else {
                    false
                }
            })
            .collect()
    } else {
        Vec::new()
    };

    let win_rsp_cycles: Vec<RespirationCycle> = if let Some(cycles) = rsp_cycles {
        cycles
            .iter()
            .filter(|c| {
                if let Ok(t) = sample_to_time(c.inspiration_index, rsp_fs, rsp_off) {
                    t >= window.start_time_sec && t < window.end_time_sec
                } else {
                    false
                }
            })
            .cloned()
            .collect()
    } else {
        Vec::new()
    };

    let win_scr_events: Vec<ScrEvent> = if let Some(events) = scr_events {
        events
            .iter()
            .filter(|e| {
                if let Ok(t) = sample_to_time(e.peak_index, eda_fs, eda_off) {
                    t >= window.start_time_sec && t < window.end_time_sec
                } else {
                    false
                }
            })
            .cloned()
            .collect()
    } else {
        Vec::new()
    };

    // 1. RSA Features
    let (rsa_bpm, rsa_rr) = if !win_r_peaks.is_empty() && !win_rsp_cycles.is_empty() {
        if let Ok(res) = rsa(
            &win_r_peaks,
            ecg_fs,
            ecg_off,
            &win_rsp_cycles,
            rsp_fs,
            rsp_off,
        ) {
            (Some(res.amplitude_bpm), Some(res.amplitude_rr_sec))
        } else {
            (None, None)
        }
    } else {
        (None, None)
    };

    // 2. Cardiorespiratory Phase Coupling
    let (conc, mean_p) = if !win_r_peaks.is_empty() && !win_rsp_cycles.is_empty() {
        if let Ok(cr_events) = cardiac_respiratory_phase(
            &win_r_peaks,
            ecg_fs,
            ecg_off,
            &win_rsp_cycles,
            rsp_fs,
            rsp_off,
        ) {
            let phases: Vec<f64> = cr_events.iter().map(|e| e.respiratory_phase).collect();
            if let Ok(coupling_res) = cardiorespiratory_phase_coupling(&phases) {
                (
                    Some(coupling_res.concentration),
                    Some(coupling_res.mean_phase),
                )
            } else {
                (None, None)
            }
        } else {
            (None, None)
        }
    } else {
        (None, None)
    };

    // 3. ECG-PPG Pulse Delay
    let (mean_delay, delay_std) = if !win_r_peaks.is_empty() && !win_ppg_peaks.is_empty() {
        if let Ok(matches) = ecg_ppg_timing(
            &win_r_peaks,
            ecg_fs,
            ecg_off,
            &win_ppg_peaks,
            ppg_fs,
            ppg_off,
        ) {
            if !matches.is_empty() {
                let delays: Vec<f64> = matches.iter().map(|m| m.pulse_delay_sec).collect();
                let m_delay = delays.iter().sum::<f64>() / delays.len() as f64;
                let var = delays.iter().map(|&x| (x - m_delay).powi(2)).sum::<f64>()
                    / delays.len() as f64;
                (Some(m_delay), Some(var.sqrt()))
            } else {
                (None, None)
            }
        } else {
            (None, None)
        }
    } else {
        (None, None)
    };

    // 4. EDA Cardiorespiratory Associations
    let scr_assoc_count = if !win_scr_events.is_empty() {
        if let Ok(assocs) = eda_cardiorespiratory_association(
            &win_scr_events,
            eda_fs,
            eda_off,
            &win_r_peaks,
            ecg_fs,
            ecg_off,
            &win_rsp_cycles,
            rsp_fs,
            rsp_off,
        ) {
            assocs.len()
        } else {
            0
        }
    } else {
        0
    };

    Ok(CouplingFeatures {
        rsa_amplitude_bpm: rsa_bpm,
        rsa_amplitude_rr_sec: rsa_rr,
        cardiac_respiratory_concentration: conc,
        cardiac_respiratory_mean_phase: mean_p,
        mean_pulse_delay_sec: mean_delay,
        pulse_delay_std_sec: delay_std,
        scr_cardiac_association_count: scr_assoc_count,
    })
}
