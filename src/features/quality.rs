use crate::features::cardiac::CardiacFeatures;
use crate::features::config::FeatureConfig;
use crate::features::coupling::CouplingFeatures;
use crate::features::eda::EdaFeatures;
use crate::features::respiration::RespirationFeatures;
use crate::features::window::FeatureWindow;

/// Detailed feature coverage assessment across present physiological modalities.
#[derive(Debug, Clone, PartialEq)]
pub struct FeatureCoverage {
    /// Mean usable window coverage ratio across all present input modalities in $[0.0, 1.0]$
    pub overall: f64,
    /// ECG signal/event coverage ratio in window $[0.0, 1.0]$ (`None` if ECG is unsupplied)
    pub ecg: Option<f64>,
    /// PPG signal/event coverage ratio in window $[0.0, 1.0]$ (`None` if PPG is unsupplied)
    pub ppg: Option<f64>,
    /// EDA signal/event coverage ratio in window $[0.0, 1.0]$ (`None` if EDA is unsupplied)
    pub eda: Option<f64>,
    /// Respiration signal/event coverage ratio in window $[0.0, 1.0]$ (`None` if RSP is unsupplied)
    pub rsp: Option<f64>,
}

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
    /// Window usable recording coverage ratio in $[0.0, 1.0]$ (alias to `modality_coverage.overall`)
    pub coverage: f64,
    /// Modality-specific coverage breakdown
    pub modality_coverage: FeatureCoverage,
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
#[allow(clippy::too_many_arguments)]
pub fn evaluate_feature_quality(
    cardiac: &CardiacFeatures,
    eda: &EdaFeatures,
    respiration: &RespirationFeatures,
    coupling: &CouplingFeatures,
    window: &FeatureWindow,
    config: &FeatureConfig,
    ecg_bounds: Option<(f64, f64)>,
    ppg_bounds: Option<(f64, f64)>,
    eda_bounds: Option<(f64, f64)>,
    rsp_bounds: Option<(f64, f64)>,
) -> FeatureQuality {
    let mut issues = Vec::new();

    let calc_mod_cov = |bounds: Option<(f64, f64)>| -> Option<f64> {
        bounds.map(|(m_start, m_end)| {
            if window.duration_sec <= 0.0 || m_start >= m_end {
                return 0.0;
            }
            let overlap_start = window.start_time_sec.max(m_start);
            let overlap_end = window.end_time_sec.min(m_end);
            let overlap = (overlap_end - overlap_start).max(0.0);
            (overlap / window.duration_sec).min(1.0)
        })
    };

    let ecg_cov = calc_mod_cov(ecg_bounds);
    let ppg_cov = calc_mod_cov(ppg_bounds);
    let eda_cov = calc_mod_cov(eda_bounds);
    let rsp_cov = calc_mod_cov(rsp_bounds);

    let mut sum_cov = 0.0;
    let mut count_cov = 0;
    for cov in [ecg_cov, ppg_cov, eda_cov, rsp_cov].iter().flatten() {
        sum_cov += cov;
        count_cov += 1;
    }

    let overall_cov = if count_cov > 0 {
        sum_cov / count_cov as f64
    } else {
        0.0
    };

    let modality_coverage = FeatureCoverage {
        overall: overall_cov,
        ecg: ecg_cov,
        ppg: ppg_cov,
        eda: eda_cov,
        rsp: rsp_cov,
    };

    if overall_cov < config.window.min_coverage {
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

    let total_features = 28;
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
        coverage: overall_cov,
        modality_coverage,
        cardiac_valid,
        eda_valid,
        respiration_valid,
        coupling_valid,
        usable_feature_count: usable_count,
        total_feature_count: total_features,
        issues,
    }
}
