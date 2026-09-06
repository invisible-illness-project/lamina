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
