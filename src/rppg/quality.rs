use crate::rppg::signal::OpticalSignal;

/// Multi-dimensional quality metrics for a temporal segment of an rPPG acquisition.
#[derive(Debug, Clone, PartialEq)]
pub struct RppgSegmentQuality {
    /// Segment start time in seconds
    pub start_sec: f64,
    /// Segment end time in seconds
    pub end_sec: f64,
    /// Overall composite quality score in $[0.0, 1.0]$
    pub overall: f64,
    /// ROI spatial validity quality score in $[0.0, 1.0]$
    pub roi_quality: f64,
    /// Motion artifact quality score in $[0.0, 1.0]$ ($1.0 = \text{no motion}$)
    pub motion_quality: f64,
    /// Illumination level and stability quality score in $[0.0, 1.0]$
    pub illumination_quality: f64,
    /// Waveform spectral/periodicity quality score in $[0.0, 1.0]$
    pub signal_quality: f64,
    /// Fraction of valid (non-missing) frames in segment
    pub valid_fraction: f64,
}

/// Recording-wide rPPG signal quality summary across all temporal segments.
#[derive(Debug, Clone, PartialEq)]
pub struct RppgQualitySummary {
    /// Overall composite quality score across recording in $[0.0, 1.0]$
    pub overall: f64,
    /// Fraction of recording duration meeting valid quality criteria
    pub valid_fraction: f64,
    /// Per-segment quality breakdown
    pub segments: Vec<RppgSegmentQuality>,
}

/// Assess ROI spatial validity quality based on pixel counts and minimum threshold.
pub fn assess_roi_quality(valid_pixel_counts: &[usize], min_pixels: usize) -> f64 {
    if valid_pixel_counts.is_empty() || min_pixels == 0 {
        return 0.0;
    }
    let mut valid_count = 0usize;
    let mut ratio_sum = 0.0f64;

    for &cnt in valid_pixel_counts {
        if cnt >= min_pixels {
            valid_count += 1;
            let ratio = (cnt as f64 / min_pixels as f64).min(2.0) / 2.0;
            ratio_sum += ratio;
        }
    }

    let fraction = valid_count as f64 / valid_pixel_counts.len() as f64;
    let mean_ratio = if valid_count > 0 {
        ratio_sum / valid_count as f64
    } else {
        0.0
    };

    (0.5 * fraction + 0.5 * mean_ratio).clamp(0.0, 1.0)
}

/// Assess motion quality based on frame displacement and intensity derivative variation.
pub fn assess_motion_quality(displacements: &[f64], intensity_deriv_std: f64) -> f64 {
    let mean_disp = if !displacements.is_empty() {
        displacements.iter().sum::<f64>() / displacements.len() as f64
    } else {
        0.0
    };

    // Exponential decay penalty for motion displacement and intensity variation
    let motion_penalty = 0.05 * mean_disp + 0.2 * intensity_deriv_std;
    (-motion_penalty).exp().clamp(0.0, 1.0)
}

/// Assess illumination quality checking for dark ROIs, saturation, and luminance jumps.
pub fn assess_illumination_quality(mean_intensities: &[f64]) -> f64 {
    if mean_intensities.is_empty() {
        return 0.0;
    }

    let mean_lum = mean_intensities.iter().sum::<f64>() / mean_intensities.len() as f64;

    // Penalty for very dark (< 15) or saturated (> 240) ROIs
    let range_score = if mean_lum < 15.0 {
        (mean_lum / 15.0).clamp(0.0, 1.0)
    } else if mean_lum > 240.0 {
        ((255.0 - mean_lum) / 15.0).clamp(0.0, 1.0)
    } else {
        1.0
    };

    // Luminance variance stability
    let var = mean_intensities
        .iter()
        .map(|x| (x - mean_lum).powi(2))
        .sum::<f64>()
        / mean_intensities.len() as f64;
    let std = var.sqrt();
    let stability_score = (-0.05 * std).exp().clamp(0.0, 1.0);

    (0.6 * range_score + 0.4 * stability_score).clamp(0.0, 1.0)
}

/// Assess pulse waveform spectral periodicity quality in the physiological pulse band.
pub fn assess_signal_quality(waveform: &[f64], sampling_rate: f64, band: (f64, f64)) -> f64 {
    if waveform.len() < 10 || sampling_rate <= 0.0 {
        return 0.0;
    }

    let n = waveform.len();
    let mean = waveform.iter().sum::<f64>() / n as f64;
    let var = waveform.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n as f64;
    if var <= 1e-12 {
        return 0.0; // Zero variance flatline signal
    }

    // Autocorrelation peak in physiological lag range (0.4 s to 1.33 s for 45-150 BPM)
    let min_lag = (0.4 * sampling_rate).round() as usize;
    let max_lag = ((1.0 / band.0) * sampling_rate).round() as usize;
    let max_lag = max_lag.min(n - 1);

    if min_lag >= max_lag || max_lag >= n {
        return 0.5;
    }

    let mut max_autocorr = 0.0f64;
    for lag in min_lag..=max_lag {
        let mut sum = 0.0f64;
        for i in 0..(n - lag) {
            sum += (waveform[i] - mean) * (waveform[i + lag] - mean);
        }
        let autocorr = sum / ((n - lag) as f64 * var);
        if autocorr > max_autocorr {
            max_autocorr = autocorr;
        }
    }

    max_autocorr.clamp(0.0, 1.0)
}

/// Compute segment quality breakdown and overall composite score.
pub fn evaluate_segment_quality(
    optical: &OpticalSignal,
    waveform: &[f64],
    displacements: &[f64],
    min_pixels: usize,
    band: (f64, f64),
) -> RppgSegmentQuality {
    let start_sec = optical.timestamps_sec.first().copied().unwrap_or(0.0);
    let end_sec = optical.timestamps_sec.last().copied().unwrap_or(0.0);

    let roi_q = assess_roi_quality(&optical.valid_pixel_counts, min_pixels);

    // Compute green channel intensity derivative standard deviation for motion assessment
    let green_deriv_std = if optical.green.len() > 1 {
        let deriv: Vec<f64> = optical
            .green
            .windows(2)
            .map(|w| (w[1] - w[0]).abs())
            .collect();
        let m = deriv.iter().sum::<f64>() / deriv.len() as f64;
        (deriv.iter().map(|x| (x - m).powi(2)).sum::<f64>() / deriv.len() as f64).sqrt()
    } else {
        0.0
    };

    let motion_q = assess_motion_quality(displacements, green_deriv_std);
    let illumination_q = assess_illumination_quality(&optical.green);

    let fs = optical.mean_sampling_rate().unwrap_or(30.0);
    let signal_q = assess_signal_quality(waveform, fs, band);

    let valid_count = optical
        .valid_pixel_counts
        .iter()
        .filter(|&&c| c >= min_pixels)
        .count();
    let valid_fraction = if !optical.valid_pixel_counts.is_empty() {
        valid_count as f64 / optical.valid_pixel_counts.len() as f64
    } else {
        0.0
    };

    // Composite weighted score: 25% ROI, 25% Motion, 25% Illumination, 25% Signal
    let overall =
        (0.25 * roi_q + 0.25 * motion_q + 0.25 * illumination_q + 0.25 * signal_q).clamp(0.0, 1.0);

    RppgSegmentQuality {
        start_sec,
        end_sec,
        overall,
        roi_quality: roi_q,
        motion_quality: motion_q,
        illumination_quality: illumination_q,
        signal_quality: signal_q,
        valid_fraction,
    }
}
