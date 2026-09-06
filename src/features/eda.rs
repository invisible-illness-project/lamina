use crate::eda::ScrEvent;
use crate::error::{Result, SignalError};
use crate::features::window::{FeatureWindow, time_range_to_sample_range};
use crate::multimodal::sync::sample_to_time;
use ndarray::Array1;

/// Electrodermal activity (EDA) physiological features extracted over a time window.
#[derive(Debug, Clone, PartialEq)]
pub struct EdaFeatures {
    /// Mean Tonic Skin Conductance Level (SCL) in microsiemens ($\mu\text{S}$)
    pub mean_tonic_us: Option<f64>,
    /// Median Tonic Skin Conductance Level (SCL) in microsiemens ($\mu\text{S}$)
    pub median_tonic_us: Option<f64>,
    /// Standard deviation of Tonic SCL in microsiemens ($\mu\text{S}$)
    pub tonic_std_us: Option<f64>,
    /// Mean Phasic Skin Conductance Response (SCR) in microsiemens ($\mu\text{S}$)
    pub mean_phasic_us: Option<f64>,
    /// Standard deviation of Phasic SCR in microsiemens ($\mu\text{S}$)
    pub phasic_std_us: Option<f64>,
    /// Count of SCR events occurring within the window
    pub scr_count: usize,
    /// Rate of SCR events normalized to events per minute ($\text{events/min}$)
    pub scr_rate_per_min: Option<f64>,
    /// Mean amplitude of SCR events in microsiemens ($\mu\text{S}$)
    pub mean_scr_amplitude_us: Option<f64>,
    /// Median amplitude of SCR events in microsiemens ($\mu\text{S}$)
    pub median_scr_amplitude_us: Option<f64>,
    /// Mean rise time of SCR events in seconds (seconds)
    pub mean_scr_rise_time_sec: Option<f64>,
}

/// Extract EDA features (tonic SCL, phasic SCR, and SCR events) over a feature window.
///
/// # Errors
/// Returns [`SignalError::DimensionMismatch`] if `tonic` and `phasic` signal lengths differ.
/// Returns [`SignalError::InvalidSamplingRate`] if `sampling_rate` is $\le 0.0$ or non-finite.
/// Returns [`SignalError::NonFiniteInput`] if `offset_sec` is non-finite.
pub fn eda_features(
    tonic: &Array1<f64>,
    phasic: &Array1<f64>,
    scr_events: &[ScrEvent],
    sampling_rate: f64,
    offset_sec: f64,
    window: &FeatureWindow,
) -> Result<EdaFeatures> {
    if tonic.len() != phasic.len() {
        return Err(SignalError::DimensionMismatch);
    }
    if !sampling_rate.is_finite() || sampling_rate <= 0.0 {
        return Err(SignalError::InvalidSamplingRate(sampling_rate));
    }
    if !offset_sec.is_finite() {
        return Err(SignalError::NonFiniteInput);
    }

    let sample_range = time_range_to_sample_range(
        window.start_time_sec,
        window.end_time_sec,
        sampling_rate,
        offset_sec,
        tonic.len(),
    )?;

    let (mean_tonic, median_tonic, tonic_std, mean_phasic, phasic_std) =
        if let Some((start_idx, end_idx)) = sample_range {
            let t_slice = tonic.slice(ndarray::s![start_idx..end_idx]);
            let p_slice = phasic.slice(ndarray::s![start_idx..end_idx]);

            let m_t = t_slice.mean().unwrap_or(0.0);
            let m_p = p_slice.mean().unwrap_or(0.0);

            let t_var =
                t_slice.iter().map(|&x| (x - m_t).powi(2)).sum::<f64>() / t_slice.len() as f64;
            let p_var =
                p_slice.iter().map(|&x| (x - m_p).powi(2)).sum::<f64>() / p_slice.len() as f64;

            let mut sorted_t: Vec<f64> = t_slice.to_vec();
            sorted_t.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let med_t = if sorted_t.is_empty() {
                m_t
            } else if sorted_t.len() % 2 == 1 {
                sorted_t[sorted_t.len() / 2]
            } else {
                (sorted_t[sorted_t.len() / 2 - 1] + sorted_t[sorted_t.len() / 2]) / 2.0
            };

            (
                Some(m_t),
                Some(med_t),
                Some(t_var.sqrt()),
                Some(m_p),
                Some(p_var.sqrt()),
            )
        } else {
            (None, None, None, None, None)
        };

    // SCR events within window
    let mut window_scrs = Vec::new();
    for event in scr_events {
        let t_scr = sample_to_time(event.peak_index, sampling_rate, offset_sec)?;
        if t_scr >= window.start_time_sec && t_scr < window.end_time_sec {
            window_scrs.push(event);
        }
    }

    let scr_count = window_scrs.len();
    let scr_rate_per_min = if window.duration_sec > 0.0 {
        Some((scr_count as f64 / window.duration_sec) * 60.0)
    } else {
        None
    };

    let (mean_amp, median_amp, mean_rise) = if scr_count > 0 {
        let amps: Vec<f64> = window_scrs.iter().map(|e| e.amplitude).collect();
        let rises: Vec<f64> = window_scrs.iter().map(|e| e.rise_time_sec).collect();

        let m_amp = amps.iter().sum::<f64>() / amps.len() as f64;
        let m_rise = rises.iter().sum::<f64>() / rises.len() as f64;

        let mut sorted_amps = amps;
        sorted_amps.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let med_amp = if sorted_amps.len() % 2 == 1 {
            sorted_amps[sorted_amps.len() / 2]
        } else {
            (sorted_amps[sorted_amps.len() / 2 - 1] + sorted_amps[sorted_amps.len() / 2]) / 2.0
        };

        (Some(m_amp), Some(med_amp), Some(m_rise))
    } else {
        (None, None, None)
    };

    Ok(EdaFeatures {
        mean_tonic_us: mean_tonic,
        median_tonic_us: median_tonic,
        tonic_std_us: tonic_std,
        mean_phasic_us: mean_phasic,
        phasic_std_us: phasic_std,
        scr_count,
        scr_rate_per_min,
        mean_scr_amplitude_us: mean_amp,
        median_scr_amplitude_us: median_amp,
        mean_scr_rise_time_sec: mean_rise,
    })
}
