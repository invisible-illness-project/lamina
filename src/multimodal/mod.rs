pub mod config;
pub mod coupling;
pub mod ecg_ppg;
pub mod eda_assoc;
pub mod phase;
pub mod quality;
pub mod rsa;
pub mod sync;

pub use config::{MultimodalConfig, PulseTimingConfig, RsaConfig};
pub use coupling::{PhaseCouplingResult, cardiorespiratory_phase_coupling};
pub use ecg_ppg::{PulseTimingResult, ecg_ppg_timing, ecg_ppg_timing_config};
pub use eda_assoc::{ScrCardiorespiratoryAssociation, eda_cardiorespiratory_association};
pub use phase::respiratory_phase_at_time;
pub use quality::{
    ModalityQuality, MultimodalQuality, QualityIssue, evaluate_ecg_quality, evaluate_rsp_quality,
    multimodal_quality,
};
pub use rsa::{CardiacRespiratoryEvent, RsaResult, cardiac_respiratory_phase, rsa, rsa_config};
pub use sync::{Modality, TimedEvent, sample_to_time};
