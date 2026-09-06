use crate::error::{Result, SignalError};
use crate::rppg::config::{RppgAlgorithmId, RppgPreprocessingConfig};
use crate::rppg::quality::{RppgQualitySummary, RppgSegmentQuality};
use ndarray::Array1;

/// Spatial mean RGB channel intensity sample for a single video frame.
#[derive(Debug, Clone, PartialEq)]
pub struct RoiSample {
    /// Physical timestamp in seconds
    pub timestamp_sec: f64,
    /// Spatial mean red channel intensity in $[0.0, 255.0]$
    pub red: f64,
    /// Spatial mean green channel intensity in $[0.0, 255.0]$
    pub green: f64,
    /// Spatial mean blue channel intensity in $[0.0, 255.0]$
    pub blue: f64,
    /// Number of valid pixels contributing to ROI sample
    pub valid_pixels: usize,
}

/// Temporal RGB optical intensity series extracted from sequential video ROIs.
#[derive(Debug, Clone, PartialEq)]
pub struct OpticalSignal {
    /// Physical timestamps in seconds
    pub timestamps_sec: Vec<f64>,
    /// Red channel intensity series
    pub red: Vec<f64>,
    /// Green channel intensity series
    pub green: Vec<f64>,
    /// Blue channel intensity series
    pub blue: Vec<f64>,
    /// Valid pixel count series
    pub valid_pixel_counts: Vec<usize>,
}

impl OpticalSignal {
    /// Construct an `OpticalSignal` from a sequence of `RoiSample`s.
    pub fn from_samples(samples: &[RoiSample]) -> Result<Self> {
        if samples.is_empty() {
            return Err(SignalError::EmptySignal);
        }

        let mut timestamps_sec = Vec::with_capacity(samples.len());
        let mut red = Vec::with_capacity(samples.len());
        let mut green = Vec::with_capacity(samples.len());
        let mut blue = Vec::with_capacity(samples.len());
        let mut valid_pixel_counts = Vec::with_capacity(samples.len());

        for s in samples {
            timestamps_sec.push(s.timestamp_sec);
            red.push(s.red);
            green.push(s.green);
            blue.push(s.blue);
            valid_pixel_counts.push(s.valid_pixels);
        }

        let signal = Self {
            timestamps_sec,
            red,
            green,
            blue,
            valid_pixel_counts,
        };

        signal.validate()?;
        Ok(signal)
    }

    /// Validate vector dimension equality, finite values, and timestamp monotonicity.
    pub fn validate(&self) -> Result<()> {
        let len = self.timestamps_sec.len();
        if len == 0 {
            return Err(SignalError::EmptySignal);
        }
        if self.red.len() != len || self.green.len() != len || self.blue.len() != len {
            return Err(SignalError::DimensionMismatch);
        }

        let mut prev_t = self.timestamps_sec[0];
        if !prev_t.is_finite() {
            return Err(SignalError::NonFiniteInput);
        }

        for i in 1..len {
            let t = self.timestamps_sec[i];
            if !t.is_finite() || t <= prev_t {
                return Err(SignalError::UnsortedEvents);
            }
            if !self.red[i].is_finite() || !self.green[i].is_finite() || !self.blue[i].is_finite() {
                return Err(SignalError::NonFiniteInput);
            }
            prev_t = t;
        }

        Ok(())
    }

    /// Estimate mean effective sampling rate in Hz ($F_s = \frac{N-1}{t_{\text{last}} - t_{\text{first}}}$).
    pub fn mean_sampling_rate(&self) -> Result<f64> {
        self.validate()?;
        if self.timestamps_sec.len() < 2 {
            return Err(SignalError::InsufficientSamples {
                required: 2,
                provided: self.timestamps_sec.len(),
            });
        }
        let dt = self.timestamps_sec.last().unwrap() - self.timestamps_sec[0];
        if dt <= 0.0 || !dt.is_finite() {
            return Err(SignalError::InvalidSamplingRate(0.0));
        }
        let fs = (self.timestamps_sec.len() - 1) as f64 / dt;
        if fs <= 0.0 || !fs.is_finite() {
            return Err(SignalError::InvalidSamplingRate(fs));
        }
        Ok(fs)
    }

