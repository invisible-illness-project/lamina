pub mod cardiac;
pub mod config;
pub mod coupling;
pub mod eda;
pub mod quality;
pub mod respiration;
pub mod window;

pub use cardiac::{CardiacFeatures, cardiac_features};
pub use config::{FeatureConfig, WindowConfig};
pub use coupling::{CouplingFeatures, coupling_features};
pub use eda::{EdaFeatures, eda_features};
pub use quality::{FeatureQuality, FeatureQualityIssue, evaluate_feature_quality};
pub use respiration::{RespirationFeatures, respiration_features};
pub use window::{FeatureWindow, generate_windows};

use crate::eda::ScrEvent;
use crate::error::Result;
use crate::multimodal::sync::sample_to_time;
use crate::rsp::RespirationCycle;
use ndarray::Array1;

/// Input container holding preprocessed modality outputs for windowed feature extraction.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MultimodalInput {
    /// ECG R-peak sample indices (optional)
    pub ecg_r_peaks: Option<Vec<usize>>,
    pub ecg_sampling_rate: f64,
    pub ecg_offset_sec: f64,

    /// EDA Tonic Skin Conductance Level signal (optional)
    pub eda_tonic: Option<Array1<f64>>,
    /// EDA Phasic Skin Conductance Response signal (optional)
    pub eda_phasic: Option<Array1<f64>>,
    /// EDA SCR events (optional)
    pub eda_scr_events: Option<Vec<ScrEvent>>,
    pub eda_sampling_rate: f64,
    pub eda_offset_sec: f64,

    /// RSP respiration cycles (optional)
    pub rsp_cycles: Option<Vec<RespirationCycle>>,
    pub rsp_sampling_rate: f64,
    pub rsp_offset_sec: f64,

    /// PPG systolic peak sample indices (optional)
    pub ppg_peaks: Option<Vec<usize>>,
    pub ppg_sampling_rate: f64,
    pub ppg_offset_sec: f64,
}

/// Unified windowed multimodal feature vector.
#[derive(Debug, Clone, PartialEq)]
pub struct MultimodalFeatureVector {
    /// Window physical start, end, and duration
    pub window: FeatureWindow,
    /// Cardiac features (HR, HRV)
    pub cardiac: CardiacFeatures,
    /// EDA features (Tonic, Phasic, SCR events)
    pub eda: EdaFeatures,
    /// Respiration features (Rate, Duration, Amplitude)
    pub respiration: RespirationFeatures,
    /// Cross-modal coupling features (RSA, Coupling, ECG-PPG delay, SCR associations)
    pub coupling: CouplingFeatures,
    /// Transparent rule-based feature quality summary
    pub quality: FeatureQuality,
}

