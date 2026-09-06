use crate::api::error::SignalError;
use lamina::hrv::intervals::peaks_to_intervals as core_peaks_to_intervals;
use lamina::hrv::time::{hrv_mean_nn as core_hrv_mean_nn, hrv_rmssd as core_hrv_rmssd};
use ndarray::Array1;

pub fn process_peaks_to_intervals(
    peaks: Vec<usize>,
    sampling_rate: f64,
    total_samples: usize,
) -> Result<Vec<f64>, SignalError> {
    let mut mask = Array1::<bool>::from_elem(total_samples, false);
    for idx in peaks {
        if idx < total_samples {
            mask[idx] = true;
        }
    }
    let intervals = core_peaks_to_intervals(&mask, sampling_rate)?;
    Ok(intervals.into_raw_vec_and_offset().0)
}

pub fn process_hrv_rmssd(intervals: Vec<f64>) -> Result<f64, SignalError> {
    let nd = Array1::from_vec(intervals);
    let val = core_hrv_rmssd(&nd)?;
    Ok(val)
}

pub fn process_hrv_mean_nn(intervals: Vec<f64>) -> Result<f64, SignalError> {
    let nd = Array1::from_vec(intervals);
    let val = core_hrv_mean_nn(&nd)?;
    Ok(val)
}

pub fn process_hrv_sdnn(intervals: Vec<f64>) -> Result<f64, SignalError> {
    let n = intervals.len();
    if n == 0 {
        return Err(SignalError::empty_signal());
    }
    if n < 2 {
        return Err(SignalError::insufficient_peaks(2, n));
    }
    for &val in &intervals {
        if !val.is_finite() {
            return Err(SignalError::non_finite_input());
        }
    }
    let mean = intervals.iter().sum::<f64>() / (n as f64);
    let var = intervals.iter().map(|&x| (x - mean).powi(2)).sum::<f64>() / (n as f64);
    Ok(var.sqrt())
}

pub fn process_hrv_pnn50(intervals: Vec<f64>) -> Result<f64, SignalError> {
    let n = intervals.len();
    if n < 2 {
        return Err(SignalError::insufficient_peaks(2, n));
    }
    for &val in &intervals {
        if !val.is_finite() {
            return Err(SignalError::non_finite_input());
        }
    }
    let mut count_50 = 0;
    for i in 0..(n - 1) {
        let diff = (intervals[i + 1] - intervals[i]).abs();
        if diff > 50.0 {
            count_50 += 1;
        }
    }
    Ok((count_50 as f64 / (n - 1) as f64) * 100.0)
}
