"""Type stubs for native extension _lamina."""

from typing import Optional, List, Any
import numpy as np
from numpy.typing import NDArray

__version__: str

class LaminaError(Exception): ...
class LaminaInputError(ValueError, LaminaError): ...
class EmptySignalError(LaminaInputError): ...
class NonFiniteInputError(LaminaInputError): ...
class DimensionMismatchError(LaminaInputError): ...
class UnsortedEventsError(LaminaInputError): ...
class InsufficientSamplesError(LaminaInputError): ...
class InsufficientPeaksError(LaminaInputError): ...
class LaminaConfigurationError(ValueError, LaminaError): ...
class InvalidSamplingRateError(LaminaConfigurationError): ...
class InvalidCutoffFrequencyError(LaminaConfigurationError): ...
class InvalidWindowSizeError(LaminaConfigurationError): ...
class InvalidFilterOrderError(LaminaConfigurationError): ...
class LaminaProcessingError(RuntimeError, LaminaError): ...

class PyPeakDetectionConfig:
    min_height: Optional[float]
    min_distance: Optional[int]
    min_prominence: Optional[float]
    min_width: Optional[int]
    threshold: Optional[float]
    def __init__(
        self,
        min_height: Optional[float] = ...,
        min_distance: Optional[int] = ...,
        min_prominence: Optional[float] = ...,
        min_width: Optional[int] = ...,
        threshold: Optional[float] = ...,
    ) -> None: ...

def smooth_moving_average(signal: NDArray[np.float64], window_size: int) -> NDArray[np.float64]: ...
def filter(
    signal: NDArray[np.float64],
    sampling_rate: float,
    lowcut: Optional[float] = ...,
    highcut: Optional[float] = ...,
    order: int = ...,
    kind: Optional[str] = ...,
) -> NDArray[np.float64]: ...
def filtfilt(
    signal: NDArray[np.float64],
    sampling_rate: float,
    lowcut: Optional[float] = ...,
    highcut: Optional[float] = ...,
    order: int = ...,
    kind: Optional[str] = ...,
) -> NDArray[np.float64]: ...
def findpeaks(signal: NDArray[np.float64], config: Optional[PyPeakDetectionConfig] = ...) -> NDArray[np.uint64]: ...
def findpeaks_mask(signal: NDArray[np.float64], config: Optional[PyPeakDetectionConfig] = ...) -> NDArray[np.bool_]: ...

class PyEcgPeakDetectionConfig:
    min_rr_sec: float
    max_rr_sec: float
    def __init__(self, min_rr_sec: float = ..., max_rr_sec: float = ...) -> None: ...

def ecg_clean(signal: NDArray[np.float64], sampling_rate: float) -> NDArray[np.float64]: ...
def ecg_findpeaks(signal: NDArray[np.float64], sampling_rate: float, config: Optional[PyEcgPeakDetectionConfig] = ...) -> List[int]: ...
def ecg_findpeaks_mask(signal: NDArray[np.float64], sampling_rate: float, config: Optional[PyEcgPeakDetectionConfig] = ...) -> List[bool]: ...

class PyPpgPeakDetectionConfig:
    min_distance_sec: float
    def __init__(self, min_distance_sec: float = ...) -> None: ...

def ppg_clean(signal: NDArray[np.float64], sampling_rate: float) -> NDArray[np.float64]: ...
def ppg_findpeaks(signal: NDArray[np.float64], sampling_rate: float, config: Optional[PyPpgPeakDetectionConfig] = ...) -> List[int]: ...
def ppg_findpeaks_mask(signal: NDArray[np.float64], sampling_rate: float, config: Optional[PyPpgPeakDetectionConfig] = ...) -> List[bool]: ...

class PyEdaCleaningConfig:
    lowpass_cutoff_hz: float
    order: int
    def __init__(self, lowpass_cutoff_hz: float = ..., order: int = ...) -> None: ...

class PyEdaDecompositionConfig:
    lowpass_cutoff_hz: float
    def __init__(self, lowpass_cutoff_hz: float = ...) -> None: ...

class PyEdaPeakDetectionConfig:
    min_amplitude: float
    def __init__(self, min_amplitude: float = ...) -> None: ...

class PyScrEvent:
    onset_index: int
    peak_index: int
    amplitude: float
    rise_time_sec: float

class PyEdaComponents:
    tonic: Any
    phasic: Any

