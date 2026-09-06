//! lamina_bridge — JSON in/out CLI bridge over the Lamina crate.
//!
//! Part of the Lamina validation framework (validation-only; this crate never
//! modifies Lamina). See validation/SPEC.md §3 for the protocol contract.
//!
//! Usage:
//!   lamina_bridge --op <OP> --input <in.json> --output <out.json>
//!   lamina_bridge version            (no input file; JSON to stdout)
//!
//! Exit codes: 0 ok · 1 Lamina Err · 2 panic caught · 3 protocol/IO error.

use std::fs;
use std::panic::{self, AssertUnwindSafe};
use std::process::ExitCode;

use lamina::complexity::entropy::sample_entropy;
use lamina::ecg::{EcgPeakDetectionConfig, ecg_clean, ecg_findpeaks_config};
use lamina::eda::{
    EdaDecompositionConfig, EdaPeakDetectionConfig, eda_clean, eda_decompose,
    eda_findpeaks_events,
};
use lamina::hrv::intervals::peaks_to_intervals;
use lamina::hrv::time::{hrv_mean_nn, hrv_rmssd};
use lamina::ppg::{PpgPeakDetectionConfig, ppg_clean, ppg_findpeaks_config};
use lamina::rppg::{
    ChromAlgorithm, GreenAlgorithm, OpticalSignal, PosAlgorithm, RoiSample, RppgAlgorithm,
    RppgAlgorithmId, RppgConfig,
};
use lamina::rsp::{RspCleaningConfig, RspProcessingConfig, rsp_clean_config, rsp_cycles_config, rsp_rate_config};
use lamina::signal::filter::{FilterSpec, SosFilter, signal_filtfilt};
use lamina::signal::peaks::{PeakDetectionConfig, signal_findpeaks_config};
use ndarray::Array1;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

const LAMINA_VERSION: &str = env!("LAMINA_VERSION");
const BRIDGE_VERSION: &str = env!("CARGO_PKG_VERSION");

// ---------------------------------------------------------------------------
// Envelopes
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, Default)]
struct Input {
    signal: Option<Vec<f64>>,
    sampling_rate: Option<f64>,
    config: Option<Value>,
    // hrv op
    peaks: Option<Vec<usize>>,
    signal_length: Option<usize>,
    // rppg-algorithm op
    timestamps_sec: Option<Vec<f64>>,
    red: Option<Vec<f64>>,
    green: Option<Vec<f64>>,
    blue: Option<Vec<f64>>,
    valid_pixel_counts: Option<Vec<usize>>,
}

#[derive(Debug, Serialize)]
struct Output {
    ok: bool,
    op: String,
    lamina_version: String,
    bridge_version: String,
    result: Option<Value>,
    error: Option<OpError>,
}

#[derive(Debug, Serialize)]
struct OpError {
    kind: &'static str,
    message: String,
}

impl OpError {
    fn bad_request(msg: impl Into<String>) -> Self {
        Self { kind: "bad_request", message: msg.into() }
    }
    fn lamina(err: lamina::SignalError) -> Self {
        Self { kind: "lamina_error", message: format!("{err}") }
    }
    fn panic(msg: impl Into<String>) -> Self {
        Self { kind: "panic", message: msg.into() }
    }
}

type OpResult = std::result::Result<Value, OpError>;

// ---------------------------------------------------------------------------
// Input helpers
// ---------------------------------------------------------------------------

fn arr(input: &Input) -> std::result::Result<Array1<f64>, OpError> {
    let v = input
        .signal
        .as_ref()
        .ok_or_else(|| OpError::bad_request("missing required field 'signal'"))?;
    Ok(Array1::from_vec(v.clone()))
}

fn fs(input: &Input) -> std::result::Result<f64, OpError> {
    input
        .sampling_rate
        .ok_or_else(|| OpError::bad_request("missing required field 'sampling_rate'"))
}

