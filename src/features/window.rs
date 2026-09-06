use crate::error::{Result, SignalError};
use crate::features::config::WindowConfig;

/// A fixed-duration physiological feature extraction time window.
#[derive(Debug, Clone, PartialEq)]
pub struct FeatureWindow {
    /// Window physical start time in seconds relative to recording reference
    pub start_time_sec: f64,
    /// Window physical end time in seconds (half-open $[t_{\text{start}}, t_{\text{end}})$)
    pub end_time_sec: f64,
    /// Window duration in seconds ($t_{\text{end}} - t_{\text{start}}$)
    pub duration_sec: f64,
}

/// Generate sliding fixed-duration feature windows over a recording interval $[t_{\text{start}}, t_{\text{end}}]$.
///
/// # Half-Open Boundary Convention
/// Windows are generated with half-open physical time boundaries $[t_{\text{start}}, t_{\text{end}})$.
///
/// # Errors
/// Returns [`SignalError::NonFiniteInput`] if timestamps or configuration parameters are non-finite.
/// Returns [`SignalError::InvalidWindowSize`] if duration $\le 0.0$, step $\le 0.0$, or `start_time_sec` $\ge$ `end_time_sec`.
pub fn generate_windows(
    start_time_sec: f64,
    end_time_sec: f64,
    config: &WindowConfig,
) -> Result<Vec<FeatureWindow>> {
    if !start_time_sec.is_finite() || !end_time_sec.is_finite() {
        return Err(SignalError::NonFiniteInput);
    }
    if start_time_sec >= end_time_sec {
        return Err(SignalError::InvalidWindowSize(0));
    }
    if !config.window_duration_sec.is_finite() || config.window_duration_sec <= 0.0 {
        return Err(SignalError::InvalidWindowSize(0));
    }
    if !config.step_sec.is_finite() || config.step_sec <= 0.0 {
        return Err(SignalError::InvalidWindowSize(0));
    }

    let mut windows = Vec::new();
    let mut current_start = start_time_sec;

    // Small numerical threshold (1e-9) to account for floating point addition accumulation
    while current_start + config.window_duration_sec <= end_time_sec + 1e-9 {
        let current_end = current_start + config.window_duration_sec;
        windows.push(FeatureWindow {
            start_time_sec: current_start,
            end_time_sec: current_end,
            duration_sec: config.window_duration_sec,
        });
        current_start += config.step_sec;
    }

    Ok(windows)
}

/// Convert a physical time window $[t_{\text{start}}, t_{\text{end}})$ into discrete sample indices $[i_{\text{start}}, i_{\text{end}})$.
///
/// Under the sample timestamp convention $t_i = \text{offset\_sec} + \frac{i}{f_s}$, a sample $i$ lies inside $[t_{\text{start}}, t_{\text{end}})$
/// if and only if $t_{\text{start}} \le t_i < t_{\text{end}}$.
///
/// # Returns
/// - `Ok(Some((start_idx, end_idx)))` if there is a non-empty overlap with signal indices $[0, \text{signal\_len})$.
/// - `Ok(None)` if the window is outside signal bounds or contains zero samples.
/// - `Err(SignalError)` if sampling rate or offset is non-finite or invalid.
pub fn time_range_to_sample_range(
    start_time_sec: f64,
    end_time_sec: f64,
    sampling_rate: f64,
    offset_sec: f64,
    signal_len: usize,
) -> Result<Option<(usize, usize)>> {
    if !sampling_rate.is_finite() || sampling_rate <= 0.0 {
        return Err(SignalError::InvalidSamplingRate(sampling_rate));
    }
    if !offset_sec.is_finite() || !start_time_sec.is_finite() || !end_time_sec.is_finite() {
        return Err(SignalError::NonFiniteInput);
    }
    if start_time_sec >= end_time_sec || signal_len == 0 {
        return Ok(None);
    }

    let rel_start = start_time_sec - offset_sec;
    let rel_end = end_time_sec - offset_sec;

    let raw_start = (rel_start * sampling_rate).ceil();
    let start_idx = if raw_start <= 0.0 {
        0
    } else {
        raw_start as usize
    };

    let raw_end = (rel_end * sampling_rate).ceil();
    let end_idx = if raw_end <= 0.0 { 0 } else { raw_end as usize };

    let start_clamped = start_idx.min(signal_len);
    let end_clamped = end_idx.min(signal_len);

    if start_clamped < end_clamped {
        Ok(Some((start_clamped, end_clamped)))
    } else {
        Ok(None)
    }
}

/// Monotonic range cursor for advancing through sorted event series across sliding windows.
#[derive(Debug, Clone, Default)]
pub struct EventCursor {
    start_idx: usize,
    end_idx: usize,
}

impl EventCursor {
    /// Create a new event cursor initialized at index 0.
    pub fn new() -> Self {
        Self {
            start_idx: 0,
            end_idx: 0,
        }
    }

    /// Reset cursor indices to 0.
    pub fn reset(&mut self) {
        self.start_idx = 0;
        self.end_idx = 0;
    }

    /// Advance start and end indices monotonically to find the event range $[i_{\text{start}}, i_{\text{end}})$
    /// whose timestamps satisfy $start\_sec \le t(event) < end\_sec$.
    pub fn find_range<T>(
        &mut self,
        events: &[T],
        start_sec: f64,
        end_sec: f64,
        time_fn: impl Fn(&T) -> f64,
    ) -> (usize, usize) {
        let len = events.len();
        if self.start_idx > len {
            self.start_idx = len;
        }
        if self.end_idx < self.start_idx {
            self.end_idx = self.start_idx;
        }
        if self.end_idx > len {
            self.end_idx = len;
        }

        while self.start_idx < len && time_fn(&events[self.start_idx]) < start_sec {
            self.start_idx += 1;
        }

        if self.end_idx < self.start_idx {
            self.end_idx = self.start_idx;
        }
        while self.end_idx < len && time_fn(&events[self.end_idx]) < end_sec {
            self.end_idx += 1;
        }

        (self.start_idx, self.end_idx)
    }
}
