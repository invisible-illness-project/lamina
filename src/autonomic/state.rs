use crate::autonomic::confidence::StateConfidence;

/// Cardiac physiological state evidence derived from windowed HRV and heart rate features.
///
/// # Scientific Guardrail & Non-Clinical Interpretation
/// `variability_index` represents normalized cardiac variability evidence relative to baseline
/// (SDNN preferred when valid, with RMSSD used as fallback). It is **not** a direct measurement of
/// cardiac sympathetic outflow, vagal tone, or sympathovagal balance ($X/Y$).
///
/// # Citation
/// Carter JR, Jenkins NDM, Bigalke JA, et al. Guidelines for rigor and reproducibility of heart rate
/// variability within human cardiovascular research. *Am J Physiol Heart Circ Physiol*. 2026.
/// DOI: [10.1152/ajpheart.00041.2026](https://doi.org/10.1152/ajpheart.00041.2026).
#[derive(Debug, Clone, PartialEq)]
pub struct CardiacState {
    /// Normalized cardiac variability evidence index in $[-1.0, 1.0]$ (SDNN preferred, RMSSD fallback)
    pub variability_index: Option<f64>,
    /// Baseline-relative heart rate evidence index in $[-1.0, 1.0]$ derived from mean HR
    pub heart_rate_index: Option<f64>,
    /// Engineered cardiac recovery evidence index in $[-1.0, 1.0]$ ($\frac{w_{\text{var}} v - w_{\text{hr}} h}{w_{\text{var}} + w_{\text{hr}}}$)
    pub recovery_evidence: Option<f64>,
    /// Total observed ECG R-peaks in feature window
    pub beat_count: usize,
}

impl CardiacState {
    /// Empty [`CardiacState`] container with default `None` values and zero beats.
    pub fn empty() -> Self {
        Self {
            variability_index: None,
            heart_rate_index: None,
            recovery_evidence: None,
            beat_count: 0,
        }
    }
}

/// Electrodermal activation state evidence derived from Tonic SCL, Phasic SCR, and event rates.
///
/// Represents autonomic electrodermal arousal evidence. It is **not** a complete measure of total
/// sympathetic nervous system output.
#[derive(Debug, Clone, PartialEq)]
pub struct ElectrodermalState {
    /// Bounded tonic skin conductance level index in $[-1.0, 1.0]$
    pub tonic_level_index: Option<f64>,
    /// Bounded phasic skin conductance response activation index in $[-1.0, 1.0]$
    pub phasic_activation_index: Option<f64>,
    /// Bounded SCR event rate index in $[-1.0, 1.0]$
    pub scr_rate_index: Option<f64>,
    /// Total observed SCR events in feature window
    pub scr_count: usize,
}

impl ElectrodermalState {
    /// Empty [`ElectrodermalState`] container with default `None` values and zero SCR events.
    pub fn empty() -> Self {
        Self {
            tonic_level_index: None,
            phasic_activation_index: None,
            scr_rate_index: None,
            scr_count: 0,
        }
    }
}

/// Respiratory dynamics state evidence derived from rate, duration, amplitude, and variability features.
///
/// # Citation
/// Buron J, Menuet C. Respiratory heart rate variability: Insights into mechanisms, measurements and
/// interpretations. *Biol Psychol*. 2026;208:109299. DOI: [10.1016/j.biopsycho.2026.109299](https://doi.org/10.1016/j.biopsycho.2026.109299).
#[derive(Debug, Clone, PartialEq)]
pub struct RespiratoryState {
    /// Bounded respiratory rate evidence index in $[-1.0, 1.0]$
    pub rate_index: Option<f64>,
    /// Bounded breath cycle amplitude index in $[-1.0, 1.0]$ derived from mean cycle height
    pub amplitude_index: Option<f64>,
    /// Baseline-relative evidence of respiratory-rate regularity derived from the inverse direction of `rate_std_bpm`
    pub regularity_index: Option<f64>,
    /// Total observed respiration cycles in feature window
    pub cycle_count: usize,
}