/// Deserialize the op-specific config object into `T` (absent config = all defaults).
fn cfg<T: serde::de::DeserializeOwned + Default>(input: &Input) -> std::result::Result<T, OpError> {
    match &input.config {
        None => Ok(T::default()),
        Some(v) => serde_json::from_value(v.clone())
            .map_err(|e| OpError::bad_request(format!("invalid config: {e}"))),
    }
}

fn sig_json(a: &Array1<f64>) -> Value {
    json!(a.to_vec())
}

// ---------------------------------------------------------------------------
// Config DTOs (absent field = Lamina default)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, Default)]
struct EcgCleanCfg {
    method: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct EcgPeaksCfg {
    lowcut: Option<f64>,
    highcut: Option<f64>,
    filter_order: Option<usize>,
    integration_window_sec: Option<f64>,
    refractory_period_sec: Option<f64>,
    searchback: Option<bool>,
    threshold_multiplier: Option<f64>,
    /// Bridge-only: echo the cleaned signal in the result.
    return_cleaned: Option<bool>,
}

#[derive(Debug, Deserialize, Default)]
struct PpgPeaksCfg {
    lowcut: Option<f64>,
    highcut: Option<f64>,
    filter_order: Option<usize>,
    w_peak_sec: Option<f64>,
    w_beat_sec: Option<f64>,
    alpha: Option<f64>,
    refractory_period_sec: Option<f64>,
}

#[derive(Debug, Deserialize, Default)]
struct EdaDecomposeCfg {
    tonic_cutoff_hz: Option<f64>,
    filter_order: Option<usize>,
}

#[derive(Debug, Deserialize, Default)]
struct EdaPeaksCfg {
    tonic_cutoff_hz: Option<f64>,
    filter_order: Option<usize>,
    min_amplitude: Option<f64>,
    min_prominence: Option<f64>,
    min_distance_sec: Option<f64>,
    min_rise_time_sec: Option<f64>,
    max_rise_time_sec: Option<f64>,
}

#[derive(Debug, Deserialize, Default)]
struct RspCleanCfg {
    lowcut: Option<f64>,
    highcut: Option<f64>,
    filter_order: Option<usize>,
}

#[derive(Debug, Deserialize, Default)]
struct RspCyclesCfg {
    lowcut: Option<f64>,
    highcut: Option<f64>,
    filter_order: Option<usize>,
    min_breath_interval_sec: Option<f64>,
    max_breath_interval_sec: Option<f64>,
    min_amplitude: Option<f64>,
}

#[derive(Debug, Deserialize, Default)]
struct SignalPeaksCfg {
    min_height: Option<f64>,
    min_distance: Option<usize>,
    min_prominence: Option<f64>,
    min_width: Option<usize>,
    threshold: Option<f64>,
}

#[derive(Debug, Deserialize, Default)]
struct FilterCfg {
    kind: Option<String>,
    cutoff: Option<f64>,
    cutoffs: Option<Vec<f64>>,
    order: Option<usize>,
    zero_phase: Option<bool>,
}

#[derive(Debug, Deserialize, Default)]
struct SampleEntropyCfg {
    m: Option<usize>,
    r: Option<f64>,
}

#[derive(Debug, Deserialize, Default)]
struct RppgCfg {
    algorithm: Option<String>,
    window_sec: Option<f64>,
    step_sec: Option<f64>,
    min_window_fraction: Option<f64>,
    normalize_channels: Option<bool>,
    detrend: Option<bool>,
}

// ---------------------------------------------------------------------------
// Config -> Lamina builder mapping
// ---------------------------------------------------------------------------

fn ecg_peak_cfg(c: &EcgPeaksCfg) -> EcgPeakDetectionConfig {
    let mut b = EcgPeakDetectionConfig::new();
    if let Some(v) = c.lowcut { b = b.with_lowcut(v); }
    if let Some(v) = c.highcut { b = b.with_highcut(v); }
    if let Some(v) = c.filter_order { b = b.with_filter_order(v); }
    if let Some(v) = c.integration_window_sec { b = b.with_integration_window_sec(v); }
    if let Some(v) = c.refractory_period_sec { b = b.with_refractory_period_sec(v); }
    if let Some(v) = c.searchback { b = b.with_searchback(v); }
    if let Some(v) = c.threshold_multiplier { b.threshold_multiplier = Some(v); }
    b
}

fn ppg_peak_cfg(c: &PpgPeaksCfg) -> PpgPeakDetectionConfig {
    let mut b = PpgPeakDetectionConfig::new();
    if let Some(v) = c.lowcut { b = b.with_lowcut(v); }
    if let Some(v) = c.highcut { b = b.with_highcut(v); }
    if let Some(v) = c.filter_order { b = b.with_filter_order(v); }
    if let Some(v) = c.w_peak_sec { b = b.with_w_peak_sec(v); }
    if let Some(v) = c.w_beat_sec { b = b.with_w_beat_sec(v); }
    if let Some(v) = c.alpha { b = b.with_alpha(v); }
    if let Some(v) = c.refractory_period_sec { b = b.with_refractory_period_sec(v); }
    b
}

fn eda_decompose_cfg(cutoff: Option<f64>, order: Option<usize>) -> EdaDecompositionConfig {
    let mut b = EdaDecompositionConfig::new();
    if let Some(v) = cutoff { b = b.with_tonic_cutoff_hz(v); }
    if let Some(v) = order { b = b.with_filter_order(v); }
    b
}

fn eda_peak_cfg(c: &EdaPeaksCfg) -> EdaPeakDetectionConfig {
    let mut b = EdaPeakDetectionConfig::new();
    if let Some(v) = c.min_amplitude { b = b.with_min_amplitude(v); }
    if let Some(v) = c.min_prominence { b = b.with_min_prominence(v); }
    if let Some(v) = c.min_distance_sec { b = b.with_min_distance_sec(v); }
    if let Some(v) = c.min_rise_time_sec { b = b.with_min_rise_time_sec(v); }
    if let Some(v) = c.max_rise_time_sec { b = b.with_max_rise_time_sec(v); }
    b
}

fn rsp_processing_cfg(c: &RspCyclesCfg) -> RspProcessingConfig {
    let mut b = RspProcessingConfig::new();
    if let Some(v) = c.lowcut { b = b.with_lowcut(v); }
    if let Some(v) = c.highcut { b = b.with_highcut(v); }
    if let Some(v) = c.filter_order { b = b.with_filter_order(v); }
    if let Some(v) = c.min_breath_interval_sec { b = b.with_min_breath_interval_sec(v); }
    if let Some(v) = c.max_breath_interval_sec { b = b.with_max_breath_interval_sec(v); }
    if let Some(v) = c.min_amplitude { b = b.with_min_amplitude(v); }
    b
}

// ---------------------------------------------------------------------------
// Ops (each returns the `result` payload; Lamina errors -> OpError::lamina)
// ---------------------------------------------------------------------------

fn op_version(_input: &Input) -> OpResult {
    Ok(json!({
        "lamina_version": LAMINA_VERSION,
        "bridge_version": BRIDGE_VERSION,
    }))
}

fn op_ecg_clean(input: &Input) -> OpResult {
    let c: EcgCleanCfg = cfg(input)?;
    let method = c.method.as_deref().unwrap_or("none");
    let out = ecg_clean(&arr(input)?, fs(input)?, method).map_err(OpError::lamina)?;
    Ok(json!({ "signal": sig_json(&out) }))
}

fn op_ecg_peaks(input: &Input) -> OpResult {
    let c: EcgPeaksCfg = cfg(input)?;
    let fs = fs(input)?;
    // Pipeline mirrors Lamina tests: clean first, then detect on the cleaned signal.
    let cleaned = ecg_clean(&arr(input)?, fs, "none").map_err(OpError::lamina)?;
    let peaks = ecg_findpeaks_config(&cleaned, fs, &ecg_peak_cfg(&c)).map_err(OpError::lamina)?;
    let mut out = json!({ "peaks": &peaks, "count": peaks.len() });
    if c.return_cleaned.unwrap_or(false) {
        out["cleaned"] = sig_json(&cleaned);
    }
    Ok(out)
}

fn op_ppg_clean(input: &Input) -> OpResult {
    let out = ppg_clean(&arr(input)?, fs(input)?).map_err(OpError::lamina)?;
    Ok(json!({ "signal": sig_json(&out) }))
}

fn op_ppg_peaks(input: &Input) -> OpResult {
    let c: PpgPeaksCfg = cfg(input)?;
    let fs = fs(input)?;
    let cleaned = ppg_clean(&arr(input)?, fs).map_err(OpError::lamina)?;
    let peaks = ppg_findpeaks_config(&cleaned, fs, &ppg_peak_cfg(&c)).map_err(OpError::lamina)?;
    Ok(json!({ "peaks": peaks, "count": peaks.len() }))
}

fn op_eda_clean(input: &Input) -> OpResult {
    let out = eda_clean(&arr(input)?, fs(input)?).map_err(OpError::lamina)?;
    Ok(json!({ "signal": sig_json(&out) }))
}

fn op_eda_decompose(input: &Input) -> OpResult {
    let c: EdaDecomposeCfg = cfg(input)?;
    let comp = eda_decompose(&arr(input)?, fs(input)?, &eda_decompose_cfg(c.tonic_cutoff_hz, c.filter_order))
        .map_err(OpError::lamina)?;
    Ok(json!({ "tonic": sig_json(&comp.tonic), "phasic": sig_json(&comp.phasic) }))
}

fn op_eda_peaks(input: &Input) -> OpResult {
    let c: EdaPeaksCfg = cfg(input)?;
    let fs = fs(input)?;
    let cleaned = eda_clean(&arr(input)?, fs).map_err(OpError::lamina)?;
    let comp = eda_decompose(&cleaned, fs, &eda_decompose_cfg(c.tonic_cutoff_hz, c.filter_order))
        .map_err(OpError::lamina)?;
    let events = eda_findpeaks_events(&comp.phasic, fs, &eda_peak_cfg(&c)).map_err(OpError::lamina)?;
    let peaks: Vec<usize> = events.iter().map(|e| e.peak_index).collect();
    let events_json: Vec<Value> = events
        .iter()
        .map(|e| json!({
            "onset_index": e.onset_index,
            "peak_index": e.peak_index,
            "amplitude": e.amplitude,
            "rise_time_sec": e.rise_time_sec,
        }))
        .collect();
    Ok(json!({ "events": events_json, "peaks": peaks, "count": events.len() }))
}

fn op_rsp_clean(input: &Input) -> OpResult {
    let c: RspCleanCfg = cfg(input)?;
    let mut b = RspCleaningConfig::new();
    if let Some(v) = c.lowcut { b = b.with_lowcut(v); }
    if let Some(v) = c.highcut { b = b.with_highcut(v); }
    if let Some(v) = c.filter_order { b = b.with_filter_order(v); }
    let out = rsp_clean_config(&arr(input)?, fs(input)?, &b).map_err(OpError::lamina)?;
    Ok(json!({ "signal": sig_json(&out) }))
}

fn op_rsp_cycles(input: &Input) -> OpResult {
    let c: RspCyclesCfg = cfg(input)?;
    let fs = fs(input)?;
    let raw = arr(input)?;
    // NOTE: rsp_cycles_config / rsp_rate_config re-clean internally; pass the raw signal.
    let rcfg = rsp_processing_cfg(&c);
    let cycles = rsp_cycles_config(&raw, fs, &rcfg).map_err(OpError::lamina)?;
    let rate = rsp_rate_config(&raw, fs, &rcfg).map_err(OpError::lamina)?;
    let cycles_json: Vec<Value> = cycles
        .iter()
        .map(|cy| json!({
            "inspiration_index": cy.inspiration_index,
            "expiration_index": cy.expiration_index,
            "next_inspiration_index": cy.next_inspiration_index,
            "duration_sec": cy.duration_sec,
            "respiratory_rate_bpm": cy.respiratory_rate_bpm,
            "amplitude": cy.amplitude,
        }))
        .collect();
    Ok(json!({ "cycles": cycles_json, "rate": sig_json(&rate), "count": cycles.len() }))
}

fn op_hrv(input: &Input) -> OpResult {
    let fs = fs(input)?;
    let peaks = input
        .peaks
        .as_ref()
        .ok_or_else(|| OpError::bad_request("missing required field 'peaks'"))?;
    let n = input
        .signal_length
        .ok_or_else(|| OpError::bad_request("missing required field 'signal_length'"))?;
    if peaks.iter().any(|&p| p >= n) {
        return Err(OpError::bad_request("peak index out of range for signal_length"));
    }
    let mut mask = Array1::from_elem(n, false);
    for &p in peaks {
        mask[p] = true;
    }
    // Fewer than 2 peaks -> empty intervals with null HRV (not an error; SPEC §3.2).
    let intervals = match peaks_to_intervals(&mask, fs) {
        Ok(v) => v,
        Err(lamina::SignalError::InsufficientPeaks { .. }) => Array1::<f64>::zeros(0),
        Err(e) => return Err(OpError::lamina(e)),
    };
    // Insufficient intervals -> null (not an error), per SPEC §3.2.
    let rmssd = hrv_rmssd(&intervals).ok();
    let mean_nn = hrv_mean_nn(&intervals).ok();
    Ok(json!({
        "intervals_ms": sig_json(&intervals),
        "n_intervals": intervals.len(),
        "rmssd_ms": rmssd,
        "mean_nn_ms": mean_nn,
    }))
}

fn op_signal_peaks(input: &Input) -> OpResult {
    let c: SignalPeaksCfg = cfg(input)?;
    let mut b = PeakDetectionConfig::new();
    if let Some(v) = c.min_height { b = b.with_min_height(v); }
    if let Some(v) = c.min_distance { b = b.with_min_distance(v); }
    if let Some(v) = c.min_prominence { b = b.with_min_prominence(v); }
    if let Some(v) = c.min_width { b = b.with_min_width(v); }
    if let Some(v) = c.threshold { b = b.with_threshold(v); }
    let peaks = signal_findpeaks_config(&arr(input)?, &b).map_err(OpError::lamina)?;
    Ok(json!({ "peaks": peaks, "count": peaks.len() }))
}

fn op_filter(input: &Input) -> OpResult {
    let c: FilterCfg = cfg(input)?;
    let fs = fs(input)?;
    let order = c.order.unwrap_or(2);
    let zero_phase = c.zero_phase.unwrap_or(true);
    let kind = c
        .kind
        .as_deref()
        .ok_or_else(|| OpError::bad_request("filter config requires 'kind' (lowpass|highpass|bandpass|notch)"))?;
    let spec = match kind {
        "lowpass" => {
            let fc = c.cutoff.ok_or_else(|| OpError::bad_request("lowpass requires 'cutoff'"))?;
            FilterSpec::lowpass(fs, fc, order)
        }
        "highpass" => {
            let fc = c.cutoff.ok_or_else(|| OpError::bad_request("highpass requires 'cutoff'"))?;
            FilterSpec::highpass(fs, fc, order)
        }
        "bandpass" | "notch" => {
            let cs = c
                .cutoffs
                .as_ref()
                .ok_or_else(|| OpError::bad_request(format!("{kind} requires 'cutoffs' [low, high]")))?;
            if cs.len() != 2 {
                return Err(OpError::bad_request(format!("{kind} requires exactly 2 cutoffs")));
            }
            if kind == "bandpass" {
                FilterSpec::bandpass(fs, cs[0], cs[1], order)
            } else {
                FilterSpec::notch(fs, cs[0], cs[1], order)
            }
        }
        other => return Err(OpError::bad_request(format!("unknown filter kind '{other}'"))),
    };
    let x = arr(input)?;
    let out = if zero_phase {
        signal_filtfilt(&x, &spec).map_err(OpError::lamina)?
    } else {
        SosFilter::from_spec(&spec).map_err(OpError::lamina)?.forward(&x).map_err(OpError::lamina)?
    };
    Ok(json!({ "signal": sig_json(&out) }))
}

fn op_sample_entropy(input: &Input) -> OpResult {
    let c: SampleEntropyCfg = cfg(input)?;
    let x = arr(input)?;
    let m = c.m.unwrap_or(2);
    let r = match c.r {
        Some(v) => v,
        None => {
            // Bridge convenience default: 0.2 * sample std (documented in SPEC §3.2).
            let n = x.len() as f64;
            let mean = x.iter().sum::<f64>() / n;
            let var = x.iter().map(|&v| (v - mean) * (v - mean)).sum::<f64>() / n;
            0.2 * var.sqrt()
        }
    };
    let se = sample_entropy(&x, m, r).map_err(OpError::lamina)?;
    Ok(json!({
        "sample_entropy": if se.is_finite() { Some(se) } else { None },
        "is_infinite": !se.is_finite(),
        "m": m,
        "r": r,
    }))
}

fn op_rppg_algorithm(input: &Input) -> OpResult {
    let c: RppgCfg = cfg(input)?;
    let ts = input.timestamps_sec.as_ref()
        .ok_or_else(|| OpError::bad_request("missing required field 'timestamps_sec'"))?;
    let (r, g, b) = (
        input.red.as_ref().ok_or_else(|| OpError::bad_request("missing 'red'"))?,
        input.green.as_ref().ok_or_else(|| OpError::bad_request("missing 'green'"))?,
        input.blue.as_ref().ok_or_else(|| OpError::bad_request("missing 'blue'"))?,
    );
    let n = ts.len();
    if r.len() != n || g.len() != n || b.len() != n {
        return Err(OpError::bad_request("timestamps/red/green/blue length mismatch"));
    }
    let vpc: Vec<usize> = match &input.valid_pixel_counts {
        Some(v) if v.len() == n => v.clone(),
        Some(_) => return Err(OpError::bad_request("valid_pixel_counts length mismatch")),
        None => vec![1000usize; n],
    };
    let samples: Vec<RoiSample> = (0..n)
        .map(|i| RoiSample {
            timestamp_sec: ts[i],
            red: r[i],
            green: g[i],
            blue: b[i],
            valid_pixels: vpc[i],
        })
        .collect();
    let optical = OpticalSignal::from_samples(&samples).map_err(OpError::lamina)?;
    let mean_fs = optical.mean_sampling_rate().map_err(OpError::lamina)?;

    // Lamina-default RppgConfig with caller overrides (algorithm selection + windowing).
    let mut rc = RppgConfig::default();
    let algo: Box<dyn RppgAlgorithm> = match c.algorithm.as_deref().unwrap_or("chrom") {
        "green" | "green_channel" => { rc.algorithm = RppgAlgorithmId::GreenChannel; Box::new(GreenAlgorithm) }
        "chrom" => { rc.algorithm = RppgAlgorithmId::Chrom; Box::new(ChromAlgorithm) }
        "pos" => { rc.algorithm = RppgAlgorithmId::Pos; Box::new(PosAlgorithm) }
        other => return Err(OpError::bad_request(format!("unknown rppg algorithm '{other}'"))),
    };
    if let Some(v) = c.window_sec { rc.window.window_sec = v; }
    if let Some(v) = c.step_sec { rc.window.step_sec = v; }
    if let Some(v) = c.min_window_fraction { rc.window.min_window_fraction = v; }
    if let Some(v) = c.normalize_channels { rc.preprocessing.normalize_channels = v; }
    if let Some(v) = c.detrend { rc.preprocessing.detrend = v; }
    rc.validate().map_err(OpError::lamina)?;

    let t_first = optical.timestamps_sec[0];
    let t_last = *optical.timestamps_sec.last().unwrap();

    let mut waveform: Vec<f64> = Vec::new();
    let mut out_ts: Vec<f64> = Vec::new();
    let mut n_windows = 0usize;
    let mut covered_until = f64::NEG_INFINITY; // last timestamp already emitted

    let mut win_start = t_first;
    while win_start < t_last || n_windows == 0 {
        let win_end = (win_start + rc.window.window_sec).min(t_last + 1e-9);
        let (s, e) = optical.timestamp_range(win_start, win_end).map_err(OpError::lamina)?;
        if e <= s {
            break;
        }
        let win = optical.slice(s, e - s).map_err(OpError::lamina)?;
        let win_ts = win.timestamps_sec.clone();
        let win_pre = win.preprocess(&rc.preprocessing).map_err(OpError::lamina)?;
        let w = algo.extract_window(&win_pre, &rc).map_err(OpError::lamina)?;
        if w.len() != win_ts.len() {
            return Err(OpError::panic(format!(
                "extract_window returned {} samples for a {}-sample window",
                w.len(), win_ts.len()
            )));
        }
        // Overlap-trimmed concatenation: emit each sample exactly once.
        for (t, v) in win_ts.iter().zip(w.iter()) {
            if *t > covered_until {
                out_ts.push(*t);
                waveform.push(*v);
                covered_until = *t;
            }
        }
        n_windows += 1;
        if win_end >= t_last {
            break;
        }
        win_start += rc.window.step_sec;
    }

    Ok(json!({
        "waveform": waveform,
        "timestamps_sec": out_ts,
        "algorithm": format!("{:?}", rc.algorithm),
        "n_windows": n_windows,
        "mean_sampling_rate_hz": mean_fs,
    }))
}

// ---------------------------------------------------------------------------
// Dispatch (with panic containment) + CLI
// ---------------------------------------------------------------------------

const OPS: &[(&str, fn(&Input) -> OpResult)] = &[
    ("version", op_version),
    ("ecg-clean", op_ecg_clean),
    ("ecg-peaks", op_ecg_peaks),
    ("ppg-clean", op_ppg_clean),
    ("ppg-peaks", op_ppg_peaks),
    ("eda-clean", op_eda_clean),
    ("eda-decompose", op_eda_decompose),
    ("eda-peaks", op_eda_peaks),
    ("rsp-clean", op_rsp_clean),
    ("rsp-cycles", op_rsp_cycles),
    ("hrv", op_hrv),
    ("signal-peaks", op_signal_peaks),
    ("filter", op_filter),
    ("sample-entropy", op_sample_entropy),
    ("rppg-algorithm", op_rppg_algorithm),
];

fn dispatch(op: &str, input: &Input) -> std::result::Result<OpResult, OpError> {
    let f = OPS
        .iter()
        .find(|(name, _)| *name == op)
        .map(|(_, f)| *f)
        .ok_or_else(|| {
            OpError::bad_request(format!(
                "unknown op '{op}'; known ops: {}",
                OPS.iter().map(|(n, _)| *n).collect::<Vec<_>>().join(", ")
            ))
        })?;
    // Panic containment: a Lamina panic must never abort the bridge without a
    // JSON error envelope (SPEC §3.1).
    let caught = panic::catch_unwind(AssertUnwindSafe(|| f(input)));
    Ok(match caught {
        Ok(res) => res,
        Err(payload) => {
            let msg = if let Some(s) = payload.downcast_ref::<&str>() {
                (*s).to_string()
            } else if let Some(s) = payload.downcast_ref::<String>() {
                s.clone()
            } else {
                "panic with non-string payload".to_string()
            };
            Err(OpError::panic(msg))
        }
    })
}

fn envelope(op: &str, result: Option<Value>, error: Option<OpError>) -> Output {
    Output {
        ok: error.is_none(),
        op: op.to_string(),
        lamina_version: LAMINA_VERSION.to_string(),
        bridge_version: BRIDGE_VERSION.to_string(),
        result,
        error,
    }
}

struct Cli {
    op: String,
    input_path: Option<String>,
    output_path: Option<String>,
}

fn parse_cli() -> std::result::Result<Cli, String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        return Err(format!(
            "usage: lamina_bridge --op <OP> --input <in.json> --output <out.json>\n       lamina_bridge version\nops: {}",
            OPS.iter().map(|(n, _)| *n).collect::<Vec<_>>().join(", ")
        ));
    }
    let mut op: Option<String> = None;
    let mut input_path: Option<String> = None;
    let mut output_path: Option<String> = None;
    let mut positional: Vec<String> = Vec::new();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--op" => { i += 1; op = args.get(i).cloned(); }
            "--input" | "-i" => { i += 1; input_path = args.get(i).cloned(); }
            "--output" | "-o" => { i += 1; output_path = args.get(i).cloned(); }
            other if other.starts_with("--op=") => op = Some(other[5..].to_string()),
            other if other.starts_with("--input=") => input_path = Some(other[8..].to_string()),
            other if other.starts_with("--output=") => output_path = Some(other[9..].to_string()),
            other => positional.push(other.to_string()),
        }
        i += 1;
    }
    if op.is_none() {
        op = positional.first().cloned();
        if positional.len() > 1 { input_path = input_path.or_else(|| Some(positional[1].clone())); }
        if positional.len() > 2 { output_path = output_path.or_else(|| Some(positional[2].clone())); }
    }
    let op = op.ok_or_else(|| "missing op".to_string())?;
    Ok(Cli { op, input_path, output_path })
}

