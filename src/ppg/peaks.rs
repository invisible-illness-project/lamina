use crate::error::{Result, SignalError};
use crate::signal::filter::{FilterSpec, signal_filtfilt};
use crate::signal::smooth::signal_smooth_moving_average;
use ndarray::Array1;

/// Configuration parameters for Elgendi PPG systolic peak detection.
#[derive(Debug, Clone, PartialEq)]
pub struct PpgPeakDetectionConfig {
    /// Bandpass lower cutoff frequency in Hz (default: 0.5 Hz).
    pub lowcut: Option<f64>,
    /// Bandpass upper cutoff frequency in Hz (default: 8.0 Hz).
    pub highcut: Option<f64>,
    /// Bandpass Butterworth filter order (default: 3).
    pub filter_order: Option<usize>,
    /// Short moving average window length for peak duration in seconds (default: 0.111 s = 111 ms).
    pub w_peak_sec: Option<f64>,
    /// Long moving average window length for beat duration in seconds (default: 0.667 s = 667 ms).
    pub w_beat_sec: Option<f64>,
    /// Threshold offset multiplier factor alpha (default: 0.02).
    pub alpha: Option<f64>,
    /// Refractory period for pulse wave in seconds (default: 0.300 s = 300 ms).
    pub refractory_period_sec: Option<f64>,
}

impl Default for PpgPeakDetectionConfig {
    fn default() -> Self {
        Self {
            lowcut: Some(0.5),
            highcut: Some(8.0),
            filter_order: Some(3),
            w_peak_sec: Some(0.111),
            w_beat_sec: Some(0.667),
            alpha: Some(0.02),
            refractory_period_sec: Some(0.300),
        }
    }
}

impl PpgPeakDetectionConfig {
    /// Create a default Elgendi PPG peak detection configuration.
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

    /// Set bandpass filter order.
    pub fn with_filter_order(mut self, order: usize) -> Self {
        self.filter_order = Some(order);
        self
    }

    /// Set short moving average window length in seconds.
    pub fn with_w_peak_sec(mut self, sec: f64) -> Self {
        self.w_peak_sec = Some(sec);
        self
    }

    /// Set long moving average window length in seconds.
    pub fn with_w_beat_sec(mut self, sec: f64) -> Self {
        self.w_beat_sec = Some(sec);
        self
    }

    /// Set threshold offset multiplier alpha.
    pub fn with_alpha(mut self, alpha: f64) -> Self {
        self.alpha = Some(alpha);
        self
    }

    /// Set refractory period for pulse wave in seconds.
    pub fn with_refractory_period_sec(mut self, sec: f64) -> Self {
        self.refractory_period_sec = Some(sec);
        self
    }

    /// Validate configuration values.
    pub fn validate(&self) -> Result<()> {
        if matches!(self.lowcut, Some(lc) if !lc.is_finite() || lc <= 0.0) {
            return Err(SignalError::InvalidCutoffFrequency(
                "Lowcut must be positive".to_string(),
            ));
        }
        if matches!(self.highcut, Some(hc) if !hc.is_finite() || hc <= 0.0) {
            return Err(SignalError::InvalidCutoffFrequency(
                "Highcut must be positive".to_string(),
            ));
        }
        if matches!((self.lowcut, self.highcut), (Some(lc), Some(hc)) if lc >= hc) {
            return Err(SignalError::InvalidCutoffFrequency(
                "Lowcut must be < Highcut".to_string(),
            ));
        }
        if matches!(self.filter_order, Some(0)) {
            return Err(SignalError::InvalidFilterOrder(0));
        }
        if matches!(self.w_peak_sec, Some(wp) if !wp.is_finite() || wp <= 0.0) {
            return Err(SignalError::InvalidWindowSize(0));
        }
        if matches!(self.w_beat_sec, Some(wb) if !wb.is_finite() || wb <= 0.0) {
            return Err(SignalError::InvalidWindowSize(0));
        }
        if matches!((self.w_peak_sec, self.w_beat_sec), (Some(wp), Some(wb)) if wp >= wb) {
            return Err(SignalError::InvalidWindowSize(0));
        }
        if matches!(self.alpha, Some(a) if !a.is_finite() || a < 0.0) {
            return Err(SignalError::NonFiniteInput);
        }
        if matches!(self.refractory_period_sec, Some(rp) if !rp.is_finite() || rp <= 0.0) {
            return Err(SignalError::InvalidWindowSize(0));
        }
        Ok(())
    }
}

