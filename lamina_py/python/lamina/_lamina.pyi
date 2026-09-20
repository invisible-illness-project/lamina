"""Type stubs for native extension _lamina."""

from typing import Optional, List, Any
import numpy as np
from numpy.typing import NDArray

__version__: str

class LaminaError(Exception): ...
class LaminaInputError(ValueError): ...
class EmptySignalError(LaminaInputError): ...
class NonFiniteInputError(LaminaInputError): ...
class DimensionMismatchError(LaminaInputError): ...
class UnsortedEventsError(LaminaInputError): ...
class InsufficientSamplesError(LaminaInputError): ...
class InsufficientPeaksError(LaminaInputError): ...
class LaminaConfigurationError(ValueError): ...
class InvalidSamplingRateError(LaminaConfigurationError): ...
class InvalidCutoffFrequencyError(LaminaConfigurationError): ...
class InvalidWindowSizeError(LaminaConfigurationError): ...
class InvalidFilterOrderError(LaminaConfigurationError): ...
class LaminaProcessingError(RuntimeError): ...

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
    kind: str = ...,
) -> NDArray[np.float64]: ...
def filtfilt(
    signal: NDArray[np.float64],
    sampling_rate: float,
    lowcut: Optional[float] = ...,
    highcut: Optional[float] = ...,
    order: int = ...,
    kind: str = ...,
) -> NDArray[np.float64]: ...
def findpeaks(signal: NDArray[np.float64], config: Optional[PyPeakDetectionConfig] = ...) -> NDArray[np.uint64]: ...
def findpeaks_mask(signal: NDArray[np.float64], config: Optional[PyPeakDetectionConfig] = ...) -> NDArray[np.bool_]: ...

class PyEcgPeakDetectionConfig:
    lowcut: Optional[float]
    highcut: Optional[float]
    filter_order: Optional[int]
    integration_window_sec: Optional[float]
    refractory_period_sec: Optional[float]
    searchback: Optional[bool]
    threshold_multiplier: Optional[float]
    def __init__(
        self,
        lowcut: Optional[float] = ...,
        highcut: Optional[float] = ...,
        filter_order: Optional[int] = ...,
        integration_window_sec: Optional[float] = ...,
        refractory_period_sec: Optional[float] = ...,
        searchback: Optional[bool] = ...,
        threshold_multiplier: Optional[float] = ...,
    ) -> None: ...

def ecg_clean(signal: NDArray[np.float64], sampling_rate: float, method: str = ...) -> NDArray[np.float64]: ...
def ecg_findpeaks(signal: NDArray[np.float64], sampling_rate: float, config: Optional[PyEcgPeakDetectionConfig] = ...) -> NDArray[np.uint64]: ...
def ecg_findpeaks_mask(signal: NDArray[np.float64], sampling_rate: float, config: Optional[PyEcgPeakDetectionConfig] = ...) -> NDArray[np.bool_]: ...

class PyPpgPeakDetectionConfig:
    lowcut: Optional[float]
    highcut: Optional[float]
    filter_order: Optional[int]
    w_peak_sec: Optional[float]
    w_beat_sec: Optional[float]
    alpha: Optional[float]
    refractory_period_sec: Optional[float]
    def __init__(
        self,
        lowcut: Optional[float] = ...,
        highcut: Optional[float] = ...,
        filter_order: Optional[int] = ...,
        w_peak_sec: Optional[float] = ...,
        w_beat_sec: Optional[float] = ...,
        alpha: Optional[float] = ...,
        refractory_period_sec: Optional[float] = ...,
    ) -> None: ...

def ppg_clean(signal: NDArray[np.float64], sampling_rate: float) -> NDArray[np.float64]: ...
def ppg_findpeaks(signal: NDArray[np.float64], sampling_rate: float, config: Optional[PyPpgPeakDetectionConfig] = ...) -> NDArray[np.uint64]: ...
def ppg_findpeaks_mask(signal: NDArray[np.float64], sampling_rate: float, config: Optional[PyPpgPeakDetectionConfig] = ...) -> NDArray[np.bool_]: ...

class PyEdaCleaningConfig:
    lowpass_cutoff_hz: Optional[float]
    filter_order: Optional[int]
    pass_through_if_nyquist_violated: bool
    def __init__(
        self,
        lowpass_cutoff_hz: Optional[float] = ...,
        filter_order: Optional[int] = ...,
        pass_through_if_nyquist_violated: bool = ...,
    ) -> None: ...

class PyEdaDecompositionConfig:
    tonic_cutoff_hz: Optional[float]
    filter_order: Optional[int]
    def __init__(self, tonic_cutoff_hz: Optional[float] = ..., filter_order: Optional[int] = ...) -> None: ...

