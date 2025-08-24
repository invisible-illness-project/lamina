"""
EDA signal processing functions for tVNS modeling.

This module provides robust EDA signal cleaning suitable for ML and simulation,
including motion artifact removal, filtering, and normalization.
"""

import numpy as np
from scipy import signal as scipy_signal
from scipy import ndimage
from typing import Tuple, Optional
import warnings


def percentile_scale(x: np.ndarray, p1: float = 1, p99: float = 99) -> np.ndarray:
    """
    Scale signal to [0, 1] using percentile-based normalization.
    
    Clips extreme values and scales based on percentile range to ensure
    stable value ranges across subjects while preserving signal dynamics.
    
    Parameters
    ----------
    x : array-like
        Input signal data
    p1 : float, default=1
        Lower percentile for clipping (e.g., 1st percentile)
    p99 : float, default=99
        Upper percentile for clipping (e.g., 99th percentile)
        
    Returns
    -------
    x_scaled : ndarray
        Scaled signal in range [0, 1]
        
    Notes
    -----
    This approach is more robust than min-max scaling as it's less sensitive
    to extreme outliers while maintaining consistent ranges across subjects.
    """
    x = np.asarray(x, dtype=float)
    
    # Handle edge cases
    if len(x) == 0:
        return x
    
    # Calculate percentiles on finite values only
    finite_mask = np.isfinite(x)
    if not np.any(finite_mask):
        warnings.warn("No finite values found in signal")
        return np.full_like(x, 0.5)  # Return middle value
    
    x_finite = x[finite_mask]
    
    # Get percentile bounds
    p_low = np.percentile(x_finite, p1)
    p_high = np.percentile(x_finite, p99)
    
    # Handle degenerate case where percentiles are equal
    if np.isclose(p_low, p_high):
        warnings.warn(f"Percentiles {p1} and {p99} are nearly equal ({p_low:.6f})")
        return np.full_like(x, 0.5)
    
    # Clip and scale
    x_clipped = np.clip(x, p_low, p_high)
    x_scaled = (x_clipped - p_low) / (p_high - p_low)
    
    return x_scaled


def median_despike(x: np.ndarray, k: int = 5, threshold: float = 3.0) -> np.ndarray:
    """
    Remove spikes using median filter-based detection and replacement.
    
    Detects spikes by comparing signal to its median-filtered version,
    then replaces detected spikes with median-filtered values.
    
    Parameters
    ----------
    x : array-like
        Input signal
    k : int, default=5
        Median filter kernel size (should be odd)
    threshold : float, default=3.0
        Spike detection threshold in standard deviations
        
    Returns
    -------
    x_despiked : ndarray
        Signal with spikes removed
        
    Notes
    -----
    This method preserves signal shape better than simple median filtering
    while effectively removing isolated spikes and artifacts.
    """
    x = np.asarray(x, dtype=float)
    
    if len(x) < k:
        warnings.warn(f"Signal length ({len(x)}) shorter than kernel size ({k})")
        return x.copy()
    
    # Ensure odd kernel size
    if k % 2 == 0:
        k += 1
    
    # Apply median filter
    x_median = ndimage.median_filter(x, size=k, mode='nearest')
    
    # Calculate residuals
    residuals = x - x_median
    
    # Robust threshold using MAD (Median Absolute Deviation)
    mad = np.median(np.abs(residuals - np.median(residuals)))
    if mad == 0:
        # Fallback to standard deviation if MAD is zero
        mad = np.std(residuals)
    
    spike_threshold = threshold * mad * 1.4826  # 1.4826 converts MAD to std equivalent
    
    # Detect spikes
    spike_mask = np.abs(residuals) > spike_threshold
    
    # Replace spikes with median-filtered values
    x_despiked = x.copy()
    x_despiked[spike_mask] = x_median[spike_mask]
    
    return x_despiked


