"""Type stubs for native extension _lamina."""

from typing import Optional, List, Any
import numpy as np
from numpy.typing import NDArray

__version__: str

class PyPeakDetectionConfig:
    min_distance_sec: float
    min_height: Optional[float]
    def __init__(self, min_distance_sec: float = ..., min_height: Optional[float] = ...) -> None: ...

def smooth_moving_average(signal: NDArray[np.float64], window_size: int) -> NDArray[np.float64]: ...
def filter(signal: NDArray[np.float64], sampling_rate: float, low_cutoff: Optional[float] = ..., high_cutoff: Optional[float] = ..., order: int = ..., btype: str = ...) -> NDArray[np.float64]: ...
def filtfilt(signal: NDArray[np.float64], sampling_rate: float, low_cutoff: Optional[float] = ..., high_cutoff: Optional[float] = ..., order: int = ..., btype: str = ...) -> NDArray[np.float64]: ...
def findpeaks(signal: NDArray[np.float64], sampling_rate: float, config: Optional[PyPeakDetectionConfig] = ...) -> List[int]: ...
def findpeaks_mask(signal: NDArray[np.float64], sampling_rate: float, config: Optional[PyPeakDetectionConfig] = ...) -> List[bool]: ...

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

def eda_clean(signal: NDArray[np.float64], sampling_rate: float) -> NDArray[np.float64]: ...
def eda_decompose(signal: NDArray[np.float64], sampling_rate: float, config: Optional[PyEdaDecompositionConfig] = ...) -> PyEdaComponents: ...
def eda_phasic(signal: NDArray[np.float64], sampling_rate: float) -> NDArray[np.float64]: ...
def eda_findpeaks(signal: NDArray[np.float64], sampling_rate: float, config: Optional[PyEdaPeakDetectionConfig] = ...) -> List[int]: ...
def eda_findpeaks_events(signal: NDArray[np.float64], sampling_rate: float, config: Optional[PyEdaPeakDetectionConfig] = ...) -> List[PyScrEvent]: ...

class PyRspCleaningConfig:
    lowpass_hz: float
    highpass_hz: float
    def __init__(self, lowpass_hz: float = ..., highpass_hz: float = ...) -> None: ...

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
def rsp_cycles(signal: NDArray[np.float64], sampling_rate: float, peaks: Optional[List[int]] = ..., config: Optional[PyRspProcessingConfig] = ...) -> List[PyRespirationCycle]: ...
def rsp_rate(rsp_cycles: List[PyRespirationCycle], sampling_rate: float, signal_length: Optional[int] = ...) -> List[float]: ...

def peaks_to_intervals(peaks: List[int], sampling_rate: float) -> List[float]: ...
def indices_to_intervals(indices: List[int], sampling_rate: float) -> List[float]: ...
def rmssd(rr_intervals: NDArray[np.float64]) -> float: ...
def mean_nn(rr_intervals: NDArray[np.float64]) -> float: ...
def sample_entropy(signal: NDArray[np.float64], m: int = ..., r: float = ...) -> float: ...

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
    def from_features(baseline_features: List[PyMultimodalFeatureVector]) -> PyAutonomicBaseline: ...
    @property
    def hr_bpm_stats(self) -> PyBaselineFeatureStats: ...

class PyAutonomicState:
    timestamp: float
    duration_sec: float
    arousal_index: float
    valence_proxy: float

class PyAutonomicEstimator:
    def __init__(self, config: Optional[PyAutonomicEstimatorConfig] = ...) -> None: ...
    def update(self, feature_vector: PyMultimodalFeatureVector) -> PyAutonomicState: ...
    def reset(self) -> None: ...

class PyVideoFrame:
    def __init__(self, width: int, height: int, data: bytes, timestamp_sec: float) -> None: ...

class PyVideoStream:
    def __init__(self, fps: float, duration_sec: Optional[float] = ...) -> None: ...
    def add_frame(self, frame: PyVideoFrame) -> None: ...

class PyRoi:
    def __init__(self, x: int, y: int, width: int, height: int) -> None: ...

class PyRppgConfig:
    def __init__(self) -> None: ...

class PyRppgSignal:
    timestamps_sec: Any
    waveform: Any
    sampling_rate_hz: float
    overall_quality: float

def extract_rppg(video: PyVideoStream, roi: PyRoi, config: Optional[PyRppgConfig] = ...) -> PyRppgSignal: ...

class PyFeatureConfig:
    def __init__(self) -> None: ...

class PyMultimodalInput:
    def __init__(self, ecg_signal: Optional[NDArray[np.float64]] = ..., ppg_signal: Optional[NDArray[np.float64]] = ..., eda_signal: Optional[NDArray[np.float64]] = ..., rsp_signal: Optional[NDArray[np.float64]] = ...) -> None: ...

class PyMultimodalFeatureVector:
    hr_bpm: Optional[float]
    sdnn_ms: Optional[float]
    rmssd_ms: Optional[float]
    to_dict(self) -> dict: ...
    to_numpy(self) -> NDArray[np.float64]: ...

def extract_features(inputs: PyMultimodalInput, sampling_rate: float, config: Optional[PyFeatureConfig] = ...) -> List[PyMultimodalFeatureVector]: ...

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

class PyMultimodalQuality:
    ecg_quality: Optional[PyModalityQuality]
    ppg_quality: Option[PyModalityQuality]
    eda_quality: Optional[PyModalityQuality]
    rsp_quality: Optional[PyModalityQuality]
    overall_quality: float

def rsa(r_peaks: List[int], ecg_sampling_rate: float, ecg_offset_sec: float, rsp_cycles: List[PyRespirationCycle], rsp_sampling_rate: float, rsp_offset_sec: float) -> PyRsaResult: ...
def cardiorespiratory_phase_coupling(phases: List[float]) -> PyPhaseCouplingResult: ...
def multimodal_quality(ecg_quality: Optional[PyModalityQuality] = ..., ppg_quality: Optional[PyModalityQuality] = ..., eda_quality: Optional[PyModalityQuality] = ..., rsp_quality: Optional[PyModalityQuality] = ...) -> PyMultimodalQuality: ...