    /// Return index bounds `(start_idx, end_idx)` for half-open physical timestamp range $[t_{\text{start}}, t_{\text{end}})$.
    pub fn timestamp_range(&self, t_start: f64, t_end: f64) -> Result<(usize, usize)> {
        self.validate()?;
        if !t_start.is_finite() || !t_end.is_finite() || t_start >= t_end {
            return Err(SignalError::NonFiniteInput);
        }
        let start_idx = self.timestamps_sec.partition_point(|&t| t < t_start);
        let end_idx = self.timestamps_sec.partition_point(|&t| t < t_end);
        Ok((start_idx, end_idx))
    }

    /// Extract a sub-slice window of the optical signal.
    pub fn slice(&self, start_idx: usize, len: usize) -> Result<Self> {
        if start_idx + len > self.timestamps_sec.len() || len == 0 {
            return Err(SignalError::InsufficientSamples {
                required: start_idx + len,
                provided: self.timestamps_sec.len(),
            });
        }

        let sliced = Self {
            timestamps_sec: self.timestamps_sec[start_idx..(start_idx + len)].to_vec(),
            red: self.red[start_idx..(start_idx + len)].to_vec(),
            green: self.green[start_idx..(start_idx + len)].to_vec(),
            blue: self.blue[start_idx..(start_idx + len)].to_vec(),
            valid_pixel_counts: self.valid_pixel_counts[start_idx..(start_idx + len)].to_vec(),
        };
        sliced.validate()?;
        Ok(sliced)
    }

    /// Perform window-local optical signal preprocessing (linear detrending and channel mean normalization).
    pub fn preprocess(&self, config: &RppgPreprocessingConfig) -> Result<Self> {
        self.validate()?;
        let n = self.timestamps_sec.len();
        if n < 2 {
            return Ok(self.clone());
        }

        let mut r = self.red.clone();
        let mut g = self.green.clone();
        let mut b = self.blue.clone();

        if config.detrend {
            let t_mean = self.timestamps_sec.iter().sum::<f64>() / n as f64;
            let denom: f64 = self
                .timestamps_sec
                .iter()
                .map(|&t| (t - t_mean).powi(2))
                .sum();

            if denom > 1e-12 {
                for ch in [&mut r, &mut g, &mut b] {
                    let y_mean = ch.iter().sum::<f64>() / n as f64;
                    let num: f64 = self
                        .timestamps_sec
                        .iter()
                        .zip(ch.iter())
                        .map(|(&t, &y)| (t - t_mean) * (y - y_mean))
                        .sum();
                    let slope = num / denom;
                    for (i, val) in ch.iter_mut().enumerate() {
                        *val -= slope * (self.timestamps_sec[i] - t_mean);
                    }
                }
            }
        }

        if config.normalize_channels {
            for ch in [&mut r, &mut g, &mut b] {
                let mean = ch.iter().sum::<f64>() / n as f64;
                if mean <= 1e-6 || !mean.is_finite() {
                    return Err(SignalError::NonFiniteInput);
                }
                for val in ch.iter_mut() {
                    *val /= mean;
                }
            }
        }

        let processed = Self {
            timestamps_sec: self.timestamps_sec.clone(),
            red: r,
            green: g,
            blue: b,
            valid_pixel_counts: self.valid_pixel_counts.clone(),
        };
        processed.validate()?;
        Ok(processed)
    }
}

/// Contiguous valid rPPG optical pulse segment meeting quality and gap-continuity criteria.
#[derive(Debug, Clone, PartialEq)]
pub struct RppgSegment {
    /// Physical start timestamp in seconds
    pub start_sec: f64,
    /// Physical end timestamp in seconds
    pub end_sec: f64,
    /// Physical timestamps in seconds
    pub timestamps_sec: Vec<f64>,
    /// Extracted pulse waveform samples
    pub waveform: Vec<f64>,
    /// Sampling rate in Hz
    pub sampling_rate_hz: f64,
    /// Segment quality metadata
    pub quality: RppgSegmentQuality,
}

impl RppgSegment {
    /// Convert segment waveform into a 1D `Array1<f64>` for downstream Lamina PPG processing.
    pub fn to_ndarray(&self) -> Array1<f64> {
        Array1::from_vec(self.waveform.clone())
    }
}

/// Standardized camera-derived optical pulse surrogate waveform and quality summary.
#[derive(Debug, Clone, PartialEq)]
pub struct RppgSignal {
    /// Physical timestamps in seconds
    pub timestamps_sec: Vec<f64>,
    /// Extracted pulse waveform samples
    pub waveform: Vec<f64>,
    /// Mean or target sampling frequency in Hz
    pub sampling_rate_hz: f64,
    /// Multi-tiered quality metadata summary
    pub quality: RppgQualitySummary,
    /// Extraction algorithm identity
    pub algorithm: RppgAlgorithmId,
}

