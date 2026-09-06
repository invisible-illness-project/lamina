use crate::error::{Result, SignalError};
use crate::rsp::clean::{RspCleaningConfig, rsp_clean_config};
use crate::signal::peaks::{PeakDetectionConfig, signal_findpeaks_config};
use ndarray::Array1;

/// Configuration for Respiration (RSP) peak, cycle, and rate processing.
#[derive(Debug, Clone, PartialEq)]
pub struct RspProcessingConfig {
    /// Bandpass lower cutoff frequency in Hz (default: 0.05 Hz).
    pub lowcut: Option<f64>,
    /// Bandpass upper cutoff frequency in Hz (default: 0.50 Hz).
    pub highcut: Option<f64>,
    /// Butterworth filter order (default: 3).
    pub filter_order: Option<usize>,
    /// Minimum breath cycle duration in seconds (default: 1.2 s, ~50 breaths/min max).
    pub min_breath_interval_sec: Option<f64>,
    /// Maximum breath cycle duration in seconds (default: 12.0 s, ~5 breaths/min min).
    pub max_breath_interval_sec: Option<f64>,
    /// Minimum peak-to-trough respiratory amplitude cutoff (default: 0.05 a.u.).
    pub min_amplitude: Option<f64>,
}

impl Default for RspProcessingConfig {
    fn default() -> Self {
        Self {
            lowcut: Some(0.05),
            highcut: Some(0.50),
            filter_order: Some(3),
            min_breath_interval_sec: Some(1.2),
            max_breath_interval_sec: Some(12.0),
            min_amplitude: Some(0.05),
        }
    }
}

impl RspProcessingConfig {
    /// Create a default RSP processing configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set bandpass lower cutoff frequency in Hz.
    pub fn with_lowcut(mut self, lowcut: f64) -> Self {
        self.lowcut = Some(lowcut);
        self
    }

    /// Set bandpass upper cutoff frequency in Hz.
    pub fn with_highcut(mut self, highcut: f64) -> Self {
        self.highcut = Some(highcut);
        self
    }

    /// Set Butterworth filter order.
    pub fn with_filter_order(mut self, order: usize) -> Self {
        self.filter_order = Some(order);
        self
    }

    /// Set minimum breath interval duration in seconds.
    pub fn with_min_breath_interval_sec(mut self, sec: f64) -> Self {
        self.min_breath_interval_sec = Some(sec);
        self
    }

    /// Set maximum breath interval duration in seconds.
    pub fn with_max_breath_interval_sec(mut self, sec: f64) -> Self {
        self.max_breath_interval_sec = Some(sec);
        self
    }

    /// Set minimum respiratory peak-to-trough amplitude.
    pub fn with_min_amplitude(mut self, amp: f64) -> Self {
        self.min_amplitude = Some(amp);
        self
    }

    /// Validate configuration parameters.
    pub fn validate(&self) -> Result<()> {
        if matches!(self.lowcut, Some(lc) if !lc.is_finite() || lc <= 0.0) {
            return Err(SignalError::InvalidCutoffFrequency(
                "Lowcut frequency must be positive and finite".to_string(),
            ));
        }
        if matches!(self.highcut, Some(hc) if !hc.is_finite() || hc <= 0.0) {
            return Err(SignalError::InvalidCutoffFrequency(
                "Highcut frequency must be positive and finite".to_string(),
            ));
        }
        if matches!((self.lowcut, self.highcut), (Some(lc), Some(hc)) if lc >= hc) {
            return Err(SignalError::InvalidCutoffFrequency(
                "Lowcut frequency must be strictly less than highcut frequency".to_string(),
            ));
        }
        if matches!(self.filter_order, Some(0)) {
            return Err(SignalError::InvalidFilterOrder(0));
        }
        if matches!(self.min_breath_interval_sec, Some(mi) if !mi.is_finite() || mi <= 0.0) {
            return Err(SignalError::InvalidWindowSize(0));
        }
        if matches!(self.max_breath_interval_sec, Some(ma) if !ma.is_finite() || ma <= 0.0) {
            return Err(SignalError::InvalidWindowSize(0));
        }
        if matches!((self.min_breath_interval_sec, self.max_breath_interval_sec), (Some(mi), Some(ma)) if mi >= ma)
        {
            return Err(SignalError::InvalidWindowSize(0));
        }
        if matches!(self.min_amplitude, Some(a) if !a.is_finite() || a < 0.0) {
            return Err(SignalError::NonFiniteInput);
        }
        Ok(())
    }
}

