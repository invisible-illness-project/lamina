pub mod dc;
pub mod filter;
pub mod normalize;
pub mod peaks;
pub mod resample;
pub mod segment;
pub mod smooth;

pub use dc::signal_remove_dc;
pub use filter::{
    FilterKind, FilterSpec, SosFilter, SosSection, design_butterworth_sos, signal_filter,
    signal_filtfilt,
};
pub use normalize::{DegeneratePolicy, signal_minmax};
pub use peaks::{
    PeakDetectionConfig, signal_findpeaks, signal_findpeaks_config, signal_findpeaks_mask,
};
pub use resample::{signal_resample, signal_resample_poly};
pub use segment::{IncompleteTailPolicy, signal_segment, signal_segment_duration};
pub use smooth::signal_smooth_moving_average;