impl RppgSignal {
    /// Convert waveform into an 1D `Array1<f64>` for downstream Lamina PPG processing.
    pub fn to_ndarray(&self) -> Array1<f64> {
        Array1::from_vec(self.waveform.clone())
    }

    /// Extract contiguous valid signal segments where waveform samples are finite and physical gaps do not exceed `max_gap_sec`.
    ///
    /// Segment quality is computed as a duration-weighted overlap aggregation across quality windows covering the segment interval.
    pub fn valid_segments(&self, max_gap_sec: f64) -> Result<Vec<RppgSegment>> {
        if !max_gap_sec.is_finite() || max_gap_sec <= 0.0 {
            return Err(SignalError::InvalidWindowSize(0));
        }

        if self.waveform.is_empty() || self.timestamps_sec.len() != self.waveform.len() {
            return Ok(Vec::new());
        }

        let aggregate_quality_for_range = |t_a: f64, t_b: f64| -> RppgSegmentQuality {
            let seg_dur = (t_b - t_a).max(0.0);
            if seg_dur <= 0.0 {
                return RppgSegmentQuality {
                    start_sec: t_a,
                    end_sec: t_b,
                    overall: 0.0,
                    roi_quality: 0.0,
                    motion_quality: 0.0,
                    illumination_quality: 0.0,
                    signal_quality: 0.0,
                    valid_fraction: 0.0,
                };
            }

            // Partition segment into non-overlapping elementary intervals using quality boundaries
            let mut bounds = vec![t_a, t_b];
            for q in &self.quality.segments {
                if q.start_sec > t_a && q.start_sec < t_b {
                    bounds.push(q.start_sec);
                }
                if q.end_sec > t_a && q.end_sec < t_b {
                    bounds.push(q.end_sec);
                }
            }
            bounds.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            bounds.dedup_by(|a, b| (*a - *b).abs() < 1e-6);

            let mut sum_overall = 0.0f64;
            let mut sum_roi = 0.0f64;
            let mut sum_motion = 0.0f64;
            let mut sum_illum = 0.0f64;
            let mut sum_signal = 0.0f64;

            for win in bounds.windows(2) {
                let tau_0 = win[0];
                let tau_1 = win[1];
                let elem_dur = (tau_1 - tau_0).max(0.0);
                if elem_dur <= 0.0 {
                    continue;
                }

                let mut active_cnt = 0usize;
                let mut a_overall = 0.0f64;
                let mut a_roi = 0.0f64;
                let mut a_motion = 0.0f64;
                let mut a_illum = 0.0f64;
                let mut a_signal = 0.0f64;

                for q in &self.quality.segments {
                    if q.start_sec <= tau_0 + 1e-6 && q.end_sec >= tau_1 - 1e-6 {
                        active_cnt += 1;
                        a_overall += q.overall;
                        a_roi += q.roi_quality;
                        a_motion += q.motion_quality;
                        a_illum += q.illumination_quality;
                        a_signal += q.signal_quality;
                    }
                }

                if active_cnt > 0 {
                    let cnt_f64 = active_cnt as f64;
                    sum_overall += (a_overall / cnt_f64) * elem_dur;
                    sum_roi += (a_roi / cnt_f64) * elem_dur;
                    sum_motion += (a_motion / cnt_f64) * elem_dur;
                    sum_illum += (a_illum / cnt_f64) * elem_dur;
                    sum_signal += (a_signal / cnt_f64) * elem_dur;
                }
                // Uncovered elementary sub-intervals (active_cnt == 0) contribute 0.0
                // to avoid falsely inflating segment quality.
            }

            RppgSegmentQuality {
                start_sec: t_a,
                end_sec: t_b,
                overall: (sum_overall / seg_dur).clamp(0.0, 1.0),
                roi_quality: (sum_roi / seg_dur).clamp(0.0, 1.0),
                motion_quality: (sum_motion / seg_dur).clamp(0.0, 1.0),
                illumination_quality: (sum_illum / seg_dur).clamp(0.0, 1.0),
                signal_quality: (sum_signal / seg_dur).clamp(0.0, 1.0),
                valid_fraction: 1.0,
            }
        };

        let mut segments = Vec::new();
        let mut cur_t = Vec::new();
        let mut cur_w = Vec::new();

        for i in 0..self.waveform.len() {
            let t = self.timestamps_sec[i];
            let w = self.waveform[i];

            let gap_exceeded = if let Some(&prev_t) = cur_t.last() {
                (t - prev_t) > max_gap_sec
            } else {
                false
            };

            if w.is_nan() || gap_exceeded {
                if cur_w.len() >= 4 {
                    let start_sec = cur_t[0];
                    let end_sec = *cur_t.last().unwrap();
                    let seg_q = aggregate_quality_for_range(start_sec, end_sec);

                    segments.push(RppgSegment {
                        start_sec,
                        end_sec,
                        timestamps_sec: cur_t,
                        waveform: cur_w,
                        sampling_rate_hz: self.sampling_rate_hz,
                        quality: seg_q,
                    });
                }
                cur_t = Vec::new();
                cur_w = Vec::new();

                if !w.is_nan() {
                    cur_t.push(t);
                    cur_w.push(w);
                }
            } else {
                cur_t.push(t);
                cur_w.push(w);
            }
        }

        if cur_w.len() >= 4 {
            let start_sec = cur_t[0];
            let end_sec = *cur_t.last().unwrap();
            let seg_q = aggregate_quality_for_range(start_sec, end_sec);

            segments.push(RppgSegment {
                start_sec,
                end_sec,
                timestamps_sec: cur_t,
                waveform: cur_w,
                sampling_rate_hz: self.sampling_rate_hz,
                quality: seg_q,
            });
        }

        Ok(segments)
    }

