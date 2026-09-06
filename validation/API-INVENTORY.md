# Lamina Crate API Inventory (verbatim signatures from explore-agent report, verified against source)

Crate root: `lamina` v0.1.0, edition 2024 (Rust >= 1.85). Crate root re-exports: `pub use error::{Result, SignalError};` plus `pub mod autonomic, complexity, ecg, eda, features, hrv, multimodal, ppg, rppg, rsp, signal`.

## error
```rust
pub enum SignalError { EmptySignal, InvalidSamplingRate(f64), InvalidCutoffFrequency(String),
  InsufficientSamples { required: usize, provided: usize }, InvalidWindowSize(usize),
  InvalidFilterOrder(usize), NonFiniteInput, InsufficientPeaks { required: usize, provided: usize },
  DimensionMismatch, UnsortedEvents }
pub type Result<T> = std::result::Result<T, SignalError>;
```

## signal::filter
```rust
pub enum FilterKind { LowPass, HighPass, BandPass, Notch }
pub struct FilterSpec { pub kind, pub sampling_rate: f64, pub cutoffs: Vec<f64>, pub order: usize }
impl FilterSpec { lowpass(fs,cutoff,order); highpass(...); bandpass(fs,lowcut,highcut,order); notch(...); validate()->Result<()> }
pub struct SosSection { b0,b1,b2,a1,a2: f64 }  // Copy
pub fn design_butterworth_sos(spec: &FilterSpec) -> Result<Vec<SosSection>>
pub struct SosFilter { pub sections: Vec<SosSection> }
impl SosFilter { from_sections(...); from_spec(&FilterSpec)->Result<Self>;
  forward(&self, &Array1<f64>)->Result<Array1<f64>>; filtfilt(&self, &Array1<f64>)->Result<Array1<f64>> }
pub fn signal_filtfilt(signal: &Array1<f64>, spec: &FilterSpec) -> Result<Array1<f64>>
pub fn signal_filter(signal: &Array1<f64>, sampling_rate: f64, lowcut: Option<f64>, highcut: Option<f64>, order: usize) -> Result<Array1<f64>>
```
filtfilt: SciPy-style odd padding padlen=3*ntaps; InsufficientSamples when n <= padlen.

## signal::peaks
```rust
pub struct PeakDetectionConfig { min_height: Option<f64>, min_distance: Option<usize>, min_prominence: Option<f64>, min_width: Option<usize>, threshold: Option<f64> }  // builder: with_*
pub fn signal_findpeaks_config(signal: &Array1<f64>, config: &PeakDetectionConfig) -> Result<Vec<usize>>
pub fn signal_findpeaks_mask(signal: &Array1<f64>, config: &PeakDetectionConfig) -> Result<Array1<bool>>
pub fn signal_findpeaks(signal: &Array1<f64>) -> Result<Array1<bool>>
```

## signal::smooth
`pub fn signal_smooth_moving_average(signal: &Array1<f64>, window_size: usize) -> Result<Array1<f64>>` (centered, truncated at edges)

## ecg
```rust
pub fn ecg_clean(signal: &Array1<f64>, sampling_rate: f64, _method: &str) -> Result<Array1<f64>>  // method IGNORED; hardcoded 0.5Hz HP order5
pub struct EcgPeakDetectionConfig { lowcut: Option<f64>/*5.0*/, highcut: Option<f64>/*15.0*/, filter_order: Option<usize>/*2*/,
  integration_window_sec: Option<f64>/*0.150*/, refractory_period_sec: Option<f64>/*0.200*/, searchback: Option<bool>/*true*/, threshold_multiplier: Option<f64>/*0.25*/ }  // builder with_*
pub fn ecg_findpeaks_config(signal: &Array1<f64>, fs: f64, config: &EcgPeakDetectionConfig) -> Result<Vec<usize>>
pub fn ecg_findpeaks_mask(signal: &Array1<f64>, fs: f64, config: &EcgPeakDetectionConfig) -> Result<Array1<bool>>
pub fn ecg_findpeaks(signal: &Array1<f64>, fs: f64) -> Result<Array1<bool>>
```
Pan-Tompkins pipeline. Peak indices = 0-based sample indices, fine-aligned to max of bandpassed signal.