class PyEdaPeakDetectionConfig:
    min_amplitude: Optional[float]
    min_prominence: Optional[float]
    min_distance_sec: Optional[float]
    min_rise_time_sec: Optional[float]
    max_rise_time_sec: Optional[float]
    def __init__(
        self,
        min_amplitude: Optional[float] = ...,
        min_prominence: Optional[float] = ...,
        min_distance_sec: Optional[float] = ...,
        min_rise_time_sec: Optional[float] = ...,
        max_rise_time_sec: Optional[float] = ...,
    ) -> None: ...

class PyScrEvent:
    onset_index: int
    peak_index: int
    amplitude: float
    rise_time_sec: float
    def __init__(self, onset_index: int, peak_index: int, amplitude: float, rise_time_sec: float) -> None: ...

class PyEdaComponents:
    tonic: Any
    phasic: Any

def eda_clean(signal: NDArray[np.float64], sampling_rate: float, config: Optional[PyEdaCleaningConfig] = ...) -> NDArray[np.float64]: ...
def eda_decompose(signal: NDArray[np.float64], sampling_rate: float, config: Optional[PyEdaDecompositionConfig] = ...) -> PyEdaComponents: ...
def eda_phasic(signal: NDArray[np.float64], sampling_rate: float) -> NDArray[np.float64]: ...
def eda_findpeaks(phasic_signal: NDArray[np.float64], sampling_rate: float, config: Optional[PyEdaPeakDetectionConfig] = ...) -> NDArray[np.uint64]: ...
def eda_findpeaks_mask(phasic_signal: NDArray[np.float64], sampling_rate: float, config: Optional[PyEdaPeakDetectionConfig] = ...) -> NDArray[np.bool_]: ...
def eda_findpeaks_events(phasic_signal: NDArray[np.float64], sampling_rate: float, config: Optional[PyEdaPeakDetectionConfig] = ...) -> List[PyScrEvent]: ...

class PyRspCleaningConfig:
    lowcut: Optional[float]
    highcut: Optional[float]
    filter_order: Optional[int]
    def __init__(
        self,
        lowcut: Optional[float] = ...,
        highcut: Optional[float] = ...,
        filter_order: Optional[int] = ...,
    ) -> None: ...

class PyRspProcessingConfig:
    lowcut: Optional[float]
    highcut: Optional[float]
    filter_order: Optional[int]
    min_breath_interval_sec: Optional[float]
    max_breath_interval_sec: Optional[float]
    min_amplitude: Optional[float]
    precleaned: Optional[bool]
    def __init__(
        self,
        lowcut: Optional[float] = ...,
        highcut: Optional[float] = ...,
        filter_order: Optional[int] = ...,
        min_breath_interval_sec: Optional[float] = ...,
        max_breath_interval_sec: Optional[float] = ...,
        min_amplitude: Optional[float] = ...,
        precleaned: Optional[bool] = ...,
    ) -> None: ...

class PyRespirationCycle:
    inspiration_index: int
    expiration_index: int
    next_inspiration_index: int
    duration_sec: float
    respiratory_rate_bpm: float
    amplitude: float
    def __init__(
        self,
        inspiration_index: int,
        expiration_index: int,
        next_inspiration_index: int,
        duration_sec: float,
        respiratory_rate_bpm: float,
        amplitude: float,
    ) -> None: ...

def rsp_clean(signal: NDArray[np.float64], sampling_rate: float, config: Optional[PyRspCleaningConfig] = ...) -> NDArray[np.float64]: ...
def rsp_findpeaks(signal: NDArray[np.float64], sampling_rate: float, config: Optional[PyRspProcessingConfig] = ...) -> NDArray[np.uint64]: ...
def rsp_findpeaks_mask(signal: NDArray[np.float64], sampling_rate: float, config: Optional[PyRspProcessingConfig] = ...) -> NDArray[np.bool_]: ...
def rsp_cycles(signal: NDArray[np.float64], sampling_rate: float, config: Optional[PyRspProcessingConfig] = ...) -> List[PyRespirationCycle]: ...
def rsp_rate(signal: NDArray[np.float64], sampling_rate: float, config: Optional[PyRspProcessingConfig] = ...) -> NDArray[np.float64]: ...

class PyCorrectionPolicy:
    @staticmethod
    def none() -> PyCorrectionPolicy: ...
    @staticmethod
    def reject_invalid() -> PyCorrectionPolicy: ...
    @staticmethod
    def interpolate_linear() -> PyCorrectionPolicy: ...
    @staticmethod
    def interpolate_cubic() -> PyCorrectionPolicy: ...
    @staticmethod
    def percent_threshold(pct: float) -> PyCorrectionPolicy: ...