def lowpass_filter(x: np.ndarray, cutoff: float, fs: float, order: int = 4) -> np.ndarray:
    """
    Apply low-pass Butterworth filter to signal.
    
    Parameters
    ----------
    x : array-like
        Input signal
    cutoff : float
        Cutoff frequency in Hz
    fs : float
        Sampling frequency in Hz
    order : int, default=4
        Filter order
        
    Returns
    -------
    x_filtered : ndarray
        Low-pass filtered signal
    """
    x = np.asarray(x, dtype=float)
    
    # Design filter
    nyquist = fs / 2
    if cutoff >= nyquist:
        warnings.warn(f"Cutoff frequency ({cutoff} Hz) >= Nyquist frequency ({nyquist} Hz)")
        return x.copy()
    
    # Butterworth low-pass filter
    sos = scipy_signal.butter(order, cutoff / nyquist, btype='low', output='sos')
    
    # Apply zero-phase filter
    x_filtered = scipy_signal.sosfiltfilt(sos, x)
    
    return x_filtered


def create_motion_mask(acc_mag: np.ndarray, threshold: float = 2.5) -> np.ndarray:
    """
    Create motion artifact mask based on accelerometer magnitude.
    
    Identifies periods of high motion that likely contaminate EDA signal.
    
    Parameters
    ----------
    acc_mag : array-like
        Accelerometer magnitude signal
    threshold : float, default=2.5
        Z-score threshold for motion detection
        
    Returns
    -------
    mask_good : ndarray of bool
        Boolean mask where True indicates clean (low-motion) samples
        
    Notes
    -----
    Uses robust z-score based on median and MAD to avoid influence
    of extreme motion artifacts on the threshold calculation.
    """
    acc_mag = np.asarray(acc_mag, dtype=float)
    
    # Handle edge cases
    if len(acc_mag) == 0:
        return np.array([], dtype=bool)
    
    # Calculate robust z-scores using median and MAD
    median_acc = np.median(acc_mag)
    mad_acc = np.median(np.abs(acc_mag - median_acc))
    
    if mad_acc == 0:
        # If MAD is zero, use standard deviation
        mad_acc = np.std(acc_mag)
        if mad_acc == 0:
            # If both MAD and std are zero, all samples are good
            return np.ones_like(acc_mag, dtype=bool)
    
    # Convert MAD to standard deviation equivalent
    sigma_equivalent = mad_acc * 1.4826
    
    # Calculate robust z-scores
    z_scores = np.abs(acc_mag - median_acc) / sigma_equivalent
    
    # Create mask (True for good samples, False for high motion)
    mask_good = z_scores <= threshold
    
    return mask_good


def interpolate_masked_samples(x: np.ndarray, mask_good: np.ndarray, 
                              method: str = 'linear') -> np.ndarray:
    """
    Interpolate over masked (bad) samples.
    
    Parameters
    ----------
    x : array-like
        Input signal
    mask_good : array-like of bool
        Boolean mask where True indicates good samples
    method : str, default='linear'
        Interpolation method: 'linear', 'nearest', or 'cubic'
        
    Returns
    -------
    x_interp : ndarray
        Signal with masked samples interpolated
        
    Notes
    -----
    If too few good samples exist for interpolation, returns original signal
    with a warning.
    """
    x = np.asarray(x, dtype=float)
    mask_good = np.asarray(mask_good, dtype=bool)
    
    if len(x) != len(mask_good):
        raise ValueError("Signal and mask must have same length")
    
    # If all samples are good, return original
    if np.all(mask_good):
        return x.copy()
    
    # If no samples are good, return original with warning
    if not np.any(mask_good):
        warnings.warn("No good samples for interpolation")
        return x.copy()
    
    # Create time indices
    indices = np.arange(len(x))
    good_indices = indices[mask_good]
    bad_indices = indices[~mask_good]
    
    if len(good_indices) < 2:
        warnings.warn("Too few good samples for interpolation")
        return x.copy()
    
    # Interpolate bad samples
    x_interp = x.copy()
    x_interp[bad_indices] = np.interp(bad_indices, good_indices, x[good_indices])
    
    return x_interp