## ppg
```rust
pub fn ppg_clean(signal: &Array1<f64>, sampling_rate: f64) -> Result<Array1<f64>>  // 0.5-8.0Hz BP order3
pub struct PpgPeakDetectionConfig { lowcut/*0.5*/, highcut/*8.0*/, filter_order/*3*/, w_peak_sec/*0.111*/, w_beat_sec/*0.667*/, alpha/*0.02*/, refractory_period_sec/*0.300*/ }  // all Option, builder with_*
pub fn ppg_findpeaks_config(signal:&Array1<f64>, fs:f64, config:&PpgPeakDetectionConfig)->Result<Vec<usize>>
pub fn ppg_findpeaks_mask(...)->Result<Array1<bool>>
pub fn ppg_findpeaks(signal:&Array1<f64>, fs:f64)->Result<Array1<bool>>
```
Elgendi 2013.

## eda
```rust
pub fn eda_clean(signal: &Array1<f64>, sampling_rate: f64) -> Result<Array1<f64>>  // zero-phase LP 5Hz order4
pub struct EdaDecompositionConfig { tonic_cutoff_hz: Option<f64>/*0.05*/, filter_order: Option<usize>/*2*/ }
pub struct EdaComponents { pub tonic: Array1<f64>, pub phasic: Array1<f64> }
pub fn eda_decompose(signal:&Array1<f64>, fs:f64, config:&EdaDecompositionConfig)->Result<EdaComponents>
pub fn eda_phasic(signal:&Array1<f64>, fs:f64)->Result<Array1<f64>>
pub struct EdaPeakDetectionConfig { min_amplitude: Option<f64>/*0.01 uS*/, min_prominence: Option<f64>/*0.005*/, min_distance_sec: Option<f64>/*1.0*/, min_rise_time_sec: Option<f64>/*0.1*/, max_rise_time_sec: Option<f64>/*5.0*/ }
pub struct ScrEvent { pub onset_index: usize, pub peak_index: usize, pub amplitude: f64, pub rise_time_sec: f64 }
pub fn eda_findpeaks_events(phasic:&Array1<f64>, fs:f64, config:&EdaPeakDetectionConfig)->Result<Vec<ScrEvent>>
pub fn eda_findpeaks_config(phasic:&Array1<f64>, fs:f64, config:&EdaPeakDetectionConfig)->Result<Vec<usize>>
pub fn eda_findpeaks(phasic:&Array1<f64>)->Result<Array1<bool>>  // HARDCODES fs=100
```

## rsp
```rust
pub struct RspCleaningConfig { lowcut: Option<f64>/*0.05*/, highcut: Option<f64>/*0.50*/, filter_order: Option<usize>/*3*/ }
pub fn rsp_clean_config(signal:&Array1<f64>, fs:f64, config:&RspCleaningConfig)->Result<Array1<f64>>
pub fn rsp_clean(signal:&Array1<f64>, fs:f64)->Result<Array1<f64>>
pub struct RspProcessingConfig { lowcut/*0.05*/, highcut/*0.50*/, filter_order/*3*/, min_breath_interval_sec/*1.2*/, max_breath_interval_sec/*12.0*/, min_amplitude/*0.05*/ }
pub struct RespirationCycle { pub inspiration_index: usize, pub expiration_index: usize, pub next_inspiration_index: usize,
  pub duration_sec: f64, pub respiratory_rate_bpm: f64, pub amplitude: f64 }
pub fn rsp_cycles_config(signal:&Array1<f64>, fs:f64, config:&RspProcessingConfig)->Result<Vec<RespirationCycle>>  // RE-CLEANS input internally
pub fn rsp_cycles(signal:&Array1<f64>, fs:f64)->Result<Vec<RespirationCycle>>
pub fn rsp_rate_config(signal:&Array1<f64>, fs:f64, config:&RspProcessingConfig)->Result<Array1<f64>>  // per-sample rate
pub fn rsp_rate(signal:&Array1<f64>, fs:f64)->Result<Array1<f64>>
pub fn rsp_findpeaks_config(cleaned:&Array1<f64>, fs:f64, config:&RspProcessingConfig)->Result<Vec<usize>>
pub fn rsp_findpeaks(cleaned:&Array1<f64>)->Result<Array1<bool>>  // HARDCODES fs=100
```