def peaks_to_intervals(peaks: NDArray[np.bool_], sampling_rate: float) -> NDArray[np.float64]: ...
def indices_to_intervals(peak_indices: List[int], sampling_rate: float) -> NDArray[np.float64]: ...
def rmssd(intervals: NDArray[np.float64]) -> float: ...
def mean_nn(intervals: NDArray[np.float64]) -> float: ...
def classify_intervals(intervals: NDArray[np.float64], percent_threshold: Optional[float] = ...) -> List[str]: ...
def clean_rr_intervals(intervals: NDArray[np.float64], policy: PyCorrectionPolicy) -> NDArray[np.float64]: ...

def sample_entropy(signal: NDArray[np.float64], m: int, r: float) -> float: ...

class PyNormalizationMethod:
    ZScore: PyNormalizationMethod
    RobustMedianMad: PyNormalizationMethod

class PyFeatureDirection:
    Positive: PyFeatureDirection
    Negative: PyFeatureDirection

class PyNormalizationConfig:
    def __init__(
        self,
        method: Optional[PyNormalizationMethod] = ...,
        min_baseline_samples: Optional[int] = ...,
        bounded_scale: Optional[float] = ...,
        min_scale: Optional[float] = ...,
        mad_multiplier: Optional[float] = ...,
    ) -> None: ...

class PyActivationWeights:
    def __init__(
        self,
        hr_weight: Optional[float] = ...,
        eda_phasic_weight: Optional[float] = ...,
        scr_rate_weight: Optional[float] = ...,
        rsp_rate_weight: Optional[float] = ...,
    ) -> None: ...

class PyRegulationWeights:
    def __init__(
        self,
        cardiac_variability_weight: Optional[float] = ...,
        resphr_coupling_weight: Optional[float] = ...,
        phase_coupling_weight: Optional[float] = ...,
    ) -> None: ...

class PyRecoveryConfig:
    def __init__(
        self,
        variability_weight: Optional[float] = ...,
        heart_rate_weight: Optional[float] = ...,
    ) -> None: ...

class PyConfidenceWeights:
    def __init__(
        self,
        min_beats: Optional[int] = ...,
        min_cycles: Optional[int] = ...,
        quality_factor: Optional[float] = ...,
        cardiac_weight: Optional[float] = ...,
        eda_weight: Optional[float] = ...,
        rsp_weight: Optional[float] = ...,
        coupling_weight: Optional[float] = ...,
    ) -> None: ...

class PyQualityConfig:
    def __init__(
        self,
        min_coverage: Optional[float] = ...,
        require_cardiac_validity: Optional[bool] = ...,
        require_respiration_for_resphrv: Optional[bool] = ...,
        confidence: Optional[PyConfidenceWeights] = ...,
    ) -> None: ...

class PySmoothingConfig:
    def __init__(self, alpha: Optional[float] = ...) -> None: ...

class PyAutonomicEstimatorConfig:
    def __init__(
        self,
        normalization: Optional[PyNormalizationConfig] = ...,
        activation: Optional[PyActivationWeights] = ...,
        regulation: Optional[PyRegulationWeights] = ...,
        recovery: Optional[PyRecoveryConfig] = ...,
        quality: Optional[PyQualityConfig] = ...,
        smoothing: Optional[PySmoothingConfig] = ...,
    ) -> None: ...

class PyBaselineFeatureStats:
    mean: Optional[float]
    std: Optional[float]
    median: Optional[float]
    mad: Optional[float]
    sample_count: int
    is_valid: bool

class PyAutonomicBaseline:
    hr_bpm_stats: PyBaselineFeatureStats
    sdnn_ms_stats: PyBaselineFeatureStats
    rmssd_ms_stats: PyBaselineFeatureStats
    eda_tonic_stats: PyBaselineFeatureStats
    eda_phasic_stats: PyBaselineFeatureStats
    scr_rate_stats: PyBaselineFeatureStats
    rsp_rate_stats: PyBaselineFeatureStats
    rsp_amplitude_stats: PyBaselineFeatureStats
    rsp_std_stats: PyBaselineFeatureStats
    rsa_bpm_stats: PyBaselineFeatureStats
    phase_coupling_stats: PyBaselineFeatureStats
    pulse_delay_stats: PyBaselineFeatureStats
    @staticmethod
    def from_features(
        baseline_features: List[PyMultimodalFeatureVector],
        normalization: Optional[PyNormalizationConfig] = ...,
    ) -> PyAutonomicBaseline: ...

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
    timestamp_sec: float
    width: int
    height: int
    def __init__(self, timestamp_sec: float, width: int, height: int, data: bytes) -> None: ...

