use ndarray::Array1;
use lamina::signal::smooth::signal_smooth_moving_average;
use lamina::signal::filter::signal_filter;
use lamina::signal::peaks::signal_findpeaks;
use lamina::ecg::clean::ecg_clean;
use lamina::ecg::peaks::ecg_findpeaks;
use lamina::ppg::clean::ppg_clean;
use lamina::ppg::peaks::ppg_findpeaks;
use lamina::eda::clean::eda_clean;
use lamina::eda::phasic::eda_phasic;
use lamina::eda::peaks::eda_findpeaks;
use lamina::rsp::clean::rsp_clean;
use lamina::rsp::peaks::rsp_findpeaks;
use lamina::hrv::intervals::peaks_to_intervals;
use lamina::hrv::time::{hrv_rmssd, hrv_mean_nn};
use lamina::complexity::entropy::sample_entropy;

#[flutter_rust_bridge::frb(init)]
pub fn init_app() {
    flutter_rust_bridge::setup_default_user_utils();
}

// SIGNAL
pub fn process_signal_smooth_moving_average(signal: Vec<f64>, window_size: usize) -> Vec<f64> {
    let nd = Array1::from_vec(signal);
    signal_smooth_moving_average(&nd, window_size).into_raw_vec()
}

pub fn process_signal_filter(signal: Vec<f64>, sampling_rate: f64, lowcut: Option<f64>, highcut: Option<f64>, order: usize) -> Vec<f64> {
    let nd = Array1::from_vec(signal);
    signal_filter(&nd, sampling_rate, lowcut, highcut, order).into_raw_vec()
}

pub fn process_signal_findpeaks(signal: Vec<f64>) -> Vec<bool> {
    let nd = Array1::from_vec(signal);
    signal_findpeaks(&nd).into_raw_vec()
}

// ECG
pub fn process_ecg_clean(signal: Vec<f64>, sampling_rate: f64, method: String) -> Vec<f64> {
    let nd_signal = Array1::from_vec(signal);
    ecg_clean(&nd_signal, sampling_rate, &method).into_raw_vec()
}

pub fn process_ecg_findpeaks(signal: Vec<f64>, sampling_rate: f64) -> Vec<bool> {
    let nd_signal = Array1::from_vec(signal);
    ecg_findpeaks(&nd_signal, sampling_rate).into_raw_vec()
}

// PPG
pub fn process_ppg_clean(signal: Vec<f64>, sampling_rate: f64) -> Vec<f64> {
    let nd_signal = Array1::from_vec(signal);
    ppg_clean(&nd_signal, sampling_rate).into_raw_vec()
}

pub fn process_ppg_findpeaks(signal: Vec<f64>, sampling_rate: f64) -> Vec<bool> {
    let nd_signal = Array1::from_vec(signal);
    ppg_findpeaks(&nd_signal, sampling_rate).into_raw_vec()
}

// EDA
pub fn process_eda_clean(signal: Vec<f64>, sampling_rate: f64) -> Vec<f64> {
    let nd = Array1::from_vec(signal);
    eda_clean(&nd, sampling_rate).into_raw_vec()
}

pub fn process_eda_phasic(signal: Vec<f64>, sampling_rate: f64) -> Vec<f64> {
    let nd = Array1::from_vec(signal);
    eda_phasic(&nd, sampling_rate).into_raw_vec()
}

pub fn process_eda_findpeaks(phasic_signal: Vec<f64>) -> Vec<bool> {
    let nd = Array1::from_vec(phasic_signal);
    eda_findpeaks(&nd).into_raw_vec()
}

// RSP
pub fn process_rsp_clean(signal: Vec<f64>, sampling_rate: f64) -> Vec<f64> {
    let nd = Array1::from_vec(signal);
    rsp_clean(&nd, sampling_rate).into_raw_vec()
}

pub fn process_rsp_findpeaks(cleaned_signal: Vec<f64>) -> Vec<bool> {
    let nd = Array1::from_vec(cleaned_signal);
    rsp_findpeaks(&nd).into_raw_vec()
}

// HRV
pub fn process_peaks_to_intervals(peaks: Vec<bool>, sampling_rate: f64) -> Vec<f64> {
    let nd = Array1::from_vec(peaks);
    peaks_to_intervals(&nd, sampling_rate).into_raw_vec()
}

pub fn process_hrv_rmssd(intervals: Vec<f64>) -> Option<f64> {
    let nd = Array1::from_vec(intervals);
    hrv_rmssd(&nd)
}

pub fn process_hrv_mean_nn(intervals: Vec<f64>) -> Option<f64> {
    let nd = Array1::from_vec(intervals);
    hrv_mean_nn(&nd)
}

// COMPLEXITY
pub fn process_sample_entropy(signal: Vec<f64>, m: usize, r: f64) -> f64 {
    let nd = Array1::from_vec(signal);
    sample_entropy(&nd, m, r)
}