def eda_clean(signal: NDArray[np.float64], sampling_rate: float, config: Optional[PyEdaCleaningConfig] = ...) -> NDArray[np.float64]: ...
def eda_decompose(signal: NDArray[np.float64], sampling_rate: float, config: Optional[PyEdaDecompositionConfig] = ...) -> PyEdaComponents: ...
def eda_phasic(signal: NDArray[np.float64], sampling_rate: float) -> NDArray[np.float64]: ...
def eda_findpeaks(signal: NDArray[np.float64], sampling_rate: float, config: Optional[PyEdaPeakDetectionConfig] = ...) -> List[int]: ...
def eda_findpeaks_mask(signal: NDArray[np.float64], sampling_rate: float, config: Optional[PyEdaPeakDetectionConfig] = ...) -> List[bool]: ...
def eda_findpeaks_events(signal: NDArray[np.float64], sampling_rate: float, config: Optional[PyEdaPeakDetectionConfig] = ...) -> List[PyScrEvent]: ...

class PyRspCleaningConfig:
    lowpass_hz: float
    highpass_hz: float
    order: int
    def __init__(self, lowpass_hz: float = ..., highpass_hz: float = ..., order: int = ...) -> None: ...

class PyRspProcessingConfig:
    min_breath_duration_sec: float
    max_breath_duration_sec: float
    def __init__(self, min_breath_duration_sec: float = ..., max_breath_duration_sec: float = ...) -> None: ...

class PyRespirationCycle:
    inspiration_index: int
    expiration_index: int
    next_inspiration_index: int
    duration_sec: float
    respiratory_rate_bpm: float
    amplitude: float

def rsp_clean(signal: NDArray[np.float64], sampling_rate: float, config: Optional[PyRspCleaningConfig] = ...) -> NDArray[np.float64]: ...
def rsp_findpeaks(signal: NDArray[np.float64], sampling_rate: float, config: Optional[PyRspProcessingConfig] = ...) -> List[int]: ...
def rsp_findpeaks_mask(signal: NDArray[np.float64], sampling_rate: float, config: Optional[PyRspProcessingConfig] = ...) -> List[bool]: ...
def rsp_cycles(signal: NDArray[np.float64], sampling_rate: float, config: Optional[PyRspProcessingConfig] = ...) -> List[PyRespirationCycle]: ...
def rsp_rate(signal: NDArray[np.float64], sampling_rate: float, config: Optional[PyRspProcessingConfig] = ...) -> NDArray[np.float64]: ...

class PyCorrectionPolicy:
    min_interval_ms: float
    max_interval_ms: float
    max_pct_change: float
    min_consecutive_valid: int
    def __init__(
        self,
        min_interval_ms: float = ...,
        max_interval_ms: float = ...,
        max_pct_change: float = ...,
        min_consecutive_valid: int = ...,
    ) -> None: ...

def peaks_to_intervals(peaks: Any, sampling_rate: float) -> NDArray[np.float64]: ...
def indices_to_intervals(peak_indices: List[int], sampling_rate: float) -> NDArray[np.float64]: ...
def rmssd(rr_intervals: NDArray[np.float64]) -> float: ...
def mean_nn(rr_intervals: NDArray[np.float64]) -> float: ...
def classify_intervals(intervals: NDArray[np.float64], percent_threshold: Optional[float] = ...) -> List[str]: ...
def clean_rr_intervals(intervals: NDArray[np.float64], policy: PyCorrectionPolicy) -> NDArray[np.float64]: ...

def sample_entropy(signal: NDArray[np.float64], m: int = ..., r: float = ...) -> float: ...

class PyNormalizationMethod:
    MIN_MAX: PyNormalizationMethod
    Z_SCORE: PyNormalizationMethod
    ROBUST: PyNormalizationMethod

class PyFeatureDirection:
    HIGHER_IS_MORE_AROUSAL: PyFeatureDirection
    LOWER_IS_MORE_AROUSAL: PyFeatureDirection

class PyNormalizationConfig:
    method: PyNormalizationMethod
    min_samples: int
    def __init__(self, method: Optional[PyNormalizationMethod] = ..., min_samples: Optional[int] = ...) -> None: ...

class PyActivationWeights:
    def __init__(self) -> None: ...

class PyRegulationWeights:
    def __init__(self) -> None: ...

class PyRecoveryConfig:
    def __init__(self) -> None: ...

class PyConfidenceWeights:
    def __init__(self) -> None: ...

class PyQualityConfig:
    def __init__(self) -> None: ...

class PySmoothingConfig:
    def __init__(self) -> None: ...

class PyAutonomicEstimatorConfig:
    def __init__(self) -> None: ...

