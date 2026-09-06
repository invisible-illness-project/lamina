use crate::error::{Result, SignalError};
use std::f64::consts::PI;

/// Cardiorespiratory phase coupling circular statistics result.
#[derive(Debug, Clone, PartialEq)]
pub struct PhaseCouplingResult {
    /// Circular concentration measure (resultant vector length $R \in [0.0, 1.0]$).
    /// - $R \approx 1.0$: Strong phase alignment / concentration.
    /// - $R \approx 0.0$: Uniform / dispersed phase distribution.
    pub concentration: f64,
    /// Circular mean direction $\bar{\phi} \in [0, 2\pi)$ in radians.
    pub mean_phase: f64,
    /// Number of valid phase observations contributing to the coupling metric.
    pub sample_count: usize,
}

/// Compute cardiorespiratory phase coupling circular statistics for a slice of phase values $\phi_k$.
///
/// # Equations
/// $$\bar{C} = \frac{1}{N} \sum_{k=1}^N \cos(\phi_k), \quad \bar{S} = \frac{1}{N} \sum_{k=1}^N \sin(\phi_k)$$
/// $$R = \sqrt{\bar{C}^2 + \bar{S}^2}, \quad \bar{\phi} = \operatorname{atan2}(\bar{S}, \bar{C}) \pmod{2\pi}$$
///
/// # Errors
/// Returns [`SignalError::EmptySignal`] if `phases` is empty.
/// Returns [`SignalError::NonFiniteInput`] if any element in `phases` is non-finite.
pub fn cardiorespiratory_phase_coupling(phases: &[f64]) -> Result<PhaseCouplingResult> {
    if phases.is_empty() {
        return Err(SignalError::EmptySignal);
    }

    let mut sum_cos = 0.0;
    let mut sum_sin = 0.0;

    for &phi in phases {
        if !phi.is_finite() {
            return Err(SignalError::NonFiniteInput);
        }
        sum_cos += phi.cos();
        sum_sin += phi.sin();
    }

    let n = phases.len() as f64;
    let mean_cos = sum_cos / n;
    let mean_sin = sum_sin / n;

    let r_raw = (mean_cos * mean_cos + mean_sin * mean_sin).sqrt();
    // Protect against minor floating-point rounding precision overhang (e.g. 1.0000000000000002)
    let concentration = r_raw.clamp(0.0, 1.0);

    let mut mean_phase = mean_sin.atan2(mean_cos);
    if mean_phase < 0.0 {
        mean_phase += 2.0 * PI;
    }

    Ok(PhaseCouplingResult {
        concentration,
        mean_phase,
        sample_count: phases.len(),
    })
}