class PyVideoStream:
    def __init__(self, frames: List[PyVideoFrame], nominal_fps: Optional[float] = ...) -> None: ...

class PyRoi:
    x: int
    y: int
    width: int
    height: int
    def __init__(self, x: int, y: int, width: int, height: int) -> None: ...

class PyRppgAlgorithmId:
    GreenChannel: PyRppgAlgorithmId
    Chrom: PyRppgAlgorithmId
    Pos: PyRppgAlgorithmId

class PySignalPolarity:
    Normal: PySignalPolarity
    Inverted: PySignalPolarity
    AutoDetect: PySignalPolarity

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
    start_sec: float
    end_sec: float
    overall: float
    roi_quality: float
    motion_quality: float
    illumination_quality: float
    signal_quality: float
    valid_fraction: float

class PyRppgSignal:
    timestamps_sec: NDArray[np.float64]
    waveform: NDArray[np.float64]
    sampling_rate_hz: float
    overall_quality: float
    valid_fraction: float
    algorithm: str
    segments: List[PyRppgSegmentQuality]

def extract_rppg(video: PyVideoStream, roi: PyRoi, config: Optional[PyRppgConfig] = ...) -> PyRppgSignal: ...

class PyWindowConfig:
    def __init__(
        self,
        window_duration_sec: Optional[float] = ...,
        step_sec: Optional[float] = ...,
        min_coverage: Optional[float] = ...,
    ) -> None: ...

class PyFeatureConfig:
    def __init__(
        self,
        window: Optional[PyWindowConfig] = ...,
        min_beats: Optional[int] = ...,
        min_respiration_cycles: Optional[int] = ...,
        min_scr_events: Optional[int] = ...,
        require_cardiac: Optional[bool] = ...,
        require_respiration: Optional[bool] = ...,
        require_eda: Optional[bool] = ...,
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

def extract_features(input: PyMultimodalInput, config: Optional[PyFeatureConfig] = ...) -> List[PyMultimodalFeatureVector]: ...

class PyRsaConfig:
    min_valid_beats: Optional[int]
    def __init__(self, min_valid_beats: Optional[int] = ...) -> None: ...

class PyPulseTimingConfig:
    min_delay_sec: Optional[float]
    max_delay_sec: Optional[float]
    def __init__(
        self,
        min_delay_sec: Optional[float] = ...,
        max_delay_sec: Optional[float] = ...,
    ) -> None: ...

class PyPulseTimingResult:
    ecg_peak_index: int
    ppg_peak_index: int
    ecg_timestamp_sec: float
    ppg_timestamp_sec: float
    pulse_delay_sec: float

class PyCardiacRespiratoryEvent:
    r_peak_index: int
    timestamp_sec: float
    respiratory_phase: float
    rr_interval_sec: Optional[float]
    heart_rate_bpm: Optional[float]

class PyScrCardiorespiratoryAssociation:
    scr_peak_index: int
    scr_peak_time_sec: float
    scr_amplitude: float
    respiratory_phase_rad: Optional[float]
    nearest_r_peak_time_sec: Optional[float]
    cardiac_delay_sec: Optional[float]

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
) -> List[PyPulseTimingResult]: ...
def eda_cardiorespiratory_association(
    scr_events: List[PyScrEvent],
    eda_sampling_rate: float,
    eda_offset_sec: float,
    r_peaks: List[int],
    ecg_sampling_rate: float,
    ecg_offset_sec: float,
    rsp_cycles: List[PyRespirationCycle],
    rsp_sampling_rate: float,
    rsp_offset_sec: float,
) -> List[PyScrCardiorespiratoryAssociation]: ...
def respiratory_phase_at_time(
    rsp_cycles: List[PyRespirationCycle],
    timestamp_sec: float,
    rsp_sampling_rate: float,
    rsp_offset_sec: float,
) -> Optional[float]: ...
def evaluate_ecg_quality(r_peaks: List[int], sampling_rate: float, signal_duration_sec: float) -> PyModalityQuality: ...
def evaluate_rsp_quality(cycles_count: int, signal_duration_sec: float) -> PyModalityQuality: ...
def multimodal_quality(
    ecg_quality: Optional[PyModalityQuality] = ...,
    ppg_quality: Optional[PyModalityQuality] = ...,
    eda_quality: Optional[PyModalityQuality] = ...,
    rsp_quality: Optional[PyModalityQuality] = ...,
) -> PyMultimodalQuality: ...
