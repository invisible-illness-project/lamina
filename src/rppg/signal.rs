use crate::error::{Result, SignalError};
use crate::rppg::config::RppgAlgorithmId;
use crate::rppg::quality::RppgQualitySummary;
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