fn main() -> ExitCode {
    let cli = match parse_cli() {
        Ok(c) => c,
        Err(msg) => {
            eprintln!("lamina_bridge: {msg}");
            return ExitCode::from(3);
        }
    };

    // Read + parse input (version op tolerates a missing input file).
    let input: Input = match &cli.input_path {
        Some(path) => match fs::read_to_string(path) {
            Ok(text) => match serde_json::from_str(&text) {
                Ok(v) => v,
                Err(e) => {
                    let out = envelope(&cli.op, None, Some(OpError::bad_request(format!("invalid input JSON: {e}"))));
                    write_output(&cli.output_path, &out);
                    return ExitCode::from(3);
                }
            },
            Err(e) => {
                let out = envelope(&cli.op, None, Some(OpError::bad_request(format!("cannot read input file '{path}': {e}"))));
                write_output(&cli.output_path, &out);
                return ExitCode::from(3);
            }
        },
        None => Input::default(),
    };

    match dispatch(&cli.op, &input) {
        Err(e) => {
            // bad_request (unknown op) -> exit 3
            let out = envelope(&cli.op, None, Some(e));
            write_output(&cli.output_path, &out);
            ExitCode::from(3)
        }
        Ok(Ok(result)) => {
            let out = envelope(&cli.op, Some(result), None);
            write_output(&cli.output_path, &out);
            ExitCode::SUCCESS
        }
        Ok(Err(e)) => {
            let code: u8 = match e.kind {
                "panic" => 2,
                "lamina_error" => 1,
                _ => 3,
            };
            let out = envelope(&cli.op, None, Some(e));
            write_output(&cli.output_path, &out);
            ExitCode::from(code)
        }
    }
}

fn write_output(path: &Option<String>, out: &Output) {
    let text = serde_json::to_string_pretty(out).expect("output envelope must serialize");
    match path {
        Some(p) => {
            if let Err(e) = fs::write(p, &text) {
                eprintln!("lamina_bridge: cannot write output file '{p}': {e}");
            }
        }
        None => println!("{text}"),
    }
}