/// Structured representation of a physiological Respiration Cycle.
#[derive(Debug, Clone, PartialEq)]
pub struct RespirationCycle {
    /// 0-indexed sample index of starting inspiratory peak (maximum expansion).
    pub inspiration_index: usize,
    /// 0-indexed sample index of intervening expiratory trough (maximum contraction).
    pub expiration_index: usize,
    /// 0-indexed sample index of next inspiratory peak (ending maximum expansion).
    pub next_inspiration_index: usize,
    /// Breath cycle duration in seconds ($T = (i_{k+1} - i_k) / F_s$).
    pub duration_sec: f64,
    /// Instantaneous respiratory rate in breaths per minute (BPM = $60.0 / T$).
    pub respiratory_rate_bpm: f64,
    /// Average peak-to-trough breath amplitude.
    pub amplitude: f64,
}

/// Extract validated [`RespirationCycle`] events from a raw or cleaned RSP signal given a [`RspProcessingConfig`].
///
/// # Scientific Contract
/// - **Inputs**:
///   - `signal`: 1D respiratory signal array.
///   - `sampling_rate`: Sampling frequency $F_s$ in Hertz (Hz). Must be $> 0.0$.
///   - `config`: [`RspProcessingConfig`] parameters.
/// - **Output**: `Vec<RespirationCycle>` containing validated breath cycles in chronological order.
///
/// # Errors
/// Returns [`SignalError`] if `signal` is empty, non-finite, if parameters are invalid, or if signal length is insufficient.
pub fn rsp_cycles_config(
    signal: &Array1<f64>,
    sampling_rate: f64,
    config: &RspProcessingConfig,
) -> Result<Vec<RespirationCycle>> {
    let n = signal.len();
    if n == 0 {
        return Err(SignalError::EmptySignal);
    }
    if !sampling_rate.is_finite() || sampling_rate <= 0.0 {
        return Err(SignalError::InvalidSamplingRate(sampling_rate));
    }
    for &val in signal.iter() {
        if !val.is_finite() {
            return Err(SignalError::NonFiniteInput);
        }
    }
    config.validate()?;

    let lowcut = config.lowcut.unwrap_or(0.05);
    let highcut = config.highcut.unwrap_or(0.50);
    let filter_order = config.filter_order.unwrap_or(3);
    let min_interval_sec = config.min_breath_interval_sec.unwrap_or(1.2);
    let max_interval_sec = config.max_breath_interval_sec.unwrap_or(12.0);
    let min_amp = config.min_amplitude.unwrap_or(0.05);

    let clean_cfg = RspCleaningConfig::new()
        .with_lowcut(lowcut)
        .with_highcut(highcut)
        .with_filter_order(filter_order);
    let cleaned = rsp_clean_config(signal, sampling_rate, &clean_cfg)?;

    // Noise floor protection for flat / zero / constant signals
    let max_val = cleaned.fold(f64::NEG_INFINITY, |acc, &x| acc.max(x));
    let min_val = cleaned.fold(f64::INFINITY, |acc, &x| acc.min(x));
    let p2p_amp = max_val - min_val;
    if p2p_amp < 1e-12 || p2p_amp < min_amp {
        return Ok(Vec::new());
    }

    let min_dist_samples = (min_interval_sec * sampling_rate).round().max(1.0) as usize;
    let peak_cfg = PeakDetectionConfig::new().with_min_distance(min_dist_samples);
    let candidate_peaks = signal_findpeaks_config(&cleaned, &peak_cfg)?;

    if candidate_peaks.len() < 2 {
        return Ok(Vec::new());
    }

    let mut cycles = Vec::new();

    for k in 0..(candidate_peaks.len() - 1) {
        let i_k = candidate_peaks[k];
        let i_k1 = candidate_peaks[k + 1];

        if i_k >= i_k1 || i_k1 >= n {
            continue;
        }

        // Find expiratory trough (minimum amplitude between i_k and i_k1)
        let mut min_trough_idx = i_k + 1;
        let mut min_trough_val = cleaned[min_trough_idx];

        for j in (i_k + 1)..i_k1 {
            if cleaned[j] < min_trough_val {
                min_trough_val = cleaned[j];
                min_trough_idx = j;
            }
        }

        let duration_sec = (i_k1 - i_k) as f64 / sampling_rate;
        let bpm = 60.0 / duration_sec;
        let amplitude = 0.5 * ((cleaned[i_k] - min_trough_val) + (cleaned[i_k1] - min_trough_val));

        if duration_sec >= min_interval_sec
            && duration_sec <= max_interval_sec
            && amplitude >= min_amp
        {
            cycles.push(RespirationCycle {
                inspiration_index: i_k,
                expiration_index: min_trough_idx,
                next_inspiration_index: i_k1,
                duration_sec,
                respiratory_rate_bpm: bpm,
                amplitude,
            });
        }
    }

    Ok(cycles)
}

