use crate::error::{Result, SignalError};

/// Specific quality defect flags detected during rule-based assessment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QualityIssue {
    /// Signal array or detected event set is empty.
    EmptySignal,
    /// Event count is insufficient for statistical confidence.
    InsufficientEvents,
    /// Estimated heart rate falls outside plausible physiological bounds (30–220 BPM).
    UnplausibleHeartRate,
    /// Estimated respiratory rate falls outside plausible physiological bounds (3–45 BPM).
    UnplausibleRespirationRate,
    /// Excessive inter-event interval variability indicating severe motion artifact or false detections.
    ExtremeArtifact,
    /// Signal contains non-finite values (NaN or Inf).
    NonFiniteValues,
}

/// Transparent, rule-based quality evaluation for a single modality.
#[derive(Debug, Clone, PartialEq)]
pub struct ModalityQuality {
    /// Quality score normalized to $[0.0, 1.0]$ where $1.0$ is pristine.
    pub score: f64,
    /// Binary validity flag indicating score $\ge 0.50$ and absence of critical issues.
    pub valid: bool,
    /// List of specific identified quality defects.
    pub issues: Vec<QualityIssue>,
}

/// Consolidated cross-modal quality assessment.
#[derive(Debug, Clone, PartialEq)]
pub struct MultimodalQuality {
    /// ECG modality quality assessment
    pub ecg_quality: Option<ModalityQuality>,
    /// PPG modality quality assessment
    pub ppg_quality: Option<ModalityQuality>,
    /// EDA modality quality assessment
    pub eda_quality: Option<ModalityQuality>,
    /// RSP modality quality assessment
    pub rsp_quality: Option<ModalityQuality>,
    /// Mean overall multimodal quality score in $[0.0, 1.0]$
    pub overall_quality: f64,
}

/// Rule-based quality assessment for ECG R-peak detection.
pub fn evaluate_ecg_quality(
    r_peaks: &[usize],
    sampling_rate: f64,
    signal_duration_sec: f64,
) -> ModalityQuality {
    let mut issues = Vec::new();
    let mut score: f64 = 1.0;

    if r_peaks.is_empty() || signal_duration_sec <= 0.0 {
        return ModalityQuality {
            score: 0.0,
            valid: false,
            issues: vec![QualityIssue::EmptySignal],
        };
    }

    if r_peaks.len() < 3 {
        score -= 0.3;
        issues.push(QualityIssue::InsufficientEvents);
    }

    let mean_hr_bpm = (r_peaks.len() as f64 / signal_duration_sec) * 60.0;
    if !(30.0..=220.0).contains(&mean_hr_bpm) {
        score -= 0.4;
        issues.push(QualityIssue::UnplausibleHeartRate);
    }

    if r_peaks.len() >= 2 {
        let mut rr_secs = Vec::with_capacity(r_peaks.len() - 1);
        for i in 0..(r_peaks.len() - 1) {
            let rr = (r_peaks[i + 1] - r_peaks[i]) as f64 / sampling_rate;
            rr_secs.push(rr);
        }
        let mean_rr = rr_secs.iter().sum::<f64>() / rr_secs.len() as f64;
        let variance =
            rr_secs.iter().map(|&x| (x - mean_rr).powi(2)).sum::<f64>() / rr_secs.len() as f64;
        let sdrr = variance.sqrt();

        if mean_rr > 0.0 && (sdrr / mean_rr) > 0.50 {
            score -= 0.3;
            issues.push(QualityIssue::ExtremeArtifact);
        }
    }

    let final_score = score.clamp(0.0, 1.0);
    let valid = final_score >= 0.50 && !issues.contains(&QualityIssue::EmptySignal);

    ModalityQuality {
        score: final_score,
        valid,
        issues,
    }
}

/// Rule-based quality assessment for RSP respiratory cycles.
pub fn evaluate_rsp_quality(cycles_count: usize, signal_duration_sec: f64) -> ModalityQuality {
    let mut issues = Vec::new();
    let mut score: f64 = 1.0;

    if cycles_count == 0 || signal_duration_sec <= 0.0 {
        return ModalityQuality {
            score: 0.0,
            valid: false,
            issues: vec![QualityIssue::EmptySignal],
        };
    }

    let resp_rate_bpm = (cycles_count as f64 / signal_duration_sec) * 60.0;
    if !(3.0..=45.0).contains(&resp_rate_bpm) {
        score -= 0.4;
        issues.push(QualityIssue::UnplausibleRespirationRate);
    }

    if cycles_count < 2 {
        score -= 0.3;
        issues.push(QualityIssue::InsufficientEvents);
    }

    let final_score = score.clamp(0.0, 1.0);
    let valid = final_score >= 0.50 && !issues.contains(&QualityIssue::EmptySignal);

    ModalityQuality {
        score: final_score,
        valid,
        issues,
    }
}

/// Consolidate modality quality evaluations into a overall multimodal quality summary.
///
/// # Errors
/// Returns [`SignalError::InvalidSamplingRate`] if input bounds are invalid.
pub fn multimodal_quality(
    ecg_quality: Option<ModalityQuality>,
    ppg_quality: Option<ModalityQuality>,
    eda_quality: Option<ModalityQuality>,
    rsp_quality: Option<ModalityQuality>,
) -> Result<MultimodalQuality> {
    let mut scores = Vec::new();

    if let Some(ref q) = ecg_quality {
        scores.push(q.score);
    }
    if let Some(ref q) = ppg_quality {
        scores.push(q.score);
    }
    if let Some(ref q) = eda_quality {
        scores.push(q.score);
    }
    if let Some(ref q) = rsp_quality {
        scores.push(q.score);
    }

    if scores.is_empty() {
        return Err(SignalError::EmptySignal);
    }

    let overall = scores.iter().sum::<f64>() / scores.len() as f64;

    Ok(MultimodalQuality {
        ecg_quality,
        ppg_quality,
        eda_quality,
        rsp_quality,
        overall_quality: overall,
    })
}
