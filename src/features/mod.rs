pub mod cardiac;
pub mod config;
pub mod coupling;
pub mod eda;
pub mod quality;
pub mod respiration;
pub mod window;

pub use cardiac::{CardiacFeatures, cardiac_features, cardiac_features_range};
pub use config::{FeatureConfig, WindowConfig};
pub use coupling::{CouplingFeatures, PrecomputedCoupling, coupling_features};
pub use eda::{EdaFeatures, eda_features};
pub use quality::{FeatureCoverage, FeatureQuality, FeatureQualityIssue, evaluate_feature_quality};
pub use respiration::{RespirationFeatures, respiration_features};
pub use window::{EventCursor, FeatureWindow, generate_windows, time_range_to_sample_range};

use crate::eda::ScrEvent;
use crate::error::{Result, SignalError};
use crate::multimodal::sync::sample_to_time;
use crate::rsp::RespirationCycle;
use ndarray::Array1;

/// Container bundling sample-indexed events with required timing metadata.
#[derive(Debug, Clone, PartialEq)]
pub struct TimedEvents<T> {
    /// Vector of sample-indexed event objects
    pub events: Vec<T>,
    /// Sampling rate in Hz ($f_s > 0$)
    pub sampling_rate: f64,
    /// Physical start offset in seconds
    pub offset_sec: f64,
}

impl<T> TimedEvents<T> {
    /// Construct a validated [`TimedEvents`] container.
    pub fn new(events: Vec<T>, sampling_rate: f64, offset_sec: f64) -> Result<Self> {
        if !sampling_rate.is_finite() || sampling_rate <= 0.0 {
            return Err(SignalError::InvalidSamplingRate(sampling_rate));
        }
        if !offset_sec.is_finite() {
            return Err(SignalError::NonFiniteInput);
        }
        Ok(Self {
            events,
            sampling_rate,
            offset_sec,
        })
    }
}

/// Container bundling continuous signal array with required timing metadata.
#[derive(Debug, Clone, PartialEq)]
pub struct TimedSignal {
    /// 1D signal data array
    pub data: Array1<f64>,
    /// Sampling rate in Hz ($f_s > 0$)
    pub sampling_rate: f64,
    /// Physical start offset in seconds
    pub offset_sec: f64,
}

impl TimedSignal {
    /// Construct a validated [`TimedSignal`] container.
    pub fn new(data: Array1<f64>, sampling_rate: f64, offset_sec: f64) -> Result<Self> {
        if !sampling_rate.is_finite() || sampling_rate <= 0.0 {
            return Err(SignalError::InvalidSamplingRate(sampling_rate));
        }
        if !offset_sec.is_finite() {
            return Err(SignalError::NonFiniteInput);
        }
        Ok(Self {
            data,
            sampling_rate,
            offset_sec,
        })
    }
}

/// Input container holding preprocessed modality outputs for windowed feature extraction.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MultimodalInput {
    /// ECG R-peak sample indices (optional)
    pub ecg_r_peaks: Option<TimedEvents<usize>>,
    /// EDA Tonic Skin Conductance Level signal (optional)
    pub eda_tonic: Option<TimedSignal>,
    /// EDA Phasic Skin Conductance Response signal (optional)
    pub eda_phasic: Option<TimedSignal>,
    /// EDA SCR events (optional)
    pub eda_scr_events: Option<TimedEvents<ScrEvent>>,
    /// RSP respiration cycles (optional)
    pub rsp_cycles: Option<TimedEvents<RespirationCycle>>,
    /// PPG systolic peak sample indices (optional)
    pub ppg_peaks: Option<TimedEvents<usize>>,
}

impl MultimodalInput {
    /// Create a new empty [`MultimodalInput`] container.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add ECG R-peak inputs with validation.
    pub fn with_ecg(
        mut self,
        r_peaks: Vec<usize>,
        sampling_rate: f64,
        offset_sec: f64,
    ) -> Result<Self> {
        self.ecg_r_peaks = Some(TimedEvents::new(r_peaks, sampling_rate, offset_sec)?);
        Ok(self)
    }

