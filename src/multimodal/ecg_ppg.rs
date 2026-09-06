use crate::error::{Result, SignalError};
use crate::multimodal::config::PulseTimingConfig;
use crate::multimodal::sync::sample_to_time;

/// A paired ECG R-peak and PPG systolic pulse wave timing match.
#[derive(Debug, Clone, PartialEq)]
pub struct PulseTimingResult {
    /// ECG R-peak sample index
    pub ecg_peak_index: usize,
    /// PPG systolic peak sample index
    pub ppg_peak_index: usize,
    /// ECG R-peak timestamp in seconds
    pub ecg_timestamp_sec: f64,
    /// PPG systolic peak timestamp in seconds
    pub ppg_timestamp_sec: f64,
    /// Pulse delay interval $t_{\text{ppg}} - t_{\text{ecg}}$ in seconds
    pub pulse_delay_sec: f64,
}

/// Compute ECG-to-PPG pulse delay timing with custom configuration.
///
/// # Operational Semantics
/// Executes deterministic one-to-one matching between ECG R-peak timestamps and PPG systolic pulse timestamps:
/// - Each ECG R-peak is paired with at most one following PPG pulse within delay range $[t_{\text{min}}, t_{\text{max}}]$.
/// - Each PPG pulse is paired with at most one preceding ECG R-peak (no double-counting).
/// - Supports independent sampling rates and recording start offsets between modalities.
///
/// # Errors
/// Returns [`SignalError::InvalidSamplingRate`] if sampling rates are $\le 0.0$ or non-finite.
/// Returns [`SignalError::InvalidCutoffFrequency`] if `min_delay_sec` $\ge$ `max_delay_sec`.
pub fn ecg_ppg_timing_config(
    ecg_peaks: &[usize],
    ecg_sampling_rate: f64,
    ecg_offset_sec: f64,
    ppg_peaks: &[usize],
    ppg_sampling_rate: f64,
    ppg_offset_sec: f64,
    config: &PulseTimingConfig,
) -> Result<Vec<PulseTimingResult>> {
    if !ecg_sampling_rate.is_finite() || ecg_sampling_rate <= 0.0 {
        return Err(SignalError::InvalidSamplingRate(ecg_sampling_rate));
    }
    if !ppg_sampling_rate.is_finite() || ppg_sampling_rate <= 0.0 {
        return Err(SignalError::InvalidSamplingRate(ppg_sampling_rate));
    }
    if !ecg_offset_sec.is_finite() || !ppg_offset_sec.is_finite() {
        return Err(SignalError::NonFiniteInput);
    }

    let min_delay = config.min_delay_sec.unwrap_or(0.10);
    let max_delay = config.max_delay_sec.unwrap_or(0.60);

    if !min_delay.is_finite() || !max_delay.is_finite() || min_delay >= max_delay || min_delay < 0.0
    {
        return Err(SignalError::InvalidCutoffFrequency(format!(
            "Invalid pulse delay window: min = {}, max = {}",
            min_delay, max_delay
        )));
    }

    if ecg_peaks.is_empty() || ppg_peaks.is_empty() {
        return Ok(Vec::new());
    }

    let mut ecg_times: Vec<(usize, f64)> = Vec::with_capacity(ecg_peaks.len());
    for &idx in ecg_peaks {
        let t = sample_to_time(idx, ecg_sampling_rate, ecg_offset_sec)?;
        ecg_times.push((idx, t));
    }
    ecg_times.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    let mut ppg_times: Vec<(usize, f64)> = Vec::with_capacity(ppg_peaks.len());
    for &idx in ppg_peaks {
        let t = sample_to_time(idx, ppg_sampling_rate, ppg_offset_sec)?;
        ppg_times.push((idx, t));
    }
    ppg_times.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    let mut results = Vec::new();
    let mut ppg_ptr = 0;

    for (ecg_idx, t_ecg) in ecg_times {
        while ppg_ptr < ppg_times.len() && (ppg_times[ppg_ptr].1 - t_ecg) < min_delay {
            ppg_ptr += 1;
        }

        if ppg_ptr < ppg_times.len() {
            let delay = ppg_times[ppg_ptr].1 - t_ecg;
            if delay >= min_delay && delay <= max_delay {
                results.push(PulseTimingResult {
                    ecg_peak_index: ecg_idx,
                    ppg_peak_index: ppg_times[ppg_ptr].0,
                    ecg_timestamp_sec: t_ecg,
                    ppg_timestamp_sec: ppg_times[ppg_ptr].1,
                    pulse_delay_sec: delay,
                });
                ppg_ptr += 1; // Consume PPG pulse to ensure strictly 1-to-1 matching
            }
        }
    }

    Ok(results)
}

/// Compute ECG-to-PPG pulse delay timing with default configuration.
pub fn ecg_ppg_timing(
    ecg_peaks: &[usize],
    ecg_sampling_rate: f64,
    ecg_offset_sec: f64,
    ppg_peaks: &[usize],
    ppg_sampling_rate: f64,
    ppg_offset_sec: f64,
) -> Result<Vec<PulseTimingResult>> {
    ecg_ppg_timing_config(
        ecg_peaks,
        ecg_sampling_rate,
        ecg_offset_sec,
        ppg_peaks,
        ppg_sampling_rate,
        ppg_offset_sec,
        &PulseTimingConfig::default(),
    )
}
