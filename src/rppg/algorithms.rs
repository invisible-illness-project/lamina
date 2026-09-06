use crate::error::{Result, SignalError};
use crate::rppg::config::{RppgAlgorithmId, RppgConfig};
use crate::rppg::signal::OpticalSignal;

/// Common trait abstraction for rPPG optical pulse waveform extraction algorithms.
pub trait RppgAlgorithm: std::fmt::Debug {
    /// Return the algorithm identifier.
    fn id(&self) -> RppgAlgorithmId;

    /// Extract a single temporal segment pulse waveform from an `OpticalSignal` window.
    fn extract_window(&self, optical: &OpticalSignal, config: &RppgConfig) -> Result<Vec<f64>>;
}

/// Simple Green-channel intensity baseline pulse extraction algorithm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GreenAlgorithm;

impl RppgAlgorithm for GreenAlgorithm {
    fn id(&self) -> RppgAlgorithmId {
        RppgAlgorithmId::GreenChannel
    }

    fn extract_window(&self, optical: &OpticalSignal, _config: &RppgConfig) -> Result<Vec<f64>> {
        optical.validate()?;
        let n = optical.green.len();
        if n < 4 {
            return Err(SignalError::InsufficientSamples {
                required: 4,
                provided: n,
            });
        }

        let mean_g = optical.green.iter().sum::<f64>() / n as f64;
        if mean_g <= 1e-6 || !mean_g.is_finite() {
            return Err(SignalError::NonFiniteInput);
        }

        // Normalized green channel temporal trace: G_n = G / mean(G) - 1.0
        let waveform: Vec<f64> = optical.green.iter().map(|&g| g / mean_g - 1.0).collect();
        Ok(waveform)
    }
}
