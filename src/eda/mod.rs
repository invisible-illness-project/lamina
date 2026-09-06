pub mod clean;
pub mod peaks;
pub mod phasic;

pub use clean::{EdaCleaningConfig, eda_clean, eda_clean_config};
pub use peaks::{
    EdaPeakDetectionConfig, ScrEvent, eda_findpeaks, eda_findpeaks_config, eda_findpeaks_events,
    eda_findpeaks_mask,
};
pub use phasic::{EdaComponents, EdaDecompositionConfig, eda_decompose, eda_phasic};