class PyBaselineFeatureStats:
    mean: Optional[float]
    std: Optional[float]
    median: Optional[float]
    mad: Optional[float]
    sample_count: int
    is_valid: bool

class PyAutonomicBaseline:
    @staticmethod
    def from_features(
        baseline_features: List[PyMultimodalFeatureVector],
        normalization: Optional[PyNormalizationConfig] = ...,
    ) -> PyAutonomicBaseline: ...
    @property
    def hr_bpm_stats(self) -> PyBaselineFeatureStats: ...
    @property
    def sdnn_ms_stats(self) -> PyBaselineFeatureStats: ...
    @property
    def rmssd_ms_stats(self) -> PyBaselineFeatureStats: ...
    @property
    def eda_tonic_stats(self) -> PyBaselineFeatureStats: ...
    @property
    def eda_phasic_stats(self) -> PyBaselineFeatureStats: ...
    @property
    def scr_rate_stats(self) -> PyBaselineFeatureStats: ...
    @property
    def rsp_rate_stats(self) -> PyBaselineFeatureStats: ...
    @property
    def rsp_amplitude_stats(self) -> PyBaselineFeatureStats: ...
    @property
    def rsp_std_stats(self) -> PyBaselineFeatureStats: ...
    @property
    def rsa_bpm_stats(self) -> PyBaselineFeatureStats: ...
    @property
    def phase_coupling_stats(self) -> PyBaselineFeatureStats: ...
    @property
    def pulse_delay_stats(self) -> PyBaselineFeatureStats: ...

class PyCardiacState:
    variability_index: Optional[float]
    heart_rate_index: Optional[float]
    recovery_evidence: Optional[float]
    beat_count: int

class PyElectrodermalState:
    tonic_level_index: Optional[float]
    phasic_activation_index: Optional[float]
    scr_rate_index: Optional[float]
    scr_count: int

class PyRespiratoryState:
    rate_index: Optional[float]
    amplitude_index: Optional[float]
    regularity_index: Optional[float]
    cycle_count: int

class PyCouplingState:
    resphr_coupling_index: Optional[float]
    phase_coupling_index: Optional[float]
    pulse_delay_index: Optional[float]
    association_count: int

class PyStateConfidence:
    overall: Optional[float]
    cardiac: Optional[float]
    electrodermal: Optional[float]
    respiratory: Optional[float]
    coupling: Optional[float]

class PyAutonomicState:
    timestamp: float
    duration_sec: float
    activation_score: Optional[float]
    regulation_score: Optional[float]
    cardiac: PyCardiacState
    electrodermal: PyElectrodermalState
    respiratory: PyRespiratoryState
    coupling: PyCouplingState
    confidence: PyStateConfidence

class PyAutonomicStateSeries:
    states: List[PyAutonomicState]
    smoothed_states: Optional[List[PyAutonomicState]]
    window_duration_sec: float
    step_sec: float

class PyAutonomicEstimator:
    def __init__(self, config: Optional[PyAutonomicEstimatorConfig] = ...) -> None: ...
    def estimate(
        self,
        features: PyMultimodalFeatureVector,
        baseline: PyAutonomicBaseline,
    ) -> PyAutonomicState: ...
    def estimate_series(
        self,
        feature_series: List[PyMultimodalFeatureVector],
        baseline: PyAutonomicBaseline,
    ) -> PyAutonomicStateSeries: ...

class PyVideoFrame:
    def __init__(self, timestamp_sec: float, width: int, height: int, data: bytes) -> None: ...

class PyVideoStream:
    def __init__(self, frames: List[PyVideoFrame], nominal_fps: Optional[float] = ...) -> None: ...
    def add_frame(self, frame: PyVideoFrame) -> None: ...

class PyRoi:
    def __init__(self, x: int, y: int, width: int, height: int) -> None: ...

class PyRppgAlgorithmId:
    CHROM: PyRppgAlgorithmId
    POS: PyRppgAlgorithmId
    GREEN: PyRppgAlgorithmId

class PySignalPolarity:
    NON_INVERTED: PySignalPolarity
    INVERTED: PySignalPolarity
    AUTO_DETECT: PySignalPolarity

class PyRppgWindowConfig:
    window_sec: float
    step_sec: float
    min_window_fraction: float
    def __init__(
        self,
        window_sec: float = ...,
        step_sec: float = ...,
        min_window_fraction: float = ...,
    ) -> None: ...

class PyRppgPreprocessingConfig:
    normalize_channels: bool
    detrend: bool
    def __init__(self, normalize_channels: bool = ..., detrend: bool = ...) -> None: ...

