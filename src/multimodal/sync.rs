use crate::error::{Result, SignalError};

/// Enum representing the physiological sensor modalities in Lamina.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Modality {
    /// Electrocardiography
    Ecg,
    /// Photoplethysmography
    Ppg,
    /// Electrodermal Activity (Galvanic Skin Response)
    Eda,
    /// Respiration
    Rsp,
}

/// A discrete physiological event bound to physical time (seconds).
#[derive(Debug, Clone, PartialEq)]
pub struct TimedEvent {
    /// Sensor modality
    pub modality: Modality,
    /// Original 0-indexed sample position in raw signal array
    pub index: usize,
    /// Physical timestamp in seconds relative to recording start offset
    pub timestamp_sec: f64,
}

/// Convert a sample index to physical timestamp (seconds) given a sampling rate and offset.
///
/// # Formula
/// $$\text{timestamp\_sec} = \text{offset\_sec} + \frac{\text{index}}{F_s}$$
///
/// # Errors
/// Returns [`SignalError::InvalidSamplingRate`] if `sampling_rate` is $\le 0.0$ or non-finite.
/// Returns [`SignalError::NonFiniteInput`] if `offset_sec` is non-finite.
pub fn sample_to_time(index: usize, sampling_rate: f64, offset_sec: f64) -> Result<f64> {
    if !sampling_rate.is_finite() || sampling_rate <= 0.0 {
        return Err(SignalError::InvalidSamplingRate(sampling_rate));
    }
    if !offset_sec.is_finite() {
        return Err(SignalError::NonFiniteInput);
    }
    let timestamp = offset_sec + (index as f64 / sampling_rate);
    if !timestamp.is_finite() {
        return Err(SignalError::NonFiniteInput);
    }
    Ok(timestamp)
}