## hrv
```rust
pub fn peaks_to_intervals(peaks: &Array1<bool>, sampling_rate: f64) -> Result<Array1<f64>>  // ms
pub fn hrv_rmssd(intervals: &Array1<f64>) -> Result<f64>  // ms
pub fn hrv_mean_nn(intervals: &Array1<f64>) -> Result<f64>  // ms
```

## features (windowed multimodal feature extraction)
```rust
pub struct FeatureWindow { pub start_time_sec: f64, pub end_time_sec: f64, pub duration_sec: f64 }
pub struct WindowConfig { window_duration_sec: f64/*60*/, step_sec: f64/*30*/, min_coverage: f64/*0.80*/ }
pub struct FeatureConfig { window: WindowConfig, min_beats: usize/*10*/, min_respiration_cycles: usize/*3*/, min_scr_events: usize/*0*/, require_cardiac: bool/*false*/, require_respiration: bool, require_eda: bool }
pub fn generate_windows(start_time_sec: f64, end_time_sec: f64, config: &WindowConfig) -> Result<Vec<FeatureWindow>>
pub struct TimedEvents<T> { pub events: Vec<T>, pub sampling_rate: f64, pub offset_sec: f64 }
pub struct TimedSignal { pub data: Array1<f64>, pub sampling_rate: f64, pub offset_sec: f64 }
pub struct MultimodalInput { ecg_r_peaks: Option<TimedEvents<usize>>, eda_tonic, eda_phasic, eda_scr_events, rsp_cycles, ppg_peaks }
impl MultimodalInput { new(); with_ecg(Vec<usize>, fs, offset)->Result<Self>; with_ppg(...); with_eda(tonic, phasic, Vec<ScrEvent>, fs, offset); with_rsp(Vec<RespirationCycle>, fs, offset) }
pub struct CardiacFeatures { mean_hr_bpm, median_hr_bpm, sdnn_ms, rmssd_ms, pnn50, rr_mean_ms, rr_std_ms: Option<f64>, beat_count: usize }
pub fn cardiac_features(r_peaks:&[usize], fs:f64, offset_sec:f64, window:&FeatureWindow)->Result<CardiacFeatures>
pub struct EdaFeatures { mean_tonic_us, median_tonic_us, tonic_std_us, mean_phasic_us, phasic_std_us: Option<f64>, scr_count: usize, scr_rate_per_min, mean_scr_amplitude_us, median_scr_amplitude_us, mean_scr_rise_time_sec }
pub fn eda_features(tonic:&Array1<f64>, phasic:&Array1<f64>, scr_events:&[ScrEvent], fs:f64, offset:f64, window:&FeatureWindow)->Result<EdaFeatures>
pub struct RespirationFeatures { mean_rate_bpm, median_rate_bpm, rate_std_bpm, mean_cycle_duration_sec: Option<f64>, cycle_count: usize, mean_amplitude, amplitude_std }
pub fn respiration_features(cycles:&[RespirationCycle], fs:f64, offset:f64, window:&FeatureWindow)->Result<RespirationFeatures>
pub struct CouplingFeatures { rsa_amplitude_bpm, rsa_amplitude_rr_sec, cardiac_respiratory_concentration, cardiac_respiratory_mean_phase, mean_pulse_delay_sec, pulse_delay_std_sec: Option<f64>, scr_cardiac_association_count: usize }
pub struct MultimodalFeatureVector { window, cardiac, eda, respiration, coupling, quality: FeatureQuality }
pub fn extract_features(input: &MultimodalInput, config: &FeatureConfig) -> Result<Vec<MultimodalFeatureVector>>
```