    /// Add PPG peak inputs with validation.
    pub fn with_ppg(
        mut self,
        peaks: Vec<usize>,
        sampling_rate: f64,
        offset_sec: f64,
    ) -> Result<Self> {
        self.ppg_peaks = Some(TimedEvents::new(peaks, sampling_rate, offset_sec)?);
        Ok(self)
    }

    /// Add EDA signals and SCR events with validation.
    pub fn with_eda(
        mut self,
        tonic: Array1<f64>,
        phasic: Array1<f64>,
        scr_events: Vec<ScrEvent>,
        sampling_rate: f64,
        offset_sec: f64,
    ) -> Result<Self> {
        if tonic.len() != phasic.len() {
            return Err(SignalError::DimensionMismatch);
        }
        self.eda_tonic = Some(TimedSignal::new(tonic, sampling_rate, offset_sec)?);
        self.eda_phasic = Some(TimedSignal::new(phasic, sampling_rate, offset_sec)?);
        self.eda_scr_events = Some(TimedEvents::new(scr_events, sampling_rate, offset_sec)?);
        Ok(self)
    }

    /// Add respiration cycles with validation.
    pub fn with_rsp(
        mut self,
        cycles: Vec<RespirationCycle>,
        sampling_rate: f64,
        offset_sec: f64,
    ) -> Result<Self> {
        self.rsp_cycles = Some(TimedEvents::new(cycles, sampling_rate, offset_sec)?);
        Ok(self)
    }
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

/// Helper function to compute physical time bounds $[t_{\text{start}}, t_{\text{end}}]$ for a modality.
fn get_modality_bounds<T>(
    events: Option<&TimedEvents<T>>,
    time_fn: impl Fn(&T) -> Result<f64>,
) -> Result<Option<(f64, f64)>> {
    if let Some(timed) = events {
        if timed.events.is_empty() {
            Ok(None)
        } else {
            let t_first = time_fn(&timed.events[0])?;
            let t_last = time_fn(&timed.events[timed.events.len() - 1])?;
            Ok(Some((t_first.min(t_last), t_first.max(t_last))))
        }
    } else {
        Ok(None)
    }
}

fn validate_sorted_slice<T>(events: &[T], key_fn: impl Fn(&T) -> f64) -> Result<()> {
    for window in events.windows(2) {
        if key_fn(&window[0]) > key_fn(&window[1]) {
            return Err(SignalError::UnsortedEvents);
        }
    }
    Ok(())
}

/// Extract fixed-duration sliding feature vectors from multimodal inputs using monotonic $O(N + W)$ range lookup.
#[allow(clippy::collapsible_if)]
pub fn extract_features(
    input: &MultimodalInput,
    config: &FeatureConfig,
) -> Result<Vec<MultimodalFeatureVector>> {
    // 1. Validate chronological sorting invariants for present event streams once
    if let Some(ref timed) = input.ecg_r_peaks {
        validate_sorted_slice(&timed.events, |&idx| idx as f64)?;
    }
    if let Some(ref timed) = input.ppg_peaks {
        validate_sorted_slice(&timed.events, |&idx| idx as f64)?;
    }
    if let Some(ref timed) = input.eda_scr_events {
        validate_sorted_slice(&timed.events, |e| e.peak_index as f64)?;
    }
    if let Some(ref timed) = input.rsp_cycles {
        validate_sorted_slice(&timed.events, |c| c.inspiration_index as f64)?;
    }

    let mut min_t = f64::MAX;
    let mut max_t = f64::MIN;

    let ecg_bounds = get_modality_bounds(input.ecg_r_peaks.as_ref(), |&idx| {
        sample_to_time(
            idx,
            input.ecg_r_peaks.as_ref().unwrap().sampling_rate,
            input.ecg_r_peaks.as_ref().unwrap().offset_sec,
        )
    })?;
    if let Some((t1, t2)) = ecg_bounds {
        min_t = min_t.min(t1);
        max_t = max_t.max(t2);
    }

    let ppg_bounds = get_modality_bounds(input.ppg_peaks.as_ref(), |&idx| {
        sample_to_time(
            idx,
            input.ppg_peaks.as_ref().unwrap().sampling_rate,
            input.ppg_peaks.as_ref().unwrap().offset_sec,
        )
    })?;
    if let Some((t1, t2)) = ppg_bounds {
        min_t = min_t.min(t1);
        max_t = max_t.max(t2);
    }

    let eda_bounds = if let (Some(t_sig), Some(p_sig)) = (&input.eda_tonic, &input.eda_phasic) {
        if t_sig.data.len() != p_sig.data.len() {
            return Err(SignalError::DimensionMismatch);
        }
        if !t_sig.data.is_empty() {
            let t1 = t_sig.offset_sec;
            let t2 = t_sig.offset_sec + (t_sig.data.len() as f64 / t_sig.sampling_rate);
            Some((t1, t2))
        } else {
            None
        }
    } else {
        None
    };
    if let Some((t1, t2)) = eda_bounds {
        min_t = min_t.min(t1);
        max_t = max_t.max(t2);
    }

    let rsp_bounds = get_modality_bounds(input.rsp_cycles.as_ref(), |c| {
        sample_to_time(
            c.inspiration_index,
            input.rsp_cycles.as_ref().unwrap().sampling_rate,
            input.rsp_cycles.as_ref().unwrap().offset_sec,
        )
    })?;
    if let Some((t1, t2)) = rsp_bounds {
        min_t = min_t.min(t1);
        max_t = max_t.max(t2);
    }

    if min_t >= max_t {
        return Ok(Vec::new());
    }

    let windows = generate_windows(min_t, max_t, &config.window)?;

    // 2. Precompute multimodal coupling observations once across recording
    let precomputed_coupling = PrecomputedCoupling::compute(
        input.ecg_r_peaks.as_ref().map(|e| e.events.as_slice()),
        input
            .ecg_r_peaks
            .as_ref()
            .map_or(100.0, |e| e.sampling_rate),
        input.ecg_r_peaks.as_ref().map_or(0.0, |e| e.offset_sec),
        input.ppg_peaks.as_ref().map(|e| e.events.as_slice()),
        input.ppg_peaks.as_ref().map_or(100.0, |e| e.sampling_rate),
        input.ppg_peaks.as_ref().map_or(0.0, |e| e.offset_sec),
        input.rsp_cycles.as_ref().map(|e| e.events.as_slice()),
        input.rsp_cycles.as_ref().map_or(100.0, |e| e.sampling_rate),
        input.rsp_cycles.as_ref().map_or(0.0, |e| e.offset_sec),
        input.eda_scr_events.as_ref().map(|e| e.events.as_slice()),
        input
            .eda_scr_events
            .as_ref()
            .map_or(100.0, |e| e.sampling_rate),
        input.eda_scr_events.as_ref().map_or(0.0, |e| e.offset_sec),
    )?;

    // 3. Initialize monotonic event cursors for O(N + W) window range lookup
    let mut ecg_cursor = EventCursor::new();
    let mut scr_cursor = EventCursor::new();
    let mut rsp_cursor = EventCursor::new();

    let mut pulse_delay_cursor = EventCursor::new();
    let mut cr_phase_cursor = EventCursor::new();
    let mut eda_assoc_cursor = EventCursor::new();

    let mut feature_vectors = Vec::with_capacity(windows.len());

    for win in windows {
        let (ecg_s, ecg_e) = if let Some(ref timed) = input.ecg_r_peaks {
            let fs = timed.sampling_rate;
            let off = timed.offset_sec;
            ecg_cursor.find_range(
                &timed.events,
                win.start_time_sec,
                win.end_time_sec,
                |&idx| sample_to_time(idx, fs, off).unwrap_or(-1.0),
            )
        } else {
            (0, 0)
        };

        let cardiac = if let Some(ref timed) = input.ecg_r_peaks {
            cardiac_features_range(
                &timed.events,
                ecg_s,
                ecg_e,
                timed.sampling_rate,
                timed.offset_sec,
                &win,
            )?
        } else {
            CardiacFeatures::empty()
        };

        let (scr_s, scr_e) = if let Some(ref timed) = input.eda_scr_events {
            let fs = timed.sampling_rate;
            let off = timed.offset_sec;
            scr_cursor.find_range(&timed.events, win.start_time_sec, win.end_time_sec, |e| {
                sample_to_time(e.peak_index, fs, off).unwrap_or(-1.0)
            })
        } else {
            (0, 0)
        };

        let eda = if let (Some(t_sig), Some(p_sig), Some(scrs)) =
            (&input.eda_tonic, &input.eda_phasic, &input.eda_scr_events)
        {
            let win_scrs = if scr_s < scr_e && scr_s < scrs.events.len() {
                &scrs.events[scr_s..scr_e.min(scrs.events.len())]
            } else {
                &[]
            };
            eda_features(
                &t_sig.data,
                &p_sig.data,
                win_scrs,
                t_sig.sampling_rate,
                t_sig.offset_sec,
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

        let (rsp_s, rsp_e) = if let Some(ref timed) = input.rsp_cycles {
            let fs = timed.sampling_rate;
            let off = timed.offset_sec;
            rsp_cursor.find_range(&timed.events, win.start_time_sec, win.end_time_sec, |c| {
                sample_to_time(c.inspiration_index, fs, off).unwrap_or(-1.0)
            })
        } else {
            (0, 0)
        };

        let respiration = if let Some(ref timed) = input.rsp_cycles {
            let win_cycles = if rsp_s < rsp_e && rsp_s < timed.events.len() {
                &timed.events[rsp_s..rsp_e.min(timed.events.len())]
            } else {
                &[]
            };
            respiration_features(win_cycles, timed.sampling_rate, timed.offset_sec, &win)?
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

        let delay_range = pulse_delay_cursor.find_range(
            &precomputed_coupling.pulse_delays,
            win.start_time_sec,
            win.end_time_sec,
            |(t, _)| *t,
        );
        let phase_range = cr_phase_cursor.find_range(
            &precomputed_coupling.cr_phases,
            win.start_time_sec,
            win.end_time_sec,
            |(t, _)| *t,
        );
        let assoc_range = eda_assoc_cursor.find_range(
            &precomputed_coupling.eda_assocs,
            win.start_time_sec,
            win.end_time_sec,
            |&t| t,
        );

        let coupling = precomputed_coupling.extract_for_window_range(
            input.ecg_r_peaks.as_ref().map(|e| e.events.as_slice()),
            (ecg_s, ecg_e),
            input
                .ecg_r_peaks
                .as_ref()
                .map_or(100.0, |e| e.sampling_rate),
            input.ecg_r_peaks.as_ref().map_or(0.0, |e| e.offset_sec),
            input.rsp_cycles.as_ref().map(|e| e.events.as_slice()),
            (rsp_s, rsp_e),
            input.rsp_cycles.as_ref().map_or(100.0, |e| e.sampling_rate),
            input.rsp_cycles.as_ref().map_or(0.0, |e| e.offset_sec),
            delay_range,
            phase_range,
            assoc_range,
            &win,
        )?;

        let quality = evaluate_feature_quality(
            &cardiac,
            &eda,
            &respiration,
            &coupling,
            &win,
            config,
            ecg_bounds,
            ppg_bounds,
            eda_bounds,
            rsp_bounds,
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

/// Naive reference implementation of feature extraction for equivalence oracle testing in test suites.
///
/// Intentionally performs a straightforward reference vector scan ($O(W \times N)$) for every window
/// to serve as an unoptimized numerical ground truth oracle against [`extract_features`].
#[allow(clippy::collapsible_if)]
pub fn extract_features_naive(
    input: &MultimodalInput,
    config: &FeatureConfig,
) -> Result<Vec<MultimodalFeatureVector>> {
    let mut min_t = f64::MAX;
    let mut max_t = f64::MIN;

    let ecg_bounds = get_modality_bounds(input.ecg_r_peaks.as_ref(), |&idx| {
        sample_to_time(
            idx,
            input.ecg_r_peaks.as_ref().unwrap().sampling_rate,
            input.ecg_r_peaks.as_ref().unwrap().offset_sec,
        )
    })?;
    if let Some((t1, t2)) = ecg_bounds {
        min_t = min_t.min(t1);
        max_t = max_t.max(t2);
    }

    let ppg_bounds = get_modality_bounds(input.ppg_peaks.as_ref(), |&idx| {
        sample_to_time(
            idx,
            input.ppg_peaks.as_ref().unwrap().sampling_rate,
            input.ppg_peaks.as_ref().unwrap().offset_sec,
        )
    })?;
    if let Some((t1, t2)) = ppg_bounds {
        min_t = min_t.min(t1);
        max_t = max_t.max(t2);
    }

    let eda_bounds = if let (Some(t_sig), Some(p_sig)) = (&input.eda_tonic, &input.eda_phasic) {
        if t_sig.data.len() != p_sig.data.len() {
            return Err(SignalError::DimensionMismatch);
        }
        if !t_sig.data.is_empty() {
            let t1 = t_sig.offset_sec;
            let t2 = t_sig.offset_sec + (t_sig.data.len() as f64 / t_sig.sampling_rate);
            Some((t1, t2))
        } else {
            None
        }
    } else {
        None
    };
    if let Some((t1, t2)) = eda_bounds {
        min_t = min_t.min(t1);
        max_t = max_t.max(t2);
    }

    let rsp_bounds = get_modality_bounds(input.rsp_cycles.as_ref(), |c| {
        sample_to_time(
            c.inspiration_index,
            input.rsp_cycles.as_ref().unwrap().sampling_rate,
            input.rsp_cycles.as_ref().unwrap().offset_sec,
        )
    })?;
    if let Some((t1, t2)) = rsp_bounds {
        min_t = min_t.min(t1);
        max_t = max_t.max(t2);
    }

    if min_t >= max_t {
        return Ok(Vec::new());
    }

    let windows = generate_windows(min_t, max_t, &config.window)?;

    let mut feature_vectors = Vec::with_capacity(windows.len());

    for win in windows {
        let cardiac = if let Some(ref timed) = input.ecg_r_peaks {
            cardiac_features(&timed.events, timed.sampling_rate, timed.offset_sec, &win)?
        } else {
            CardiacFeatures::empty()
        };

        let eda = if let (Some(t_sig), Some(p_sig), Some(scrs)) =
            (&input.eda_tonic, &input.eda_phasic, &input.eda_scr_events)
        {
            eda_features(
                &t_sig.data,
                &p_sig.data,
                &scrs.events,
                t_sig.sampling_rate,
                t_sig.offset_sec,
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

        let respiration = if let Some(ref timed) = input.rsp_cycles {
            respiration_features(&timed.events, timed.sampling_rate, timed.offset_sec, &win)?
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
            input.ecg_r_peaks.as_ref().map(|e| e.events.as_slice()),
            input
                .ecg_r_peaks
                .as_ref()
                .map_or(100.0, |e| e.sampling_rate),
            input.ecg_r_peaks.as_ref().map_or(0.0, |e| e.offset_sec),
            input.ppg_peaks.as_ref().map(|e| e.events.as_slice()),
            input.ppg_peaks.as_ref().map_or(100.0, |e| e.sampling_rate),
            input.ppg_peaks.as_ref().map_or(0.0, |e| e.offset_sec),
            input.rsp_cycles.as_ref().map(|e| e.events.as_slice()),
            input.rsp_cycles.as_ref().map_or(100.0, |e| e.sampling_rate),
            input.rsp_cycles.as_ref().map_or(0.0, |e| e.offset_sec),
            input.eda_scr_events.as_ref().map(|e| e.events.as_slice()),
            input
                .eda_scr_events
                .as_ref()
                .map_or(100.0, |e| e.sampling_rate),
            input.eda_scr_events.as_ref().map_or(0.0, |e| e.offset_sec),
            &win,
        )?;

        let quality = evaluate_feature_quality(
            &cardiac,
            &eda,
            &respiration,
            &coupling,
            &win,
            config,
            ecg_bounds,
            ppg_bounds,
            eda_bounds,
            rsp_bounds,
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
