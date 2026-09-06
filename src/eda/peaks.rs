use crate::error::{Result, SignalError};
use crate::signal::peaks::{PeakDetectionConfig, signal_findpeaks_config};
use ndarray::Array1;

/// Configuration for Skin Conductance Response (SCR) peak and event detection.
#[derive(Debug, Clone, PartialEq)]
pub struct EdaPeakDetectionConfig {
    /// Minimum SCR amplitude threshold in microsiemens ($\mu\text{S}$) (default: 0.01 $\mu\text{S}$).
    pub min_amplitude: Option<f64>,
    /// Minimum peak prominence in microsiemens ($\mu\text{S}$) (default: 0.005 $\mu\text{S}$).
    pub min_prominence: Option<f64>,
    /// Minimum inter-peak distance in seconds (default: 1.0 s).
    pub min_distance_sec: Option<f64>,
    /// Minimum SCR rise time duration in seconds from onset to peak (default: 0.1 s).
    pub min_rise_time_sec: Option<f64>,
    /// Maximum SCR rise time duration in seconds from onset to peak (default: 5.0 s).
    pub max_rise_time_sec: Option<f64>,
}

impl Default for EdaPeakDetectionConfig {
    fn default() -> Self {
        Self {
            min_amplitude: Some(0.01),
            min_prominence: Some(0.005),
            min_distance_sec: Some(1.0),
            min_rise_time_sec: Some(0.1),
            max_rise_time_sec: Some(5.0),
        }
    }
}

impl EdaPeakDetectionConfig {
    /// Create a default EDA peak detection configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set minimum SCR amplitude in microsiemens ($\mu\text{S}$).
    pub fn with_min_amplitude(mut self, amp: f64) -> Self {
        self.min_amplitude = Some(amp);
        self
    }

    /// Set minimum peak prominence in microsiemens ($\mu\text{S}$).
    pub fn with_min_prominence(mut self, prom: f64) -> Self {
        self.min_prominence = Some(prom);
        self
    }

    /// Set minimum inter-peak distance in seconds.
    pub fn with_min_distance_sec(mut self, sec: f64) -> Self {
        self.min_distance_sec = Some(sec);
        self
    }

    /// Set minimum SCR rise time in seconds.
    pub fn with_min_rise_time_sec(mut self, sec: f64) -> Self {
        self.min_rise_time_sec = Some(sec);
        self
    }

    /// Set maximum SCR rise time in seconds.
    pub fn with_max_rise_time_sec(mut self, sec: f64) -> Self {
        self.max_rise_time_sec = Some(sec);
        self
    }

    /// Validate configuration parameters.
    pub fn validate(&self) -> Result<()> {
        if matches!(self.min_amplitude, Some(a) if !a.is_finite() || a < 0.0) {
            return Err(SignalError::NonFiniteInput);
        }
        if matches!(self.min_prominence, Some(p) if !p.is_finite() || p < 0.0) {
            return Err(SignalError::NonFiniteInput);
        }
        if matches!(self.min_distance_sec, Some(d) if !d.is_finite() || d <= 0.0) {
            return Err(SignalError::InvalidWindowSize(0));
        }
        if matches!(self.min_rise_time_sec, Some(r) if !r.is_finite() || r <= 0.0) {
            return Err(SignalError::InvalidWindowSize(0));
        }
        if matches!(self.max_rise_time_sec, Some(r) if !r.is_finite() || r <= 0.0) {
            return Err(SignalError::InvalidWindowSize(0));
        }
        if matches!((self.min_rise_time_sec, self.max_rise_time_sec), (Some(min_r), Some(max_r)) if min_r >= max_r)
        {
            return Err(SignalError::InvalidWindowSize(0));
        }
        Ok(())
    }
}

/// Structured representation of a discrete Skin Conductance Response (SCR) event.
#[derive(Debug, Clone, PartialEq)]
pub struct ScrEvent {
    /// 0-indexed sample index of SCR onset (preceding trough).
    pub onset_index: usize,
    /// 0-indexed sample index of SCR peak (maximum amplitude).
    pub peak_index: usize,
    /// SCR amplitude in microsiemens ($\mu\text{S}$) measured from onset to peak.
    pub amplitude: f64,
    /// SCR rise time duration in seconds from onset to peak.
    pub rise_time_sec: f64,
}

