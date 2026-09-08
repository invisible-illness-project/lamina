use crate::error::{Result, SignalError};
use crate::signal::filter::{FilterSpec, signal_filtfilt};
use crate::signal::peaks::{PeakDetectionConfig, signal_findpeaks_config};
use crate::signal::smooth::signal_smooth_moving_average;
use ndarray::Array1;

/// Configuration parameters for Pan-Tompkins ECG R-peak detection.
#[derive(Debug, Clone, PartialEq)]
pub struct EcgPeakDetectionConfig {
    /// Bandpass lower cutoff frequency in Hz (default: 5.0 Hz).
    pub lowcut: Option<f64>,
    /// Bandpass upper cutoff frequency in Hz (default: 15.0 Hz).
    pub highcut: Option<f64>,
    /// Bandpass Butterworth filter order (default: 2).
    pub filter_order: Option<usize>,
    /// Moving window integration length in seconds (default: 0.150 s = 150 ms).
    pub integration_window_sec: Option<f64>,
    /// Physiological refractory period in seconds (default: 0.200 s = 200 ms).
    pub refractory_period_sec: Option<f64>,
    /// Enable searchback for missed beats when RR interval exceeds 1.66 * RR_avg (default: true).
    pub searchback: Option<bool>,
    /// Primary threshold multiplier factor for signal/noise estimation (default: 0.25).
    pub threshold_multiplier: Option<f64>,
}

impl Default for EcgPeakDetectionConfig {
    fn default() -> Self {
        Self {
            lowcut: Some(5.0),
            highcut: Some(15.0),
            filter_order: Some(2),
            integration_window_sec: Some(0.150),
            refractory_period_sec: Some(0.200),
            searchback: Some(true),
            threshold_multiplier: Some(0.25),
        }
    }
}

impl EcgPeakDetectionConfig {
    /// Create a new default Pan-Tompkins ECG configuration.
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

    /// Set moving window integration length in seconds.
    pub fn with_integration_window_sec(mut self, sec: f64) -> Self {
        self.integration_window_sec = Some(sec);
        self
    }

    /// Set physiological refractory period in seconds.
    pub fn with_refractory_period_sec(mut self, sec: f64) -> Self {
        self.refractory_period_sec = Some(sec);
        self
    }

    /// Enable or disable searchback for missed R-peaks.
    pub fn with_searchback(mut self, enable: bool) -> Self {
        self.searchback = Some(enable);
        self
    }

    /// Set primary threshold multiplier factor.
    pub fn with_threshold_multiplier(mut self, mult: f64) -> Self {
        self.threshold_multiplier = Some(mult);
        self
    }

    /// Validate configuration values.
    pub fn validate(&self) -> Result<()> {
        if matches!(self.lowcut, Some(lc) if !lc.is_finite() || lc <= 0.0) {
            return Err(SignalError::InvalidCutoffFrequency(format!(
                "Lowcut ({}) must be positive and finite",
                self.lowcut.unwrap()
            )));
        }
        if matches!(self.highcut, Some(hc) if !hc.is_finite() || hc <= 0.0) {
            return Err(SignalError::InvalidCutoffFrequency(format!(
                "Highcut ({}) must be positive and finite",
                self.highcut.unwrap()
            )));
        }
        if matches!((self.lowcut, self.highcut), (Some(lc), Some(hc)) if lc >= hc) {
            return Err(SignalError::InvalidCutoffFrequency(
                "Lowcut must be < Highcut".to_string(),
            ));
        }
        if matches!(self.filter_order, Some(0)) {
            return Err(SignalError::InvalidFilterOrder(0));
        }
        if matches!(self.integration_window_sec, Some(iw) if !iw.is_finite() || iw <= 0.0) {
            return Err(SignalError::InvalidWindowSize(0));
        }
        if matches!(self.refractory_period_sec, Some(rp) if !rp.is_finite() || rp <= 0.0) {
            return Err(SignalError::InvalidWindowSize(0));
        }
        if matches!(self.threshold_multiplier, Some(tm) if !tm.is_finite() || tm <= 0.0 || tm >= 1.0)
        {
            return Err(SignalError::InvalidCutoffFrequency(
                "Threshold multiplier must be strictly between 0.0 and 1.0".to_string(),
            ));
        }
        Ok(())
    }
}

