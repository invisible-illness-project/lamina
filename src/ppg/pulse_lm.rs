use crate::error::Result;
use crate::signal::dc::signal_remove_dc;
use crate::signal::filter::{FilterSpec, signal_filtfilt};
use crate::signal::normalize::{DegeneratePolicy, signal_minmax};
use crate::signal::resample::signal_resample;
use crate::signal::segment::{IncompleteTailPolicy, signal_segment_duration};
use ndarray::Array1;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Machine-readable specification for the PulseLM PPG standardized preprocessing pipeline.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PulseLmPipelineSpec {
    pub name: String,              // "PulseLM"
    pub version: String,           // "1.0"
    pub target_fs: f64,            // 125.0 Hz
    pub filter_cutoff_hz: f64,     // 8.0 Hz
    pub filter_order: usize,       // 4
    pub window_sec: f64,           // 10.0 s
    pub stride_sec: f64,           // 10.0 s
    pub tail_policy: String,       // "DropIncomplete"
    pub degenerate_policy: String, // "Midpoint"
}

impl Default for PulseLmPipelineSpec {
    fn default() -> Self {
        Self {
            name: "PulseLM".to_string(),
            version: "1.0".to_string(),
            target_fs: 125.0,
            filter_cutoff_hz: 8.0,
            filter_order: 4,
            window_sec: 10.0,
            stride_sec: 10.0,
            tail_policy: "DropIncomplete".to_string(),
            degenerate_policy: "Midpoint".to_string(),
        }
    }
}

impl PulseLmPipelineSpec {
    /// Compute deterministic SHA-256 hex string hash of this pipeline specification.
    pub fn compute_sha256_hash(&self) -> String {
        let json_str = serde_json::to_string(self).unwrap_or_default();
        let mut hasher = Sha256::new();
        hasher.update(json_str.as_bytes());
        format!("{:x}", hasher.finalize())
    }
}

/// PulseLM 5-Stage PPG Preprocessing Pipeline Orchestrator.
#[derive(Debug, Clone)]
pub struct PulseLmPipeline {
    pub spec: PulseLmPipelineSpec,
    pub tail_policy: IncompleteTailPolicy,
    pub degenerate_policy: DegeneratePolicy,
}

impl Default for PulseLmPipeline {
    fn default() -> Self {
        Self {
            spec: PulseLmPipelineSpec::default(),
            tail_policy: IncompleteTailPolicy::DropIncomplete,
            degenerate_policy: DegeneratePolicy::Midpoint,
        }
    }
}

impl PulseLmPipeline {
    pub fn new(
        spec: PulseLmPipelineSpec,
        tail_policy: IncompleteTailPolicy,
        degenerate_policy: DegeneratePolicy,
    ) -> Self {
        Self {
            spec,
            tail_policy,
            degenerate_policy,
        }
    }

    /// Process raw PPG waveform through the 5-stage standardized pipeline:
    ///
    /// 1. Resample to 125 Hz via polyphase FIR filter (`signal_resample`).
    /// 2. Apply 4th-order zero-phase Butterworth lowpass filter at 8 Hz (`signal_filtfilt`).
    /// 3. Partition waveform into 10-second window segments (`signal_segment_duration`).
    /// 4. Subtract sample mean per segment to remove DC baseline drift (`signal_remove_dc`).
    /// 5. Scale each segment to range [0.0, 1.0] (`signal_minmax`).
    pub fn process(&self, signal: &Array1<f64>, sampling_rate: f64) -> Result<Vec<Array1<f64>>> {
        // Stage 1: Resampling
        let resampled = signal_resample(signal, sampling_rate, self.spec.target_fs)?;

        // Stage 2: Low-pass Filtering
        let filter_spec = FilterSpec::lowpass(
            self.spec.target_fs,
            self.spec.filter_cutoff_hz,
            self.spec.filter_order,
        );
        let filtered = signal_filtfilt(&resampled, &filter_spec)?;

        // Stage 3: Window Segmentation
        let segments = signal_segment_duration(
            &filtered,
            self.spec.target_fs,
            self.spec.window_sec,
            self.spec.stride_sec,
            self.tail_policy,
        )?;

        // Stage 4 & 5: Per-segment DC removal + Min-Max Normalization
        let mut processed_segments = Vec::with_capacity(segments.len());
        for seg in segments {
            let dc_free = signal_remove_dc(&seg)?;
            let scaled = signal_minmax(&dc_free, (0.0, 1.0), self.degenerate_policy)?;
            processed_segments.push(scaled);
        }

        Ok(processed_segments)
    }
}

/// Standardized entry point function for PulseLM PPG preprocessing.
pub fn ppg_preprocess_pulselm(
    signal: &Array1<f64>,
    sampling_rate: f64,
) -> Result<Vec<Array1<f64>>> {
    PulseLmPipeline::default().process(signal, sampling_rate)
}