/// Detect discrete Skin Conductance Response (SCR) events in a Phasic EDA signal.
///
/// # Scientific Contract
/// - **Inputs**:
///   - `phasic_signal`: 1D Phasic component array of an EDA signal.
///   - `sampling_rate`: Sampling frequency $F_s$ in Hertz (Hz). Must be $> 0.0$.
///   - `config`: [`EdaPeakDetectionConfig`] parameters.
/// - **Output**: `Vec<ScrEvent>` containing validated SCR events in ascending chronological order.
/// - **Methodology**: Uses generic peak detection (`signal_findpeaks_config`) to locate candidate phasic peaks $\to$ searches backward to identify preceding onset troughs $\to$ evaluates amplitude and rise time constraints.
///
/// # Errors
/// Returns [`SignalError`] if `phasic_signal` is empty, contains non-finite values, or if parameters are invalid.
pub fn eda_findpeaks_events(
    phasic_signal: &Array1<f64>,
    sampling_rate: f64,
    config: &EdaPeakDetectionConfig,
) -> Result<Vec<ScrEvent>> {
    let n = phasic_signal.len();
    if n == 0 {
        return Err(SignalError::EmptySignal);
    }
    if !sampling_rate.is_finite() || sampling_rate <= 0.0 {
        return Err(SignalError::InvalidSamplingRate(sampling_rate));
    }
    for &val in phasic_signal.iter() {
        if !val.is_finite() {
            return Err(SignalError::NonFiniteInput);
        }
    }
    config.validate()?;

    let min_amp = config.min_amplitude.unwrap_or(0.01);
    let min_prom = config.min_prominence.unwrap_or(0.005);
    let min_dist_sec = config.min_distance_sec.unwrap_or(1.0);
    let min_rise_sec = config.min_rise_time_sec.unwrap_or(0.1);
    let max_rise_sec = config.max_rise_time_sec.unwrap_or(5.0);

    // Noise floor protection for constant / near-zero phasic signals
    let max_val = phasic_signal.fold(f64::NEG_INFINITY, |acc, &x| acc.max(x));
    if max_val < 1e-12 || max_val < min_amp {
        return Ok(Vec::new());
    }

    let min_dist_samples = (min_dist_sec * sampling_rate).round().max(1.0) as usize;
    if n < min_dist_samples {
        return Err(SignalError::InsufficientSamples {
            required: min_dist_samples,
            provided: n,
        });
    }

    let peak_cfg = PeakDetectionConfig::new()
        .with_min_distance(min_dist_samples)
        .with_min_height(min_amp)
        .with_min_prominence(min_prom);

    let candidate_peaks = signal_findpeaks_config(phasic_signal, &peak_cfg)?;
    let mut events = Vec::new();
    let mut prev_onset = 0;

    for &peak_idx in &candidate_peaks {
        if peak_idx == 0 {
            continue;
        }

        // Search backward for preceding onset trough
        let mut onset_idx = peak_idx;
        let mut min_val = phasic_signal[peak_idx];

        let search_limit = prev_onset
            .max(peak_idx.saturating_sub((max_rise_sec * sampling_rate).round() as usize));
        for j in (search_limit..peak_idx).rev() {
            if phasic_signal[j] < min_val {
                min_val = phasic_signal[j];
                onset_idx = j;
                if min_val <= 0.0 {
                    break;
                }
            } else if j + 1 < peak_idx && phasic_signal[j] > phasic_signal[j + 1] {
                // Local minimum reached
                break;
            }
        }

        let amplitude = phasic_signal[peak_idx] - phasic_signal[onset_idx];
        let rise_time_sec = (peak_idx - onset_idx) as f64 / sampling_rate;

        if amplitude >= min_amp && rise_time_sec >= min_rise_sec && rise_time_sec <= max_rise_sec {
            events.push(ScrEvent {
                onset_index: onset_idx,
                peak_index: peak_idx,
                amplitude,
                rise_time_sec,
            });
            prev_onset = onset_idx;
        }
    }

    Ok(events)
}

/// Locate SCR peak sample indices in a Phasic EDA signal given a [`EdaPeakDetectionConfig`].
pub fn eda_findpeaks_config(
    phasic_signal: &Array1<f64>,
    sampling_rate: f64,
    config: &EdaPeakDetectionConfig,
) -> Result<Vec<usize>> {
    let events = eda_findpeaks_events(phasic_signal, sampling_rate, config)?;
    Ok(events.into_iter().map(|e| e.peak_index).collect())
}

/// Locate SCR peak mask (`Array1<bool>`) in a Phasic EDA signal given a [`EdaPeakDetectionConfig`].
pub fn eda_findpeaks_mask(
    phasic_signal: &Array1<f64>,
    sampling_rate: f64,
    config: &EdaPeakDetectionConfig,
) -> Result<Array1<bool>> {
    let indices = eda_findpeaks_config(phasic_signal, sampling_rate, config)?;
    let mut mask = Array1::<bool>::from_elem(phasic_signal.len(), false);
    for idx in indices {
        mask[idx] = true;
    }
    Ok(mask)
}

/// Locate SCR peak mask (`Array1<bool>`) in a Phasic EDA signal given a sampling rate.
pub fn eda_findpeaks(phasic_signal: &Array1<f64>, sampling_rate: f64) -> Result<Array1<bool>> {
    if !sampling_rate.is_finite() || sampling_rate <= 0.0 {
        return Err(SignalError::InvalidSamplingRate(sampling_rate));
    }
    let config = EdaPeakDetectionConfig::default();
    eda_findpeaks_mask(phasic_signal, sampling_rate, &config)
}