/// Locate R-peaks in an Electrocardiogram (ECG) signal using Pan-Tompkins pipeline with custom config.
///
/// # Scientific Contract
/// - **Inputs**:
///   - `signal`: 1D ECG signal array (raw or cleaned amplitude).
///   - `sampling_rate`: Sampling frequency $F_s$ in Hertz (Hz). Must be $> 0.0$.
///   - `config`: [`EcgPeakDetectionConfig`] parameters.
/// - **Output**: `Vec<usize>` containing 0-indexed sample indices of validated R-peaks in ascending chronological order.
/// - **Methodology**: 5–15 Hz bandpass filtering $\to$ 5-point derivative $\to$ element-wise squaring $\to$ 150 ms moving window integration $\to$ candidate local maxima $\to$ adaptive dual-threshold estimation ($SPKI, NPKI$) with 200 ms refractory period and searchback.
///
/// # Errors
/// Returns [`SignalError`] if `signal` is empty, contains non-finite samples, or if parameters are invalid.
pub fn ecg_findpeaks_config(
    signal: &Array1<f64>,
    sampling_rate: f64,
    config: &EcgPeakDetectionConfig,
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

    let lowcut = config.lowcut.unwrap_or(5.0);
    let highcut = config.highcut.unwrap_or(15.0);
    let filter_order = config.filter_order.unwrap_or(2);
    let integration_sec = config.integration_window_sec.unwrap_or(0.150);
    let refractory_sec = config.refractory_period_sec.unwrap_or(0.200);
    let enable_searchback = config.searchback.unwrap_or(true);
    let mult = config.threshold_multiplier.unwrap_or(0.25);

    let refractory_samples = (refractory_sec * sampling_rate).round().max(1.0) as usize;
    let integration_samples = (integration_sec * sampling_rate).round().max(1.0) as usize;

    // 1. Bandpass Filtering (5 - 15 Hz Butterworth SOS)
    let nyquist = sampling_rate / 2.0;
    if lowcut >= highcut || highcut >= nyquist {
        return Err(SignalError::InvalidCutoffFrequency(format!(
            "Cutoff frequencies ({}, {}) must satisfy 0 < lowcut < highcut < Nyquist ({})",
            lowcut, highcut, nyquist
        )));
    }

    let required_samples = (3 * filter_order).max(integration_samples);
    if n < required_samples {
        return Err(SignalError::InsufficientSamples {
            required: required_samples,
            provided: n,
        });
    }

    let filter_spec = FilterSpec::bandpass(sampling_rate, lowcut, highcut, filter_order);
    let filtered_ecg = signal_filtfilt(signal, &filter_spec)?;

    // 2. 5-Point Derivative: d[n] = (1 / 8T) * (-x[n-2] - 2x[n-1] + 2x[n+1] + x[n+2])
    let dt_factor = sampling_rate / 8.0;
    let mut derivative = Array1::<f64>::zeros(n);

    // Endpoints fallback
    derivative[0] = (filtered_ecg[1] - filtered_ecg[0]) * (sampling_rate / 2.0);
    derivative[1] = (filtered_ecg[2] - filtered_ecg[0]) * (sampling_rate / 4.0);

    for i in 2..(n - 2) {
        derivative[i] = (-filtered_ecg[i - 2] - 2.0 * filtered_ecg[i - 1]
            + 2.0 * filtered_ecg[i + 1]
            + filtered_ecg[i + 2])
            * dt_factor;
    }

    derivative[n - 2] = (filtered_ecg[n - 1] - filtered_ecg[n - 3]) * (sampling_rate / 4.0);
    derivative[n - 1] = (filtered_ecg[n - 1] - filtered_ecg[n - 2]) * (sampling_rate / 2.0);

    // 3. Element-wise Squaring
    let sq_diff = derivative.mapv(|x| x * x);

    // 4. Moving Window Integration (150 ms)
    let integrated = signal_smooth_moving_average(&sq_diff, integration_samples)?;

    // 5. Candidate Local Maxima Detection on Integrated Signal
    let peak_cfg = PeakDetectionConfig::new().with_min_distance(refractory_samples);
    let candidate_indices = signal_findpeaks_config(&integrated, &peak_cfg)?;

    if candidate_indices.is_empty() {
        return Ok(Vec::new());
    }

    // 6. Adaptive Dual Thresholding & Refractory Filtering
    let mut candidate_heights: Vec<f64> = candidate_indices
        .iter()
        .map(|&idx| integrated[idx])
        .collect();
    candidate_heights.sort_by(|a, b| a.total_cmp(b));

    // Initialize SPKI (Signal Peak level) and NPKI (Noise Peak level)
    let n_cands = candidate_heights.len();
    let mut spki = candidate_heights[(n_cands as f64 * 0.75) as usize];
    let mut npki = candidate_heights[(n_cands as f64 * 0.25) as usize];

    if spki <= npki || spki < 1e-12 {
        return Ok(Vec::new());
    }
    let mut validated_integrated_peaks: Vec<usize> = Vec::new();
    let mut last_peak_idx: Option<usize> = None;
    let mut rr_intervals: Vec<usize> = Vec::new();
    let mut recent_qrs_peaks: Vec<f64> = Vec::new();

    for &cand_idx in &candidate_indices {
        let y_val = integrated[cand_idx];
        let threshold_i1 = npki + mult * (spki - npki);

        if matches!(last_peak_idx, Some(last_idx) if cand_idx < last_idx + refractory_samples) {
            // Inside 200ms refractory period -> treat as T-wave or noise side-lobe.
            // Refractory peaks MUST NOT decay SPKI; update NPKI if noise.
            if y_val < threshold_i1 {
                npki = 0.125 * y_val + 0.875 * npki;
            }
            continue;
        }

        // Pan-Tompkins T-Wave Discrimination (200ms - 360ms window)
        // If candidate peak occurs within 360ms of previous peak and its magnitude is < 50% of recent QRS median,
        // classify as T-wave / secondary lobe.
        let twave_window_samples = (0.360 * sampling_rate).round() as usize;
        if matches!(last_peak_idx, Some(last_idx) if cand_idx < last_idx + twave_window_samples)
            && !recent_qrs_peaks.is_empty()
        {
            let mut sorted = recent_qrs_peaks.clone();
            sorted.sort_by(|a, b| a.total_cmp(b));
            let med_qrs = sorted[sorted.len() / 2];
            if y_val < 0.5 * med_qrs {
                npki = 0.125 * y_val + 0.875 * npki;
                continue;
            }
        }

        if y_val > threshold_i1 {
            // Check for searchback if RR interval > 1.66 * RR_avg
            if enable_searchback && !rr_intervals.is_empty() {
                let rr_avg: f64 =
                    rr_intervals.iter().sum::<usize>() as f64 / rr_intervals.len() as f64;
                let last_idx = last_peak_idx.unwrap();
                let current_rr = cand_idx - last_idx;

                if current_rr as f64 > 1.66 * rr_avg {
                    // Missed beat searchback with THRESHOLD_I2 = 0.5 * THRESHOLD_I1
                    let threshold_i2 = 0.5 * threshold_i1;
                    for &sb_cand in &candidate_indices {
                        if sb_cand > last_idx + refractory_samples
                            && sb_cand + refractory_samples < cand_idx
                        {
                            let sb_val = integrated[sb_cand];
                            if sb_val > threshold_i2 {
                                validated_integrated_peaks.push(sb_cand);
                                rr_intervals.push(sb_cand - last_idx);
                                if rr_intervals.len() > 8 {
                                    rr_intervals.remove(0);
                                }
                                last_peak_idx = Some(sb_cand);

                                let capped_val = if !recent_qrs_peaks.is_empty() {
                                    let mut sorted = recent_qrs_peaks.clone();
                                    sorted.sort_by(|a, b| a.total_cmp(b));
                                    let med_qrs = sorted[sorted.len() / 2];
                                    sb_val.min(2.5 * med_qrs)
                                } else {
                                    sb_val
                                };
                                recent_qrs_peaks.push(capped_val);
                                if recent_qrs_peaks.len() > 8 {
                                    recent_qrs_peaks.remove(0);
                                }
                                spki = 0.25 * capped_val + 0.75 * spki;
                                break;
                            }
                        }
                    }
                }
            }

            // Validate current candidate
            if let Some(last_idx) = last_peak_idx {
                rr_intervals.push(cand_idx - last_idx);
                if rr_intervals.len() > 8 {
                    rr_intervals.remove(0);
                }
            }
            validated_integrated_peaks.push(cand_idx);
            last_peak_idx = Some(cand_idx);

            let capped_val = if !recent_qrs_peaks.is_empty() {
                let mut sorted = recent_qrs_peaks.clone();
                sorted.sort_by(|a, b| a.total_cmp(b));
                let med_qrs = sorted[sorted.len() / 2];
                y_val.min(2.5 * med_qrs)
            } else {
                y_val
            };
            recent_qrs_peaks.push(capped_val);
            if recent_qrs_peaks.len() > 8 {
                recent_qrs_peaks.remove(0);
            }
            spki = 0.125 * capped_val + 0.875 * spki;
        } else {
            npki = 0.125 * y_val + 0.875 * npki;
        }
    }

    // 7. Precise R-Peak Fine-Alignment on Filtered ECG Signal
    let mut final_r_peaks: Vec<usize> = Vec::new();
    let search_radius = integration_samples;
    let near_radius = (search_radius / 3).max(1);

    for &int_idx in &validated_integrated_peaks {
        let start = int_idx.saturating_sub(search_radius);
        let end = (int_idx + search_radius + 1).min(n);

        // Determine local QRS polarity in a tight neighborhood around the candidate integrated peak
        let near_start = int_idx.saturating_sub(near_radius);
        let near_end = (int_idx + near_radius + 1).min(n);

        let mut near_max_idx = near_start;
        let mut near_max_abs = filtered_ecg[near_start].abs();

        for i in near_start..near_end {
            let abs_val = filtered_ecg[i].abs();
            if abs_val > near_max_abs {
                near_max_abs = abs_val;
                near_max_idx = i;
            }
        }

        let is_negative_qrs = filtered_ecg[near_max_idx] < 0.0;

        // Fine-align to QRS peak matching candidate polarity within search window
        let mut max_idx = start;
        let mut best_val = filtered_ecg[start];

        for i in start..end {
            let val = filtered_ecg[i];
            if is_negative_qrs {
                if val < best_val {
                    best_val = val;
                    max_idx = i;
                }
            } else if val > best_val {
                best_val = val;
                max_idx = i;
            }
        }

        // Refractory enforcement on aligned peaks
        if matches!(final_r_peaks.last(), Some(&prev_r) if max_idx < prev_r + refractory_samples) {
            continue;
        }
        final_r_peaks.push(max_idx);
    }

    final_r_peaks.sort_unstable();
    final_r_peaks.dedup();
    Ok(final_r_peaks)
}

/// Locate R-peaks mask (`Array1<bool>`) in an ECG signal given a [`EcgPeakDetectionConfig`].
pub fn ecg_findpeaks_mask(
    signal: &Array1<f64>,
    sampling_rate: f64,
    config: &EcgPeakDetectionConfig,
) -> Result<Array1<bool>> {
    let indices = ecg_findpeaks_config(signal, sampling_rate, config)?;
    let mut mask = Array1::<bool>::from_elem(signal.len(), false);
    for idx in indices {
        mask[idx] = true;
    }
    Ok(mask)
}

/// Locate R-peaks in an Electrocardiogram (ECG) signal using Pan-Tompkins pipeline with default configuration.
///
/// Convenience entry point maintaining backward compatibility.
pub fn ecg_findpeaks(signal: &Array1<f64>, sampling_rate: f64) -> Result<Array1<bool>> {
    let config = EcgPeakDetectionConfig::default();
    ecg_findpeaks_mask(signal, sampling_rate, &config)
}