/// Locate systolic peaks in a Photoplethysmogram (PPG) signal using Elgendi pipeline with custom config.
///
/// # Scientific Contract
/// - **Inputs**:
///   - `signal`: 1D PPG pulse waveform array.
///   - `sampling_rate`: Sampling frequency $F_s$ in Hertz (Hz). Must be $> 0.0$.
///   - `config`: [`PpgPeakDetectionConfig`] parameters.
/// - **Output**: `Vec<usize>` containing 0-indexed sample indices of systolic peaks in ascending chronological order.
/// - **Methodology**: 0.5–8.0 Hz bandpass pre-filtering $\to$ signal squaring $\to$ short ($W_{\text{peak}} \approx 111\text{ ms}$) and long ($W_{\text{beat}} \approx 667\text{ ms}$) moving average calculation $\to$ adaptive threshold block generation ($MA_{\text{beat}} + \alpha \bar{S}$) $\to$ block maximum systolic peak extraction with 300 ms refractory period.
///
/// # Errors
/// Returns [`SignalError`] if `signal` is empty, contains non-finite values, or if parameters are invalid.
pub fn ppg_findpeaks_config(
    signal: &Array1<f64>,
    sampling_rate: f64,
    config: &PpgPeakDetectionConfig,
) -> Result<Vec<usize>> {
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

    let lowcut = config.lowcut.unwrap_or(0.5);
    let highcut = config.highcut.unwrap_or(8.0);
    let filter_order = config.filter_order.unwrap_or(3);
    let w_peak_sec = config.w_peak_sec.unwrap_or(0.111);
    let w_beat_sec = config.w_beat_sec.unwrap_or(0.667);
    let alpha = config.alpha.unwrap_or(0.02);
    let refractory_sec = config.refractory_period_sec.unwrap_or(0.300);

    let refractory_samples = (refractory_sec * sampling_rate).round().max(1.0) as usize;
    let w_peak_samples = (w_peak_sec * sampling_rate).round().max(1.0) as usize;
    let w_beat_samples = (w_beat_sec * sampling_rate).round().max(1.0) as usize;

    // 1. Preprocessing: Bandpass Filter 0.5 - 8.0 Hz
    let nyquist = sampling_rate / 2.0;
    if lowcut >= highcut || highcut >= nyquist {
        return Err(SignalError::InvalidCutoffFrequency(format!(
            "Cutoff frequencies ({}, {}) must satisfy 0 < lowcut < highcut < Nyquist ({})",
            lowcut, highcut, nyquist
        )));
    }

    let required_samples = (3 * filter_order).max(w_beat_samples);
    if n < required_samples {
        return Err(SignalError::InsufficientSamples {
            required: required_samples,
            provided: n,
        });
    }

    let filter_spec = FilterSpec::bandpass(sampling_rate, lowcut, highcut, filter_order);
    let filtered_ppg = signal_filtfilt(signal, &filter_spec)?;

    // 2. Squaring / Signal Enhancement
    let squared = filtered_ppg.mapv(|v| if v > 0.0 { v * v } else { 0.0 });
    let s_bar = squared.mean().unwrap_or(0.0);
    if s_bar < 1e-12 {
        return Ok(Vec::new());
    }

    // 3. Short & Long Moving Averages
    let ma_peak = signal_smooth_moving_average(&squared, w_peak_samples)?;
    let ma_beat = signal_smooth_moving_average(&squared, w_beat_samples)?;

    // 4. Adaptive Threshold Block Generation
    let threshold = ma_beat.mapv(|mb| mb + alpha * s_bar);

    let mut in_block = false;
    let mut block_start = 0;
    let mut blocks: Vec<(usize, usize)> = Vec::new();

    for i in 0..n {
        if ma_peak[i] > threshold[i] {
            if !in_block {
                in_block = true;
                block_start = i;
            }
        } else if in_block {
            in_block = false;
            blocks.push((block_start, i - 1));
        }
    }
    if in_block {
        blocks.push((block_start, n - 1));
    }

    // 5. Candidate Peak Selection within Decision Blocks (Filter blocks smaller than W_peak)
    let mut candidate_peaks: Vec<usize> = Vec::new();
    for (start, end) in blocks {
        if start > end || end >= n {
            continue;
        }
        // Canonical Elgendi: Reject blocks shorter than W_peak duration (noise blocks)
        if (end - start + 1) < w_peak_samples {
            continue;
        }

        let mut max_idx = start;
        let mut max_val = filtered_ppg[start];

        for i in start..=end {
            if filtered_ppg[i] > max_val {
                max_val = filtered_ppg[i];
                max_idx = i;
            }
        }
        candidate_peaks.push(max_idx);
    }

    // 6. Refractory Period Filtering (300 ms)
    let mut final_systolic_peaks: Vec<usize> = Vec::new();
    for peak in candidate_peaks {
        if matches!(final_systolic_peaks.last(), Some(&prev_p) if peak < prev_p + refractory_samples)
        {
            continue;
        }
        final_systolic_peaks.push(peak);
    }

    final_systolic_peaks.sort_unstable();
    final_systolic_peaks.dedup();
    Ok(final_systolic_peaks)
}

/// Locate systolic peaks mask (`Array1<bool>`) in a PPG signal given a [`PpgPeakDetectionConfig`].
pub fn ppg_findpeaks_mask(
    signal: &Array1<f64>,
    sampling_rate: f64,
    config: &PpgPeakDetectionConfig,
) -> Result<Array1<bool>> {
    let indices = ppg_findpeaks_config(signal, sampling_rate, config)?;
    let mut mask = Array1::<bool>::from_elem(signal.len(), false);
    for idx in indices {
        mask[idx] = true;
    }
    Ok(mask)
}

/// Locate systolic peaks in a Photoplethysmogram (PPG) signal using Elgendi pipeline with default configuration.
///
/// Convenience entry point maintaining backward compatibility.
pub fn ppg_findpeaks(signal: &Array1<f64>, sampling_rate: f64) -> Result<Array1<bool>> {
    let config = PpgPeakDetectionConfig::default();
    ppg_findpeaks_mask(signal, sampling_rate, &config)
}