def clean_eda(eda: np.ndarray, acc_mag: np.ndarray, fs: float = 4) -> Tuple[np.ndarray, np.ndarray]:
    """
    Robust EDA signal cleaning pipeline for ML and simulation.
    
    Applies comprehensive cleaning including spike removal, motion artifact
    detection, filtering, and normalization to produce production-ready EDA data.
    
    Parameters
    ----------
    eda : array-like
        Raw EDA signal data
    acc_mag : array-like
        Accelerometer magnitude for motion artifact detection
    fs : float, default=4
        Sampling frequency in Hz
        
    Returns
    -------
    eda_clean : ndarray
        Cleaned and normalized EDA signal (0-1 range)
    mask_good : ndarray of bool
        Boolean mask indicating clean (non-motion) samples
        
    Processing Steps
    ----------------
    1. Median-based spike removal (k=5 samples)
    2. Motion artifact detection using ACC magnitude (|z| > 2.5)
    3. Linear interpolation over motion-contaminated samples
    4. Low-pass filtering at 1.0 Hz (4th order Butterworth)
    5. Percentile-based scaling (1st-99th percentile → [0,1])
        
    Notes
    -----
    This pipeline is designed for production ML applications requiring:
    - Consistent value ranges across subjects
    - Robust handling of motion artifacts
    - Preserved physiological signal characteristics
    - Minimal parameter tuning requirements
        
    The motion mask can be used for quality assessment or to exclude
    high-motion periods from analysis.
    """
    eda = np.asarray(eda, dtype=float)
    acc_mag = np.asarray(acc_mag, dtype=float)
    
    if len(eda) != len(acc_mag):
        raise ValueError("EDA and accelerometer signals must have same length")
    
    if len(eda) == 0:
        return np.array([]), np.array([], dtype=bool)
    
    # Step 1: Remove spikes using median filter
    eda_despiked = median_despike(eda, k=5, threshold=3.0)
    
    # Step 2: Create motion artifact mask
    mask_good = create_motion_mask(acc_mag, threshold=2.5)
    
    # Step 3: Interpolate over motion-contaminated samples
    eda_interp = interpolate_masked_samples(eda_despiked, mask_good, method='linear')
    
    # Step 4: Apply low-pass filter (1.0 Hz cutoff)
    eda_filtered = lowpass_filter(eda_interp, cutoff=1.0, fs=fs, order=4)
    
    # Step 5: Percentile-based scaling to [0, 1]
    eda_clean = percentile_scale(eda_filtered, p1=1, p99=99)
    
    return eda_clean, mask_good


def assess_cleaning_quality(eda_raw: np.ndarray, eda_clean: np.ndarray, 
                           mask_good: np.ndarray) -> dict:
    """
    Assess the quality of EDA cleaning process.
    
    Parameters
    ----------
    eda_raw : array-like
        Original raw EDA signal
    eda_clean : array-like
        Cleaned EDA signal
    mask_good : array-like of bool
        Motion artifact mask
        
    Returns
    -------
    quality_metrics : dict
        Dictionary containing quality assessment metrics
    """
    eda_raw = np.asarray(eda_raw)
    eda_clean = np.asarray(eda_clean)
    mask_good = np.asarray(mask_good, dtype=bool)
    
    metrics = {}
    
    # Basic statistics
    metrics['samples_total'] = len(eda_raw)
    metrics['samples_good'] = np.sum(mask_good)
    metrics['motion_percentage'] = (1 - np.mean(mask_good)) * 100
    
    # Signal range and scaling
    metrics['raw_range'] = np.ptp(eda_raw)
    metrics['clean_range'] = np.ptp(eda_clean)
    metrics['clean_min'] = np.min(eda_clean)
    metrics['clean_max'] = np.max(eda_clean)
    
    # Noise reduction (using standard deviation as proxy)
    metrics['raw_std'] = np.std(eda_raw)
    metrics['clean_std'] = np.std(eda_clean)
    metrics['noise_reduction_db'] = 20 * np.log10(metrics['raw_std'] / metrics['clean_std']) \
                                   if metrics['clean_std'] > 0 else np.inf
    
    return metrics
