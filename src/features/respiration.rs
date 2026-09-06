use crate::error::{Result, SignalError};
use crate::features::window::FeatureWindow;
use crate::multimodal::sync::sample_to_time;
use crate::rsp::RespirationCycle;

/// Respiration physiological features extracted over a time window.
#[derive(Debug, Clone, PartialEq)]
pub struct RespirationFeatures {
    /// Mean respiratory rate in beats/breaths per minute (BPM)
    pub mean_rate_bpm: Option<f64>,
    /// Median respiratory rate in beats/breaths per minute (BPM)
    pub median_rate_bpm: Option<f64>,
    /// Standard deviation of respiratory rate in BPM
    pub rate_std_bpm: Option<f64>,
    /// Mean duration of individual breath cycles in seconds (seconds)
    pub mean_cycle_duration_sec: Option<f64>,
    /// Count of valid respiration cycles intersecting the feature window
    pub cycle_count: usize,
    /// Mean amplitude of breath cycles (peak-to-trough height)
    pub mean_amplitude: Option<f64>,
    /// Standard deviation of breath cycle amplitudes
    pub amplitude_std: Option<f64>,
}

/// Extract respiration features from `RespirationCycle` events intersecting a feature window.
///
/// # Errors
/// Returns [`SignalError::InvalidSamplingRate`] if `sampling_rate` is $\le 0.0$ or non-finite.
/// Returns [`SignalError::NonFiniteInput`] if `offset_sec` is non-finite.
pub fn respiration_features(
    cycles: &[RespirationCycle],
    sampling_rate: f64,
    offset_sec: f64,
    window: &FeatureWindow,
) -> Result<RespirationFeatures> {
    if !sampling_rate.is_finite() || sampling_rate <= 0.0 {
        return Err(SignalError::InvalidSamplingRate(sampling_rate));
    }
    if !offset_sec.is_finite() {
        return Err(SignalError::NonFiniteInput);
    }

    let mut intersecting_cycles = Vec::new();
    for cycle in cycles {
        let t_insp = sample_to_time(cycle.inspiration_index, sampling_rate, offset_sec)?;
        if t_insp >= window.start_time_sec && t_insp < window.end_time_sec {
            intersecting_cycles.push(cycle);
        }
    }

    let cycle_count = intersecting_cycles.len();

    if cycle_count == 0 {
        return Ok(RespirationFeatures {
            mean_rate_bpm: None,
            median_rate_bpm: None,
            rate_std_bpm: None,
            mean_cycle_duration_sec: None,
            cycle_count: 0,
            mean_amplitude: None,
            amplitude_std: None,
        });
    }

    let rates: Vec<f64> = intersecting_cycles
        .iter()
        .map(|c| c.respiratory_rate_bpm)
        .collect();
    let durations: Vec<f64> = intersecting_cycles.iter().map(|c| c.duration_sec).collect();
    let amps: Vec<f64> = intersecting_cycles.iter().map(|c| c.amplitude).collect();

    let mean_rate = rates.iter().sum::<f64>() / rates.len() as f64;
    let mean_duration = durations.iter().sum::<f64>() / durations.len() as f64;
    let mean_amp = amps.iter().sum::<f64>() / amps.len() as f64;

    let rate_var = rates.iter().map(|&x| (x - mean_rate).powi(2)).sum::<f64>() / rates.len() as f64;
    let amp_var = amps.iter().map(|&x| (x - mean_amp).powi(2)).sum::<f64>() / amps.len() as f64;

    let mut sorted_rates = rates;
    sorted_rates.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median_rate = if sorted_rates.len() % 2 == 1 {
        sorted_rates[sorted_rates.len() / 2]
    } else {
        (sorted_rates[sorted_rates.len() / 2 - 1] + sorted_rates[sorted_rates.len() / 2]) / 2.0
    };

    Ok(RespirationFeatures {
        mean_rate_bpm: Some(mean_rate),
        median_rate_bpm: Some(median_rate),
        rate_std_bpm: Some(rate_var.sqrt()),
        mean_cycle_duration_sec: Some(mean_duration),
        cycle_count,
        mean_amplitude: Some(mean_amp),
        amplitude_std: Some(amp_var.sqrt()),
    })
}