/// Extract fixed-duration sliding feature vectors from multimodal inputs over the recording timeline.
#[allow(clippy::collapsible_if)]
pub fn extract_features(
    input: &MultimodalInput,
    config: &FeatureConfig,
) -> Result<Vec<MultimodalFeatureVector>> {
    // 1. Determine overall recording time bounds across present modalities
    let mut min_t = f64::MAX;
    let mut max_t = f64::MIN;

    if let Some(ref peaks) = input.ecg_r_peaks {
        if !peaks.is_empty() && input.ecg_sampling_rate > 0.0 {
            let t_first = sample_to_time(peaks[0], input.ecg_sampling_rate, input.ecg_offset_sec)?;
            let t_last = sample_to_time(
                peaks[peaks.len() - 1],
                input.ecg_sampling_rate,
                input.ecg_offset_sec,
            )?;
            min_t = min_t.min(t_first);
            max_t = max_t.max(t_last);
        }
    }

    if let Some(ref tonic) = input.eda_tonic {
        if !tonic.is_empty() && input.eda_sampling_rate > 0.0 {
            let t_first = input.eda_offset_sec;
            let t_last = input.eda_offset_sec + (tonic.len() as f64 / input.eda_sampling_rate);
            min_t = min_t.min(t_first);
            max_t = max_t.max(t_last);
        }
    }

    if let Some(ref cycles) = input.rsp_cycles {
        if !cycles.is_empty() && input.rsp_sampling_rate > 0.0 {
            let t_first = sample_to_time(
                cycles[0].inspiration_index,
                input.rsp_sampling_rate,
                input.rsp_offset_sec,
            )?;
            let t_last = sample_to_time(
                cycles[cycles.len() - 1].next_inspiration_index,
                input.rsp_sampling_rate,
                input.rsp_offset_sec,
            )?;
            min_t = min_t.min(t_first);
            max_t = max_t.max(t_last);
        }
    }

    if let Some(ref peaks) = input.ppg_peaks {
        if !peaks.is_empty() && input.ppg_sampling_rate > 0.0 {
            let t_first = sample_to_time(peaks[0], input.ppg_sampling_rate, input.ppg_offset_sec)?;
            let t_last = sample_to_time(
                peaks[peaks.len() - 1],
                input.ppg_sampling_rate,
                input.ppg_offset_sec,
            )?;
            min_t = min_t.min(t_first);
            max_t = max_t.max(t_last);
        }
    }

    if min_t >= max_t {
        return Ok(Vec::new());
    }

    // 2. Generate sliding feature windows
    let windows = generate_windows(min_t, max_t, &config.window)?;

    // 3. Process feature vectors per window
    let rec_dur = max_t - min_t;
    let mut feature_vectors = Vec::with_capacity(windows.len());

    for win in windows {
        let cardiac = if let Some(ref peaks) = input.ecg_r_peaks {
            cardiac_features(peaks, input.ecg_sampling_rate, input.ecg_offset_sec, &win)?
        } else {
            CardiacFeatures {
                mean_hr_bpm: None,
                median_hr_bpm: None,
                sdnn_ms: None,
                rmssd_ms: None,
                pnn50: None,
                rr_mean_ms: None,
                rr_std_ms: None,
                beat_count: 0,
            }
        };

        let eda = if let (Some(t), Some(p), Some(scrs)) =
            (&input.eda_tonic, &input.eda_phasic, &input.eda_scr_events)
        {
            eda_features(
                t,
                p,
                scrs,
                input.eda_sampling_rate,
                input.eda_offset_sec,
                &win,
            )?
        } else {
            EdaFeatures {
                mean_tonic_us: None,
                median_tonic_us: None,
                tonic_std_us: None,
                mean_phasic_us: None,
                phasic_std_us: None,
                scr_count: 0,
                scr_rate_per_min: None,
                mean_scr_amplitude_us: None,
                median_scr_amplitude_us: None,
                mean_scr_rise_time_sec: None,
            }
        };

        let respiration = if let Some(ref cycles) = input.rsp_cycles {
            respiration_features(cycles, input.rsp_sampling_rate, input.rsp_offset_sec, &win)?
        } else {
            RespirationFeatures {
                mean_rate_bpm: None,
                median_rate_bpm: None,
                rate_std_bpm: None,
                mean_cycle_duration_sec: None,
                cycle_count: 0,
                mean_amplitude: None,
                amplitude_std: None,
            }
        };

        let coupling = coupling_features(
            input.ecg_r_peaks.as_deref(),
            input.ecg_sampling_rate,
            input.ecg_offset_sec,
            input.ppg_peaks.as_deref(),
            input.ppg_sampling_rate,
            input.ppg_offset_sec,
            input.rsp_cycles.as_deref(),
            input.rsp_sampling_rate,
            input.rsp_offset_sec,
            input.eda_scr_events.as_deref(),
            input.eda_sampling_rate,
            input.eda_offset_sec,
            &win,
        )?;

        let quality = evaluate_feature_quality(
            &cardiac,
            &eda,
            &respiration,
            &coupling,
            &win,
            config,
            rec_dur,
        );

        if (config.require_cardiac && !quality.cardiac_valid)
            || (config.require_respiration && !quality.respiration_valid)
            || (config.require_eda && !quality.eda_valid)
        {
            continue;
        }

        feature_vectors.push(MultimodalFeatureVector {
            window: win,
            cardiac,
            eda,
            respiration,
            coupling,
            quality,
        });
    }

    Ok(feature_vectors)
}
