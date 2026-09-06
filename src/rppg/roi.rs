use crate::error::{Result, SignalError};
use crate::rppg::signal::RoiSample;
use crate::rppg::video::VideoFrame;

/// Spatial Region of Interest (ROI) bounding box within a video frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Roi {
    /// X-coordinate of top-left corner (column index)
    pub x: usize,
    /// Y-coordinate of top-left corner (row index)
    pub y: usize,
    /// Width in pixels
    pub width: usize,
    /// Height in pixels
    pub height: usize,
}

impl Roi {
    /// Construct a new `Roi` and validate dimensions.
    pub fn new(x: usize, y: usize, width: usize, height: usize) -> Result<Self> {
        let roi = Self {
            x,
            y,
            width,
            height,
        };
        if roi.width == 0 || roi.height == 0 {
            return Err(SignalError::InvalidWindowSize(0));
        }
        Ok(roi)
    }

    /// Validate that the ROI lies completely within frame boundaries.
    pub fn validate_for_frame(&self, frame_width: usize, frame_height: usize) -> Result<()> {
        if self.width == 0 || self.height == 0 {
            return Err(SignalError::InvalidWindowSize(0));
        }
        let right = self
            .x
            .checked_add(self.width)
            .ok_or(SignalError::DimensionMismatch)?;
        let bottom = self
            .y
            .checked_add(self.height)
            .ok_or(SignalError::DimensionMismatch)?;

        if right > frame_width || bottom > frame_height {
            return Err(SignalError::DimensionMismatch);
        }
        Ok(())
    }

    /// Compute center coordinates $(x_{\text{center}}, y_{\text{center}})$.
    pub fn center(&self) -> (f64, f64) {
        (
            self.x as f64 + self.width as f64 * 0.5,
            self.y as f64 + self.height as f64 * 0.5,
        )
    }
}

/// Abstract provider interface for retrieving frame-indexed or timestamp-indexed ROIs.
pub trait RoiProvider: std::fmt::Debug {
    /// Retrieve the ROI for a specific frame index and timestamp, or `None` if invalid/missing.
    fn roi_for_frame(&self, frame_index: usize, timestamp_sec: f64) -> Option<Roi>;

    /// Retrieve frame-to-frame ROI center displacement in pixels for frame `i` relative to frame `i-1`.
    fn displacement_at_frame(&self, frame_index: usize) -> f64 {
        let _ = frame_index;
        0.0
    }
}

/// Static ROI provider returning an unmoving ROI across all frames.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StaticRoi {
    pub roi: Roi,
}

impl StaticRoi {
    pub fn new(roi: Roi) -> Self {
        Self { roi }
    }
}

impl RoiProvider for StaticRoi {
    fn roi_for_frame(&self, _frame_index: usize, _timestamp_sec: f64) -> Option<Roi> {
        Some(self.roi)
    }

    fn displacement_at_frame(&self, _frame_index: usize) -> f64 {
        0.0
    }
}

/// Tracked ROI provider supplying dynamic per-frame ROIs and displacement metrics.
#[derive(Debug, Clone, PartialEq)]
pub struct TrackedRoiSeries {
    pub rois: Vec<Option<Roi>>,
}

impl TrackedRoiSeries {
    pub fn new(rois: Vec<Option<Roi>>) -> Self {
        Self { rois }
    }
}

impl RoiProvider for TrackedRoiSeries {
    fn roi_for_frame(&self, frame_index: usize, _timestamp_sec: f64) -> Option<Roi> {
        self.rois.get(frame_index).copied().flatten()
    }

    fn displacement_at_frame(&self, frame_index: usize) -> f64 {
        if frame_index == 0 || frame_index >= self.rois.len() {
            return 0.0;
        }
        match (self.rois[frame_index - 1], self.rois[frame_index]) {
            (Some(r1), Some(r2)) => {
                let (c1x, c1y) = r1.center();
                let (c2x, c2y) = r2.center();
                let dx = c2x - c1x;
                let dy = c2y - c1y;
                (dx * dx + dy * dy).sqrt()
            }
            _ => 0.0,
        }
    }
}

/// Deterministically aggregate mean RGB channel intensities across valid pixels in an ROI.
pub fn extract_roi_sample(frame: &VideoFrame, roi: &Roi) -> Result<RoiSample> {
    frame.validate()?;
    roi.validate_for_frame(frame.width, frame.height)?;

    let mut sum_r = 0.0f64;
    let mut sum_g = 0.0f64;
    let mut sum_b = 0.0f64;
    let mut pixel_count = 0usize;

    for row in roi.y..(roi.y + roi.height) {
        let row_offset = row * frame.width * 3;
        for col in roi.x..(roi.x + roi.width) {
            let idx = row_offset + col * 3;
            sum_r += frame.data[idx] as f64;
            sum_g += frame.data[idx + 1] as f64;
            sum_b += frame.data[idx + 2] as f64;
            pixel_count += 1;
        }
    }

    if pixel_count == 0 {
        return Err(SignalError::InvalidWindowSize(0));
    }

    let count_f64 = pixel_count as f64;
    Ok(RoiSample {
        timestamp_sec: frame.timestamp_sec,
        red: sum_r / count_f64,
        green: sum_g / count_f64,
        blue: sum_b / count_f64,
        valid_pixels: pixel_count,
    })
}
