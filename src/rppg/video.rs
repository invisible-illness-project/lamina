use crate::error::{Result, SignalError};

/// Single RGB video frame with explicit physical timestamping and pixel data.
///
/// # Layout & Conventions
/// - `channels`: Must be 3 (RGB).
/// - `data`: Row-major 8-bit unsigned integer pixel array ($[\text{R}_0, \text{G}_0, \text{B}_0, \text{R}_1, \text{G}_1, \text{B}_1, \dots]$).
/// - Length invariant: `data.len() == width * height * 3`.
#[derive(Debug, Clone, PartialEq)]
pub struct VideoFrame {
    /// Physical timestamp in seconds
    pub timestamp_sec: f64,
    /// Frame width in pixels
    pub width: usize,
    /// Frame height in pixels
    pub height: usize,
    /// Color channel count (must be 3 for RGB)
    pub channels: usize,
    /// Interleaved row-major 8-bit color pixel buffer
    pub data: Vec<u8>,
}

impl VideoFrame {
    /// Create and validate a new RGB `VideoFrame`.
    pub fn new(timestamp_sec: f64, width: usize, height: usize, data: Vec<u8>) -> Result<Self> {
        let frame = Self {
            timestamp_sec,
            width,
            height,
            channels: 3,
            data,
        };
        frame.validate()?;
        Ok(frame)
    }

    /// Validate frame data length, dimensions, channels, and timestamp finiteness.
    pub fn validate(&self) -> Result<()> {
        if !self.timestamp_sec.is_finite() {
            return Err(SignalError::NonFiniteInput);
        }
        if self.width == 0 || self.height == 0 {
            return Err(SignalError::InvalidWindowSize(0));
        }
        if self.channels != 3 {
            return Err(SignalError::DimensionMismatch);
        }
        let expected_bytes = self.width * self.height * self.channels;
        if self.data.len() != expected_bytes {
            return Err(SignalError::DimensionMismatch);
        }
        Ok(())
    }
}

/// Metadata summarizing video stream acquisition parameters.
#[derive(Debug, Clone, PartialEq)]
pub struct VideoMetadata {
    /// Optional nominal frames-per-second declared by video source
    pub nominal_fps: Option<f64>,
    /// Total valid frames in stream
    pub frame_count: usize,
    /// Total duration in seconds ($t_{\text{last}} - t_{\text{first}}$)
    pub duration_sec: f64,
    /// Frame width in pixels
    pub width: usize,
    /// Frame height in pixels
    pub height: usize,
}

/// Sequence of timestamped video frames comprising an rPPG acquisition stream.
#[derive(Debug, Clone, PartialEq)]
pub struct VideoStream {
    /// Sequential frames
    pub frames: Vec<VideoFrame>,
    /// Stream metadata summary
    pub metadata: VideoMetadata,
}

impl VideoStream {
    /// Construct a `VideoStream` from a sequence of frames, computing metadata and validating timing.
    pub fn new(frames: Vec<VideoFrame>, nominal_fps: Option<f64>) -> Result<Self> {
        if frames.is_empty() {
            return Err(SignalError::EmptySignal);
        }

        let width = frames[0].width;
        let height = frames[0].height;

        let stream = Self {
            frames: frames.clone(),
            metadata: VideoMetadata {
                nominal_fps,
                frame_count: frames.len(),
                duration_sec: if frames.len() > 1 {
                    frames.last().unwrap().timestamp_sec - frames[0].timestamp_sec
                } else {
                    0.0
                },
                width,
                height,
            },
        };

        stream.validate_timing()?;
        Ok(stream)
    }

    /// Validate complete stream timing and spatial dimension consistency.
    pub fn validate_timing(&self) -> Result<()> {
        if self.frames.is_empty() {
            return Err(SignalError::EmptySignal);
        }

        let first_frame = &self.frames[0];
        first_frame.validate()?;

        let mut prev_t = first_frame.timestamp_sec;

        for (i, frame) in self.frames.iter().enumerate().skip(1) {
            frame.validate()?;

            if frame.width != first_frame.width || frame.height != first_frame.height {
                return Err(SignalError::DimensionMismatch);
            }

            if frame.timestamp_sec <= prev_t {
                return Err(SignalError::UnsortedEvents);
            }

            let dt = frame.timestamp_sec - prev_t;
            if !dt.is_finite() || dt <= 0.0 {
                return Err(SignalError::NonFiniteInput);
            }

            prev_t = frame.timestamp_sec;
            let _ = i;
        }

        Ok(())
    }
}
