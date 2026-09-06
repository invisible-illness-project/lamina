use crate::features::cardiac::CardiacFeatures;
use crate::features::config::FeatureConfig;
use crate::features::coupling::CouplingFeatures;
use crate::features::eda::EdaFeatures;
use crate::features::respiration::RespirationFeatures;
use crate::features::window::FeatureWindow;

/// Specific feature quality issues detected during windowed extraction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FeatureQualityIssue {
    /// Beat count is lower than required min_beats threshold
    InsufficientBeats,
    /// Respiration cycle count is lower than min_respiration_cycles threshold
    InsufficientRespirationCycles,
    /// SCR event count is lower than min_scr_events threshold
    InsufficientScrEvents,
    /// Window recording coverage is below min_coverage threshold
    LowCoverage,
    /// Modality input array or events are absent
    MissingModality,
    /// Invalid or non-finite inputs
    InvalidInput,
}

/// Transparent, rule-based quality assessment for extracted feature vectors.
#[derive(Debug, Clone, PartialEq)]
pub struct FeatureQuality {
    /// Window recording coverage ratio in $[0.0, 1.0]$
    pub coverage: f64,
    /// Cardiac features validity flag
    pub cardiac_valid: bool,
    /// EDA features validity flag
    pub eda_valid: bool,
    /// Respiration features validity flag
    pub respiration_valid: bool,
    /// Coupling features validity flag
    pub coupling_valid: bool,
    /// Count of usable (non-`None`) physiological features in this window
    pub usable_feature_count: usize,
    /// Total possible physiological feature count in schema
    pub total_feature_count: usize,
    /// List of specific identified feature quality defects
    pub issues: Vec<FeatureQualityIssue>,
}

/// Evaluate transparent rule-based feature quality for a feature window.
pub fn evaluate_feature_quality(
    cardiac: &CardiacFeatures,
    eda: &EdaFeatures,
    respiration: &RespirationFeatures,
    coupling: &CouplingFeatures,
    _window: &FeatureWindow,
    config: &FeatureConfig,
    recording_duration_sec: f64,
) -> FeatureQuality {
    let mut issues = Vec::new();

    let coverage = if recording_duration_sec > 0.0 {
        (_window.duration_sec / recording_duration_sec).min(1.0)
    } else {
        1.0
    };

    if coverage < config.window.min_coverage {
        issues.push(FeatureQualityIssue::LowCoverage);
    }

    let cardiac_valid = cardiac.beat_count >= config.min_beats;
    if !cardiac_valid && cardiac.beat_count > 0 {
        issues.push(FeatureQualityIssue::InsufficientBeats);
    }

    let respiration_valid = respiration.cycle_count >= config.min_respiration_cycles;
    if !respiration_valid && respiration.cycle_count > 0 {
        issues.push(FeatureQualityIssue::InsufficientRespirationCycles);
    }

    let eda_valid = eda.scr_count >= config.min_scr_events;
    if !eda_valid && eda.scr_count > 0 {
        issues.push(FeatureQualityIssue::InsufficientScrEvents);
    }

    let coupling_valid = cardiac_valid && respiration_valid;

    // Count non-None fields
    let total_features = 30; // Scheme total features
    let mut usable_count = 0;

    if cardiac.mean_hr_bpm.is_some() {
        usable_count += 1;
    }
    if cardiac.median_hr_bpm.is_some() {
        usable_count += 1;
    }
    if cardiac.sdnn_ms.is_some() {
        usable_count += 1;
    }
    if cardiac.rmssd_ms.is_some() {
        usable_count += 1;
    }
    if cardiac.pnn50.is_some() {
        usable_count += 1;
    }
    if cardiac.rr_mean_ms.is_some() {
        usable_count += 1;
    }
    if cardiac.rr_std_ms.is_some() {
        usable_count += 1;
    }

    if eda.mean_tonic_us.is_some() {
        usable_count += 1;
    }
    if eda.median_tonic_us.is_some() {
        usable_count += 1;
    }
    if eda.tonic_std_us.is_some() {
        usable_count += 1;
    }
    if eda.mean_phasic_us.is_some() {
        usable_count += 1;
    }
    if eda.phasic_std_us.is_some() {
        usable_count += 1;
    }
    if eda.scr_rate_per_min.is_some() {
        usable_count += 1;
    }
    if eda.mean_scr_amplitude_us.is_some() {
        usable_count += 1;
    }
    if eda.median_scr_amplitude_us.is_some() {
        usable_count += 1;
    }
    if eda.mean_scr_rise_time_sec.is_some() {
        usable_count += 1;
    }

    if respiration.mean_rate_bpm.is_some() {
        usable_count += 1;
    }
    if respiration.median_rate_bpm.is_some() {
        usable_count += 1;
    }
    if respiration.rate_std_bpm.is_some() {
        usable_count += 1;
    }
    if respiration.mean_cycle_duration_sec.is_some() {
        usable_count += 1;
    }
    if respiration.mean_amplitude.is_some() {
        usable_count += 1;
    }
    if respiration.amplitude_std.is_some() {
        usable_count += 1;
    }

    if coupling.rsa_amplitude_bpm.is_some() {
        usable_count += 1;
    }
    if coupling.rsa_amplitude_rr_sec.is_some() {
        usable_count += 1;
    }
    if coupling.cardiac_respiratory_concentration.is_some() {
        usable_count += 1;
    }
    if coupling.cardiac_respiratory_mean_phase.is_some() {
        usable_count += 1;
    }
    if coupling.mean_pulse_delay_sec.is_some() {
        usable_count += 1;
    }
    if coupling.pulse_delay_std_sec.is_some() {
        usable_count += 1;
    }

    FeatureQuality {
        coverage,
        cardiac_valid,
        eda_valid,
        respiration_valid,
        coupling_valid,
        usable_feature_count: usable_count,
        total_feature_count: total_features,
        issues,
    }
}