/// Extract validated [`RespirationCycle`] events from an RSP signal using default configuration.
pub fn rsp_cycles(signal: &Array1<f64>, sampling_rate: f64) -> Result<Vec<RespirationCycle>> {
    let config = RspProcessingConfig::default();
    rsp_cycles_config(signal, sampling_rate, &config)
}

/// Compute instantaneous respiratory rate series array (BPM) aligned with signal samples.
///
/// # Scientific Contract
/// - **Inputs**:
///   - `signal`: 1D respiratory signal array.
///   - `sampling_rate`: Sampling frequency $F_s$ in Hertz (Hz). Must be $> 0.0$.
///   - `config`: [`RspProcessingConfig`] parameters.
/// - **Output**: 1D `Array1<f64>` of length $N$ containing instantaneous respiratory rate estimates in breaths per minute (BPM).
/// - **Interpolation**: Step-wise constant boundaries with linear interpolation across breath cycles.
///
/// # Errors
/// Returns [`SignalError`] if `signal` is empty, non-finite, or if `sampling_rate` is invalid.
pub fn rsp_rate_config(
    signal: &Array1<f64>,
    sampling_rate: f64,
    config: &RspProcessingConfig,
) -> Result<Array1<f64>> {
    let n = signal.len();
    if n == 0 {
        return Err(SignalError::EmptySignal);
    }
    let cycles = rsp_cycles_config(signal, sampling_rate, config)?;
    let mut rate_series = Array1::<f64>::zeros(n);

    if cycles.is_empty() {
        return Ok(rate_series);
    }

    let first_idx = cycles[0].inspiration_index;
    let first_rate = cycles[0].respiratory_rate_bpm;

    // Fill leading boundary before first cycle
    for i in 0..=first_idx.min(n - 1) {
        rate_series[i] = first_rate;
    }

    // Interpolate across cycles
    for k in 0..(cycles.len() - 1) {
        let idx_start = cycles[k].inspiration_index;
        let idx_end = cycles[k + 1].inspiration_index;
        let rate_start = cycles[k].respiratory_rate_bpm;
        let rate_end = cycles[k + 1].respiratory_rate_bpm;

        let len = (idx_end - idx_start) as f64;
        for i in idx_start..=idx_end.min(n - 1) {
            let alpha = (i - idx_start) as f64 / len.max(1.0);
            rate_series[i] = (1.0 - alpha) * rate_start + alpha * rate_end;
        }
    }

    // Fill trailing boundary after last cycle
    let last_idx = cycles.last().unwrap().next_inspiration_index.min(n - 1);
    let last_rate = cycles.last().unwrap().respiratory_rate_bpm;
    for i in last_idx..n {
        rate_series[i] = last_rate;
    }

    Ok(rate_series)
}

/// Compute instantaneous respiratory rate series array (BPM) using default configuration.
pub fn rsp_rate(signal: &Array1<f64>, sampling_rate: f64) -> Result<Array1<f64>> {
    let config = RspProcessingConfig::default();
    rsp_rate_config(signal, sampling_rate, &config)
}

/// Locate inspiratory peak indices in a cleaned RSP signal given a [`RspProcessingConfig`].
pub fn rsp_findpeaks_config(
    cleaned_signal: &Array1<f64>,
    sampling_rate: f64,
    config: &RspProcessingConfig,
) -> Result<Vec<usize>> {
    let cycles = rsp_cycles_config(cleaned_signal, sampling_rate, config)?;
    let mut peaks: Vec<usize> = cycles.into_iter().map(|c| c.inspiration_index).collect();
    peaks.sort_unstable();
    peaks.dedup();
    Ok(peaks)
}

/// Locate inspiratory peak mask (`Array1<bool>`) in a cleaned RSP signal given a [`RspProcessingConfig`].
pub fn rsp_findpeaks_mask(
    cleaned_signal: &Array1<f64>,
    sampling_rate: f64,
    config: &RspProcessingConfig,
) -> Result<Array1<bool>> {
    let indices = rsp_findpeaks_config(cleaned_signal, sampling_rate, config)?;
    let mut mask = Array1::<bool>::from_elem(cleaned_signal.len(), false);
    for idx in indices {
        mask[idx] = true;
    }
    Ok(mask)
}

/// Locate inspiratory peak mask (`Array1<bool>`) in a cleaned RSP signal assuming default 100 Hz sampling rate.
///
/// Convenience entry point maintaining backward compatibility.
pub fn rsp_findpeaks(cleaned_signal: &Array1<f64>) -> Result<Array1<bool>> {
    let config = RspProcessingConfig::default();
    rsp_findpeaks_mask(cleaned_signal, 100.0, &config)
}