## autonomic
```rust
pub struct AutonomicBaseline { ...12 BaselineFeatureStats fields... }
impl AutonomicBaseline { pub fn fit(feature_series: &[MultimodalFeatureVector], config: &NormalizationConfig) -> Result<Self> }
pub struct AutonomicEstimator { pub config: AutonomicEstimatorConfig }
impl AutonomicEstimator { new(config); estimate(&self, fv:&MultimodalFeatureVector, baseline:&AutonomicBaseline)->Result<AutonomicState>;
  estimate_series(&self, &[MultimodalFeatureVector], &AutonomicBaseline)->Result<AutonomicStateSeries> }  // requires uniform window spacing
pub struct AutonomicState { timestamp: f64, duration_sec: f64, cardiac: CardiacState, electrodermal: ElectrodermalState,
  respiratory: RespiratoryState, coupling: CouplingState, activation_score: Option<f64>, regulation_score: Option<f64>, confidence: StateConfidence }
```

## multimodal
```rust
pub fn sample_to_time(index: usize, sampling_rate: f64, offset_sec: f64) -> Result<f64>  // offset + index/fs
pub fn rsa(r_peaks:&[usize], ecg_fs:f64, ecg_off:f64, rsp_cycles:&[RespirationCycle], rsp_fs:f64, rsp_off:f64)->Result<RsaResult>
pub struct RsaResult { amplitude_bpm: f64, amplitude_rr_sec: f64, valid_beats: usize, valid_cycles: usize }
pub fn cardiorespiratory_phase_coupling(phases:&[f64])->Result<PhaseCouplingResult>  // concentration, mean_phase, sample_count
pub fn ecg_ppg_timing(ecg_peaks:&[usize], ecg_fs:f64, ecg_off:f64, ppg_peaks:&[usize], ppg_fs:f64, ppg_off:f64)->Result<Vec<PulseTimingResult>>
pub fn eda_cardiorespiratory_association(scr_events, eda_fs, eda_off, r_peaks, ecg_fs, ecg_off, rsp_cycles, rsp_fs, rsp_off)->Result<Vec<ScrCardiorespiratoryAssociation>>
pub fn evaluate_ecg_quality(r_peaks:&[usize], fs:f64, signal_duration_sec:f64)->ModalityQuality
pub fn evaluate_rsp_quality(cycles_count: usize, signal_duration_sec: f64)->ModalityQuality
pub fn multimodal_quality(...)->Result<MultimodalQuality>
```

