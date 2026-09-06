use crate::error::{Result, SignalError};
use crate::features::window::FeatureWindow;
use crate::hrv::time::hrv_rmssd;
use crate::multimodal::sync::sample_to_time;
use ndarray::Array1;

/// Cardiac physiological features extracted over a time window.
#[derive(Debug, Clone, PartialEq)]
pub struct CardiacFeatures {
    /// Mean heart rate in beats per minute (BPM)
    pub mean_hr_bpm: Option<f64>,
    /// Median heart rate in beats per minute (BPM)
    pub median_hr_bpm: Option<f64>,
    /// Standard deviation of NN/RR intervals (SDNN) in milliseconds (ms)
    pub sdnn_ms: Option<f64>,
    /// Root mean square of successive NN/RR differences (RMSSD) in milliseconds (ms)
    pub rmssd_ms: Option<f64>,
    /// Percentage of successive NN/RR interval differences $> 50\text{ ms}$ (pNN50)
    pub pnn50: Option<f64>,
    /// Mean NN/RR interval length in milliseconds (ms)
    pub rr_mean_ms: Option<f64>,
    /// Standard deviation of NN/RR intervals in milliseconds (ms)
    pub rr_std_ms: Option<f64>,
    /// Total count of detected ECG R-peaks intersecting the feature window
    pub beat_count: usize,
}

/// Extract cardiac physiological features from ECG R-peaks intersecting a feature window.
///
/// # Errors
/// Returns [`SignalError::InvalidSamplingRate`] if `sampling_rate` is $\le 0.0$ or non-finite.
/// Returns [`SignalError::NonFiniteInput`] if `offset_sec` is non-finite.
pub fn cardiac_features(
    r_peaks: &[usize],
    sampling_rate: f64,
    offset_sec: f64,
    window: &FeatureWindow,
) -> Result<CardiacFeatures> {
    if !sampling_rate.is_finite() || sampling_rate <= 0.0 {
        return Err(SignalError::InvalidSamplingRate(sampling_rate));
    }
    if !offset_sec.is_finite() {
        return Err(SignalError::NonFiniteInput);
    }

    let mut intersecting_times = Vec::new();
    for &idx in r_peaks {
        let t = sample_to_time(idx, sampling_rate, offset_sec)?;
        if t >= window.start_time_sec && t < window.end_time_sec {
            intersecting_times.push(t);
        }
    }

    let beat_count = intersecting_times.len();

    if beat_count < 2 {
        return Ok(CardiacFeatures {
            mean_hr_bpm: None,
            median_hr_bpm: None,
            sdnn_ms: None,
            rmssd_ms: None,
            pnn50: None,
            rr_mean_ms: None,
            rr_std_ms: None,
            beat_count,
        });
    }

    let mut rr_ms_vec = Vec::with_capacity(beat_count - 1);
    let mut bpm_vec = Vec::with_capacity(beat_count - 1);

    for i in 0..(beat_count - 1) {
        let diff_sec = intersecting_times[i + 1] - intersecting_times[i];
        if diff_sec > 0.0 {
            let rr_ms = diff_sec * 1000.0;
            rr_ms_vec.push(rr_ms);
            bpm_vec.push(60.0 / diff_sec);
        }
    }

    if rr_ms_vec.is_empty() {
        return Ok(CardiacFeatures {
            mean_hr_bpm: None,
            median_hr_bpm: None,
            sdnn_ms: None,
            rmssd_ms: None,
            pnn50: None,
            rr_mean_ms: None,
            rr_std_ms: None,
            beat_count,
        });
    }

    // Mean RR
    let rr_mean = rr_ms_vec.iter().sum::<f64>() / rr_ms_vec.len() as f64;

    // SDNN / RR std
    let variance = rr_ms_vec
        .iter()
        .map(|&x| (x - rr_mean).powi(2))
        .sum::<f64>()
        / rr_ms_vec.len() as f64;
    let sdnn = variance.sqrt();

    // RMSSD via lamina::hrv::time::hrv_rmssd
    let rmssd = hrv_rmssd(&Array1::from(rr_ms_vec.clone())).ok();

    // pNN50
    let pnn50 = if rr_ms_vec.len() >= 2 {
        let mut nn50_count = 0;
        for i in 0..(rr_ms_vec.len() - 1) {
            if (rr_ms_vec[i + 1] - rr_ms_vec[i]).abs() > 50.0 {
                nn50_count += 1;
            }
        }
        Some((nn50_count as f64 / (rr_ms_vec.len() - 1) as f64) * 100.0)
    } else {
        None
    };

    // Mean HR
    let mean_hr = bpm_vec.iter().sum::<f64>() / bpm_vec.len() as f64;

    // Median HR
    let mut sorted_bpm = bpm_vec.clone();
    sorted_bpm.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median_hr = if sorted_bpm.len() % 2 == 1 {
        sorted_bpm[sorted_bpm.len() / 2]
    } else {
        (sorted_bpm[sorted_bpm.len() / 2 - 1] + sorted_bpm[sorted_bpm.len() / 2]) / 2.0
    };

    Ok(CardiacFeatures {
        mean_hr_bpm: Some(mean_hr),
        median_hr_bpm: Some(median_hr),
        sdnn_ms: Some(sdnn),
        rmssd_ms: rmssd,
        pnn50,
        rr_mean_ms: Some(rr_mean),
        rr_std_ms: Some(sdnn),
        beat_count,
    })
}