class PyRppgConfig:
    algorithm: PyRppgAlgorithmId
    min_quality: float
    minimum_roi_pixels: int
    max_gap_sec: float
    window: PyRppgWindowConfig
    preprocessing: PyRppgPreprocessingConfig
    signal_band_hz: tuple[float, float]
    polarity: PySignalPolarity
    def __init__(
        self,
        algorithm: Optional[PyRppgAlgorithmId] = ...,
        min_quality: Optional[float] = ...,
        minimum_roi_pixels: Optional[int] = ...,
        max_gap_sec: Optional[float] = ...,
        window: Optional[PyRppgWindowConfig] = ...,
        preprocessing: Optional[PyRppgPreprocessingConfig] = ...,
        signal_band_hz: Optional[tuple[float, float]] = ...,
        polarity: Optional[PySignalPolarity] = ...,
    ) -> None: ...

class PyRppgSegmentQuality:
    start_time_sec: float
    end_time_sec: float
    snr_db: float
    peak_prominence: float
    spectral_entropy: float
    overall_quality: float

class PyRppgSignal:
    timestamps_sec: NDArray[np.float64]
    waveform: NDArray[np.float64]
    sampling_rate_hz: float
    overall_quality: float
    segment_qualities: List[PyRppgSegmentQuality]

def extract_rppg(video: PyVideoStream, roi: PyRoi, config: Optional[PyRppgConfig] = ...) -> PyRppgSignal: ...

class PyWindowConfig:
    window_duration_sec: float
    step_sec: float
    min_coverage: float
    def __init__(
        self,
        window_duration_sec: float = ...,
        step_sec: float = ...,
        min_coverage: float = ...,
    ) -> None: ...

class PyFeatureConfig:
    window: PyWindowConfig
    cardiac_rr_min_ms: float
    cardiac_rr_max_ms: float
    eda_min_amplitude_us: float
    def __init__(
        self,
        window: Optional[PyWindowConfig] = ...,
        cardiac_rr_min_ms: Optional[float] = ...,
        cardiac_rr_max_ms: Optional[float] = ...,
        eda_min_amplitude_us: Optional[float] = ...,
    ) -> None: ...

class PyMultimodalInput:
    def __init__(self) -> None: ...
    def with_ecg(self, r_peaks: List[int], sampling_rate: float, offset_sec: float = ...) -> None: ...
    def with_ppg(self, peaks: List[int], sampling_rate: float, offset_sec: float = ...) -> None: ...
    def with_eda(
        self,
        tonic: NDArray[np.float64],
        phasic: NDArray[np.float64],
        scr_events: List[PyScrEvent],
        sampling_rate: float,
        offset_sec: float = ...,
    ) -> None: ...
    def with_rsp(
        self,
        cycles: List[PyRespirationCycle],
        sampling_rate: float,
        offset_sec: float = ...,
    ) -> None: ...

class PyMultimodalFeatureVector:
    start_time_sec: float
    end_time_sec: float
    duration_sec: float

    mean_hr_bpm: Optional[float]
    median_hr_bpm: Optional[float]
    sdnn_ms: Optional[float]
    rmssd_ms: Optional[float]
    pnn50: Optional[float]
    rr_mean_ms: Optional[float]
    rr_std_ms: Optional[float]
    beat_count: int

    mean_tonic_us: Optional[float]
    median_tonic_us: Optional[float]
    tonic_std_us: Optional[float]
    mean_phasic_us: Optional[float]
    phasic_std_us: Optional[float]
    scr_count: int
    scr_rate_per_min: Optional[float]
    mean_scr_amplitude_us: Optional[float]
    median_scr_amplitude_us: Optional[float]
    mean_scr_rise_time_sec: Optional[float]

    mean_rsp_rate_bpm: Optional[float]
    median_rsp_rate_bpm: Optional[float]
    rsp_rate_std_bpm: Optional[float]
    mean_cycle_duration_sec: Optional[float]
    cycle_count: int
    mean_rsp_amplitude: Optional[float]
    rsp_amplitude_std: Optional[float]

    rsa_amplitude_bpm: Optional[float]
    rsa_amplitude_rr_sec: Optional[float]
    cardiac_respiratory_concentration: Optional[float]
    cardiac_respiratory_mean_phase: Optional[float]
    mean_pulse_delay_sec: Optional[float]
    pulse_delay_std_sec: Optional[float]
    scr_cardiac_association_count: int

    coverage: float
    coverage_overall: float
    coverage_ecg: Optional[float]
    coverage_ppg: Optional[float]
    coverage_eda: Optional[float]
    coverage_rsp: Optional[float]
    cardiac_valid: bool
    eda_valid: bool
    respiration_valid: bool
    coupling_valid: bool
    usable_feature_count: int
    total_feature_count: int
    quality_issues: List[str]

    def to_dict(self) -> dict: ...
    def to_numpy(self) -> NDArray[np.float64]: ...

