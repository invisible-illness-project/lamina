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

/// Precomputed multimodal coupling observations across a full recording timeline.
#[derive(Debug, Clone, Default)]
pub struct PrecomputedCoupling {
    /// ECG-PPG pulse delay matches: `(r_peak_time_sec, pulse_delay_sec)`
    pub pulse_delays: Vec<(f64, f64)>,
    /// Cardiorespiratory phase events: `(r_peak_time_sec, respiratory_phase_rad)`
    pub cr_phases: Vec<(f64, f64)>,
    /// EDA cardiorespiratory association timestamps: `scr_peak_time_sec`
    pub eda_assocs: Vec<f64>,
}

impl PrecomputedCoupling {
    /// Compute multimodal coupling observations once across the entire recording.
    #[allow(clippy::too_many_arguments)]
    pub fn compute(
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
    ) -> Result<Self> {
        let pulse_delays = if let (Some(r), Some(p)) = (r_peaks, ppg_peaks) {
            if !r.is_empty() && !p.is_empty() && ecg_fs > 0.0 && ppg_fs > 0.0 {
                if let Ok(matches) = ecg_ppg_timing(r, ecg_fs, ecg_off, p, ppg_fs, ppg_off) {
                    matches
                        .into_iter()
                        .map(|m| (m.ecg_timestamp_sec, m.pulse_delay_sec))
                        .collect()
                } else {
                    Vec::new()
                }
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        let cr_phases = if let (Some(r), Some(c)) = (r_peaks, rsp_cycles) {
            if !r.is_empty() && !c.is_empty() && ecg_fs > 0.0 && rsp_fs > 0.0 {
                if let Ok(cr_events) =
                    cardiac_respiratory_phase(r, ecg_fs, ecg_off, c, rsp_fs, rsp_off)
                {
                    cr_events
                        .into_iter()
                        .map(|e| (e.timestamp_sec, e.respiratory_phase))
                        .collect()
                } else {
                    Vec::new()
                }
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        let eda_assocs = if let (Some(scrs), Some(r), Some(c)) = (scr_events, r_peaks, rsp_cycles) {
            if !scrs.is_empty()
                && !r.is_empty()
                && !c.is_empty()
                && eda_fs > 0.0
                && ecg_fs > 0.0
                && rsp_fs > 0.0
            {
                if let Ok(assocs) = eda_cardiorespiratory_association(
                    scrs, eda_fs, eda_off, r, ecg_fs, ecg_off, c, rsp_fs, rsp_off,
                ) {
                    assocs.into_iter().map(|a| a.scr_peak_time_sec).collect()
                } else {
                    Vec::new()
                }
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        Ok(Self {
            pulse_delays,
            cr_phases,
            eda_assocs,
        })
    }

    /// Extract aggregated window coupling features for a specific window.
    #[allow(clippy::too_many_arguments)]
    pub fn extract_for_window(
        &self,
        r_peaks: Option<&[usize]>,
        ecg_fs: f64,
        ecg_off: f64,
        rsp_cycles: Option<&[RespirationCycle]>,
        rsp_fs: f64,
        rsp_off: f64,
        window: &FeatureWindow,
    ) -> Result<CouplingFeatures> {
        // 1. RSA
        let (rsa_bpm, rsa_rr) = if let (Some(r), Some(c)) = (r_peaks, rsp_cycles) {
            let win_r: Vec<usize> = r
                .iter()
                .copied()
                .filter(|&idx| {
                    if let Ok(t) = sample_to_time(idx, ecg_fs, ecg_off) {
                        t >= window.start_time_sec && t < window.end_time_sec
                    } else {
                        false
                    }
                })
                .collect();
            let win_c: Vec<RespirationCycle> = c
                .iter()
                .filter(|cyc| {
                    if let Ok(t) = sample_to_time(cyc.inspiration_index, rsp_fs, rsp_off) {
                        t >= window.start_time_sec && t < window.end_time_sec
                    } else {
                        false
                    }
                })
                .cloned()
                .collect();

            if !win_r.is_empty() && !win_c.is_empty() {
                if let Ok(res) = rsa(&win_r, ecg_fs, ecg_off, &win_c, rsp_fs, rsp_off) {
                    (Some(res.amplitude_bpm), Some(res.amplitude_rr_sec))
                } else {
                    (None, None)
                }
            } else {
                (None, None)
            }
        } else {
            (None, None)
        };

        // 2. Cardiorespiratory Phase Coupling
        let win_phases: Vec<f64> = self
            .cr_phases
            .iter()
            .filter(|(t, _)| *t >= window.start_time_sec && *t < window.end_time_sec)
            .map(|(_, p)| *p)
            .collect();

        let (conc, mean_p) = if !win_phases.is_empty() {
            if let Ok(coupling_res) = cardiorespiratory_phase_coupling(&win_phases) {
                (
                    Some(coupling_res.concentration),
                    Some(coupling_res.mean_phase),
                )
            } else {
                (None, None)
            }
        } else {
            (None, None)
        };

        // 3. ECG-PPG Pulse Delay
        let win_delays: Vec<f64> = self
            .pulse_delays
            .iter()
            .filter(|(t, _)| *t >= window.start_time_sec && *t < window.end_time_sec)
            .map(|(_, d)| *d)
            .collect();

        let (mean_delay, delay_std) = if !win_delays.is_empty() {
            let m_delay = win_delays.iter().sum::<f64>() / win_delays.len() as f64;
            let var = win_delays
                .iter()
                .map(|&x| (x - m_delay).powi(2))
                .sum::<f64>()
                / win_delays.len() as f64;
            (Some(m_delay), Some(var.sqrt()))
        } else {
            (None, None)
        };

        // 4. EDA Cardiorespiratory Associations
        let scr_assoc_count = self
            .eda_assocs
            .iter()
            .filter(|&t| *t >= window.start_time_sec && *t < window.end_time_sec)
            .count();

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
    let precomputed = PrecomputedCoupling::compute(
        r_peaks, ecg_fs, ecg_off, ppg_peaks, ppg_fs, ppg_off, rsp_cycles, rsp_fs, rsp_off,
        scr_events, eda_fs, eda_off,
    )?;

    precomputed.extract_for_window(
        r_peaks, ecg_fs, ecg_off, rsp_cycles, rsp_fs, rsp_off, window,
    )
}