    /// Explicitly resample the optical pulse signal onto a uniform temporal grid at `target_fs` Hz.
    ///
    /// # Gap Preservation
    /// If an interval between adjacent original timestamps exceeds `max_gap_sec`, interpolated points
    /// falling inside that gap are marked invalid (`f64::NAN`), preserving data gaps without artificial interpolation.
    pub fn resample_uniform(&self, target_fs: f64, max_gap_sec: f64) -> Result<Self> {
        if !target_fs.is_finite() || target_fs <= 0.0 {
            return Err(SignalError::InvalidSamplingRate(target_fs));
        }
        if self.timestamps_sec.len() < 2 {
            return Err(SignalError::InsufficientSamples {
                required: 2,
                provided: self.timestamps_sec.len(),
            });
        }

        let t_start = self.timestamps_sec[0];
        let t_end = *self.timestamps_sec.last().unwrap();
        let duration = t_end - t_start;
        if duration <= 0.0 || !duration.is_finite() {
            return Err(SignalError::NonFiniteInput);
        }

        let num_samples = (duration * target_fs).round() as usize + 1;
        let mut uniform_t = Vec::with_capacity(num_samples);
        let mut uniform_w = Vec::with_capacity(num_samples);

        let dt_target = 1.0 / target_fs;
        let mut orig_idx = 0usize;

        for step in 0..num_samples {
            let t = t_start + step as f64 * dt_target;
            if t > t_end {
                break;
            }
            uniform_t.push(t);

            // Advance orig_idx to bounding segment [orig_idx, orig_idx + 1]
            while orig_idx + 1 < self.timestamps_sec.len() && self.timestamps_sec[orig_idx + 1] < t
            {
                orig_idx += 1;
            }

            if orig_idx + 1 >= self.timestamps_sec.len() {
                uniform_w.push(*self.waveform.last().unwrap());
                continue;
            }

            let t0 = self.timestamps_sec[orig_idx];
            let t1 = self.timestamps_sec[orig_idx + 1];
            let dt_orig = t1 - t0;

            if dt_orig > max_gap_sec {
                // Gap exceeds max_gap_sec: preserve missingness with NaN
                uniform_w.push(f64::NAN);
            } else if dt_orig <= 1e-12 {
                uniform_w.push(self.waveform[orig_idx]);
            } else {
                let frac = (t - t0) / dt_orig;
                let w0 = self.waveform[orig_idx];
                let w1 = self.waveform[orig_idx + 1];
                if w0.is_nan() || w1.is_nan() {
                    uniform_w.push(f64::NAN);
                } else {
                    uniform_w.push(w0 + frac * (w1 - w0));
                }
            }
        }

        Ok(Self {
            timestamps_sec: uniform_t,
            waveform: uniform_w,
            sampling_rate_hz: target_fs,
            quality: self.quality.clone(),
            algorithm: self.algorithm,
        })
    }
}