def extract_features(input: PyMultimodalInput, config: Optional[PyFeatureConfig] = ...) -> List[PyMultimodalFeatureVector]: ...

class PyRsaConfig:
    min_rsp_amplitude: float
    def __init__(self, min_rsp_amplitude: float = ...) -> None: ...

class PyPulseTimingConfig:
    max_transit_time_sec: float
    min_transit_time_sec: float
    def __init__(self, max_transit_time_sec: float = ..., min_transit_time_sec: float = ...) -> None: ...

class PyPulseTimingResult:
    transit_times_sec: List[float]
    mean_transit_time_sec: Optional[float]
    std_transit_time_sec: Optional[float]

class PyCardiacRespiratoryEvent:
    time_sec: float
    phase_rad: float

class PyScrCardiorespiratoryAssociation:
    event_count: int

class PyRsaResult:
    amplitude_bpm: float
    amplitude_rr_sec: float
    valid_beats: int
    valid_cycles: int

class PyPhaseCouplingResult:
    concentration: float
    mean_phase: float
    sample_count: int

class PyModalityQuality:
    score: float
    valid: bool
    issues: List[str]
    def __init__(self, score: float, valid: bool, issues: Optional[List[str]] = ...) -> None: ...

class PyMultimodalQuality:
    ecg_quality: Optional[PyModalityQuality]
    ppg_quality: Optional[PyModalityQuality]
    eda_quality: Optional[PyModalityQuality]
    rsp_quality: Optional[PyModalityQuality]
    overall_quality: float

def rsa(
    r_peaks: List[int],
    ecg_sampling_rate: float,
    ecg_offset_sec: float,
    rsp_cycles: List[PyRespirationCycle],
    rsp_sampling_rate: float,
    rsp_offset_sec: float,
) -> PyRsaResult: ...
def rsa_config(
    r_peaks: List[int],
    ecg_sampling_rate: float,
    ecg_offset_sec: float,
    rsp_cycles: List[PyRespirationCycle],
    rsp_sampling_rate: float,
    rsp_offset_sec: float,
    config: Optional[PyRsaConfig] = ...,
) -> PyRsaResult: ...
def cardiac_respiratory_phase(
    r_peaks: List[int],
    ecg_sampling_rate: float,
    ecg_offset_sec: float,
    rsp_cycles: List[PyRespirationCycle],
    rsp_sampling_rate: float,
    rsp_offset_sec: float,
) -> List[PyCardiacRespiratoryEvent]: ...
def cardiorespiratory_phase_coupling(phases: List[float]) -> PyPhaseCouplingResult: ...
def ecg_ppg_timing(
    ecg_peaks: List[int],
    ecg_sampling_rate: float,
    ecg_offset_sec: float,
    ppg_peaks: List[int],
    ppg_sampling_rate: float,
    ppg_offset_sec: float,
    config: Optional[PyPulseTimingConfig] = ...,
) -> PyPulseTimingResult: ...
def eda_cardiorespiratory_association(
    scr_events: List[PyScrEvent],
    eda_sampling_rate: float,
    eda_offset_sec: float,
    rsp_cycles: List[PyRespirationCycle],
    rsp_sampling_rate: float,
    rsp_offset_sec: float,
) -> PyScrCardiorespiratoryAssociation: ...
def respiratory_phase_at_time(
    time_sec: float,
    cycles: List[PyRespirationCycle],
    sampling_rate: float,
    offset_sec: float,
) -> Optional[float]: ...
def evaluate_ecg_quality(signal: NDArray[np.float64], sampling_rate: float, r_peaks: List[int]) -> PyModalityQuality: ...
def evaluate_rsp_quality(signal: NDArray[np.float64], sampling_rate: float, cycles: List[PyRespirationCycle]) -> PyModalityQuality: ...
def multimodal_quality(
    ecg_quality: Optional[PyModalityQuality] = ...,
    ppg_quality: Optional[PyModalityQuality] = ...,
    eda_quality: Optional[PyModalityQuality] = ...,
    rsp_quality: Optional[PyModalityQuality] = ...,
) -> PyMultimodalQuality: ...