impl RespiratoryState {
    /// Empty [`RespiratoryState`] container with default `None` values and zero cycles.
    pub fn empty() -> Self {
        Self {
            rate_index: None,
            amplitude_index: None,
            regularity_index: None,
            cycle_count: 0,
        }
    }
}

/// Multimodal physiological coupling state evidence.
///
/// # Terminology & RespHRV Guardrail
/// `resphr_coupling_index` represents **Respiratory Heart Rate Variability (RespHRV)** / RSA coupling
/// evidence conditioned on valid direct respiratory context. It is **not** a direct measurement of vagal tone,
/// and is strictly unavailable (`None`) whenever valid direct respiration context is absent.
///
/// # Citations
/// - International Expert Recommendation. Redefining respiratory sinus arrhythmia as respiratory heart rate variability. 2025. PMID: [40328963](https://pubmed.ncbi.nlm.nih.gov/40328963/).
/// - Gevonden M, et al. Controlling heart rate variability for respiratory effects in ambulatory psychophysiological measurements. *Biol Psychol*. 2025. DOI: [10.1016/j.biopsycho.2025.109171](https://doi.org/10.1016/j.biopsycho.2025.109171).
#[derive(Debug, Clone, PartialEq)]
pub struct CouplingState {
    /// Bounded RespHRV (RSA) coupling evidence index in $[-1.0, 1.0]$ (strictly `None` if respiration missing)
    pub resphr_coupling_index: Option<f64>,
    /// Bounded cardiorespiratory phase concentration coupling index in $[-1.0, 1.0]$
    pub phase_coupling_index: Option<f64>,
    /// Bounded vascular ECG-PPG pulse delay index in $[-1.0, 1.0]$
    pub pulse_delay_index: Option<f64>,
    /// Total observed EDA cardiorespiratory associations in feature window
    pub association_count: usize,
}

impl CouplingState {
    /// Empty [`CouplingState`] container with default `None` values and zero associations.
    pub fn empty() -> Self {
        Self {
            resphr_coupling_index: None,
            phase_coupling_index: None,
            pulse_delay_index: None,
            association_count: 0,
        }
    }

    /// Alias getter for RSA coupling index matching Task 7 naming conventions.
    pub fn rsa_coupling_index(&self) -> Option<f64> {
        self.resphr_coupling_index
    }
}

/// Complete interpretable multimodal physiological state representation over a feature window.
#[derive(Debug, Clone, PartialEq)]
pub struct AutonomicState {
    /// Physical window start timestamp in seconds
    pub timestamp: f64,
    /// Physical window duration in seconds
    pub duration_sec: f64,
    /// Cardiac state representation
    pub cardiac: CardiacState,
    /// Electrodermal state representation
    pub electrodermal: ElectrodermalState,
    /// Respiratory state representation
    pub respiratory: RespiratoryState,
    /// Multimodal coupling state representation
    pub coupling: CouplingState,
    /// Multimodal Physiological Activation Evidence Index in $[-1.0, 1.0]$
    pub activation_score: Option<f64>,
    /// Multimodal Cardiorespiratory Regulation & Coupling Evidence Index in $[-1.0, 1.0]$
    pub regulation_score: Option<f64>,
    /// Multi-tiered evidence confidence summary in $[0.0, 1.0]$
    pub confidence: StateConfidence,
}

/// Chronologically ordered series of autonomic state estimates over sliding feature windows.
#[derive(Debug, Clone, PartialEq)]
pub struct AutonomicStateSeries {
    /// Vector of sequential raw (unsmoothed) window state estimates
    pub states: Vec<AutonomicState>,
    /// Optional vector of sequential EMA smoothed window state estimates (`None` if smoothing is disabled)
    pub smoothed_states: Option<Vec<AutonomicState>>,
    /// Window duration in seconds
    pub window_duration_sec: f64,
    /// Step size (hop size) in seconds
    pub step_sec: f64,
}
