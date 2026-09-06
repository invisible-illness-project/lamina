use crate::multimodal::sync::sample_to_time;
use crate::rsp::RespirationCycle;
use std::f64::consts::PI;

/// Map a physical timestamp (seconds) into normalized continuous respiratory phase $\phi \in [0, 2\pi)$.
///
/// # Phase Convention
/// - **Inspiration Phase $[0, \pi)$**:
///   $t \in [t_{\text{insp1}}, t_{\text{exp}}]$ maps linearly from $0$ (at inspiration peak) to $\pi$ (at expiration trough).
///   $$\phi = \pi \times \frac{t - t_{\text{insp1}}}{t_{\text{exp}} - t_{\text{insp1}}}$$
/// - **Expiration Phase $[\pi, 2\pi)$**:
///   $t \in (t_{\text{exp}}, t_{\text{insp2}}]$ maps linearly from $\pi$ (at expiration trough) to $2\pi$ (at next inspiration peak).
///   $$\phi = \pi + \pi \times \frac{t - t_{\text{exp}}}{t_{\text{insp2}} - t_{\text{exp}}}$$
///
/// # Returns
/// - `Some(phase_rad)` if `timestamp_sec` falls within a valid respiration cycle in `cycles`.
/// - `None` if `timestamp_sec` falls outside all cycles, if cycles are malformed, or if inputs are invalid/non-finite.
pub fn respiratory_phase_at_time(
    cycles: &[RespirationCycle],
    timestamp_sec: f64,
    rsp_sampling_rate: f64,
    rsp_offset_sec: f64,
) -> Option<f64> {
    if !timestamp_sec.is_finite()
        || !rsp_sampling_rate.is_finite()
        || rsp_sampling_rate <= 0.0
        || !rsp_offset_sec.is_finite()
        || cycles.is_empty()
    {
        return None;
    }

    for cycle in cycles {
        let t0 = sample_to_time(cycle.inspiration_index, rsp_sampling_rate, rsp_offset_sec).ok()?;
        let t1 = sample_to_time(cycle.expiration_index, rsp_sampling_rate, rsp_offset_sec).ok()?;
        let t2 = sample_to_time(
            cycle.next_inspiration_index,
            rsp_sampling_rate,
            rsp_offset_sec,
        )
        .ok()?;

        if t0 >= t1 || t1 >= t2 {
            continue;
        }

        if timestamp_sec >= t0 && timestamp_sec <= t2 {
            let phi = if timestamp_sec <= t1 {
                PI * (timestamp_sec - t0) / (t1 - t0)
            } else {
                PI + PI * (timestamp_sec - t1) / (t2 - t1)
            };

            // Clamp phase to [0, 2pi) to guard against floating-point precision boundary overhang
            let bounded_phi = if !(0.0..2.0 * PI).contains(&phi) {
                0.0
            } else {
                phi
            };

            return Some(bounded_phi);
        }
    }

    None
}