## rppg
```rust
pub struct VideoFrame { timestamp_sec: f64, width: usize, height: usize, channels: usize, data: Vec<u8> }  // RGB interleaved
impl VideoFrame { new(timestamp_sec, width, height, data)->Result<Self> }
pub struct VideoMetadata { nominal_fps: Option<f64>, frame_count: usize, duration_sec: f64, width, height }
pub struct VideoStream { frames: Vec<VideoFrame>, metadata: VideoMetadata }
impl VideoStream { new(frames, nominal_fps)->Result<Self>; validate_timing()->Result<()> }
pub struct Roi { x, y, width, height: usize }
impl Roi { new(x,y,w,h)->Result<Self>; validate_for_frame(fw,fh)->Result<()>; center()->(f64,f64) }
pub trait RoiProvider { fn roi_for_frame(&self, frame_index: usize, timestamp_sec: f64) -> Option<Roi>; fn displacement_at_frame(&self, frame_index)->f64 {0.0} }
pub struct StaticRoi { roi: Roi }  impl StaticRoi { new(roi)->Self }   // implements RoiProvider
pub struct TrackedRoiSeries { rois: Vec<Option<Roi>> }  // implements RoiProvider
pub struct RoiSample { timestamp_sec, red, green, blue: f64, valid_pixels: usize }
pub fn extract_roi_sample(frame:&VideoFrame, roi:&Roi)->Result<RoiSample>
pub struct OpticalSignal { timestamps_sec, red, green, blue: Vec<f64>, valid_pixel_counts: Vec<usize> }
impl OpticalSignal { from_samples(&[RoiSample])->Result<Self>; mean_sampling_rate()->Result<f64>; slice(start,len); preprocess(&RppgPreprocessingConfig) }
pub enum RppgAlgorithmId { GreenChannel, Chrom/*default*/, Pos }
pub struct RppgWindowConfig { window_sec: f64/*3.0*/, step_sec: f64/*0.5*/, min_window_fraction: f64/*0.8*/ }
pub struct RppgPreprocessingConfig { normalize_channels: bool/*true*/, detrend: bool/*true*/ }
pub struct RppgConfig { algorithm: RppgAlgorithmId, min_quality: f64/*0.4*/, minimum_roi_pixels: usize/*100*/, max_gap_sec: f64/*1.0*/,
  window: RppgWindowConfig, preprocessing: RppgPreprocessingConfig, signal_band_hz: (f64,f64)/*(0.75,2.5)*/ }
pub struct RppgSignal { timestamps_sec, waveform: Vec<f64>, sampling_rate_hz: f64, quality: RppgQualitySummary, algorithm: RppgAlgorithmId }
impl RppgSignal { to_ndarray(); valid_segments(max_gap_sec)->Result<Vec<RppgSegment>>; resample_uniform(target_fs, max_gap_sec)->Result<Self> }
pub struct RppgQualitySummary { overall: f64, valid_fraction: f64, segments: Vec<RppgSegmentQuality> }
pub fn extract_rppg(video:&VideoStream, roi_provider:&dyn RoiProvider, config:&RppgConfig)->Result<RppgSignal>
// Downstream: feed RppgSignal waveform into ppg_clean/ppg_findpeaks for HR.
```

## complexity
`pub fn sample_entropy(signal: &Array1<f64>, m: usize, r: f64) -> Result<f64>`  // O(N^2), returns INFINITY if no matches

## Sharp edges (candidate BUGS.md entries; verify before documenting)
1. `ecg_clean(signal, fs, method)` — `method` arg silently ignored (always 0.5 Hz HP order 5). [API footgun]
2. `eda_findpeaks(phasic)` and `rsp_findpeaks(cleaned)` hardcode fs=100 Hz — silently wrong for other rates. [Suspected defect]
3. `rsp_cycles_config` re-cleans input internally even if already cleaned (double filtering). [Behavioral]
4. `sample_entropy` returns Ok(INFINITY) on zero matches instead of error. [Behavioral]
5. `features::quality::FeatureQuality.total_feature_count` hardcoded 30. [Informational]
6. tests/filter_parity_tests.rs & peaks_parity_tests.rs panic if golden JSON missing (only golden_ecg.json checked in). [Test infra]
7. `lamina_dart/rust/src/api/simple.rs` likely does not compile (`.into_raw_vec()` on Result). [Out of main crate scope]
8. `multimodal::quality::multimodal_quality` doc mentions InvalidSamplingRate but actually returns EmptySignal when all None. [Doc]
9. ecg/peaks.rs:217 partial_cmp().unwrap() panic-capable on NaN (inputs pre-validated finite). [Low]

## Baseline test status
cargo test baseline: see /tmp/cargo-test.log (filter/peaks parity suites panic w/o generated goldens — pre-existing).
