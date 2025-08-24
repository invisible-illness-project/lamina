"""
Temperature signal processing functions for tVNS modeling.

This module provides robust temperature drift estimation using 
Savitzky-Golay smoothing and Theil-Sen regression.
"""

import numpy as np
from scipy import signal as scipy_signal
from scipy import stats
import warnings
from typing import Optional, Tuple, Union


def compute_temp_drift(
    temp: np.ndarray,
    fs: float = 4,
    t: Optional[np.ndarray] = None,
    smooth_seconds: float = 30.0,
    polyorder: int = 2,
    use_theilsen: bool = True,
    max_samples_theilsen: int = 10000,
    return_confidence: bool = False,
    handle_nans: str = 'drop'
) -> Union[float, Tuple[float, Tuple[float, float]]]:
    """
    Compute temperature drift rate using robust methods.
    
    Applies Savitzky-Golay smoothing followed by robust slope estimation
    to provide drift estimates stable w.r.t. isolated outliers.
    
    Parameters
    ----------
    temp : array-like
        Temperature signal data
    fs : float, default=4
        Sampling frequency in Hz (ignored if t is provided)
    t : array-like, optional
        Time vector in seconds. If None, assumes uniform sampling at fs Hz
    smooth_seconds : float, default=30.0
        Smoothing window duration in seconds (converted to samples)
    polyorder : int, default=2
        Polynomial order for Savitzky-Golay filter
    use_theilsen : bool, default=True
        Use Theil-Sen estimator. If False, uses least squares
    max_samples_theilsen : int, default=10000
        Maximum samples for Theil-Sen (downsamples if exceeded to avoid O(N²) slowdown)
    return_confidence : bool, default=False
        Return confidence interval along with slope estimate
    handle_nans : str, default='drop'
        How to handle NaN/inf values: 'drop', 'interpolate', or 'raise'
        
    Returns
    -------
    drift : float or tuple
        Temperature drift rate in °C/hour. If return_confidence=True,
        returns (drift, (lower_ci, upper_ci))
        
    Notes
    -----
    The function uses time-based smoothing windows and robust slope estimation.
    For signals longer than max_samples_theilsen, the function downsamples
    before slope fitting to maintain reasonable computation time.
    
    Robustness claims are based on empirical testing with synthetic signals
    containing isolated 3σ outliers.
    """
    temp = np.asarray(temp, dtype=float)
    
    # Handle time vector
    if t is None:
        t = np.arange(len(temp)) / fs  # Time in seconds
    else:
        t = np.asarray(t, dtype=float)
        if len(t) != len(temp):
            raise ValueError("Time vector must have same length as temperature data")
    
    # Handle non-finite values
    if handle_nans == 'drop':
        finite_mask = np.isfinite(temp) & np.isfinite(t)
        if not np.any(finite_mask):
            raise ValueError("No finite values in temperature or time data")
        temp = temp[finite_mask]
        t = t[finite_mask]
    elif handle_nans == 'interpolate':
        finite_mask = np.isfinite(temp) & np.isfinite(t)
        if not np.any(finite_mask):
            raise ValueError("No finite values to interpolate from")
        if not np.all(finite_mask):
            # Simple linear interpolation for gaps
            temp = np.interp(t, t[finite_mask], temp[finite_mask])
    elif handle_nans == 'raise':
        if not np.all(np.isfinite(temp)) or not np.all(np.isfinite(t)):
            raise ValueError("Non-finite values found in data")
    else:
        raise ValueError("handle_nans must be 'drop', 'interpolate', or 'raise'")
    
    if len(temp) < 3:
        raise ValueError("Need at least 3 samples after NaN handling")
    
    # Calculate smoothing window in samples
    if len(t) > 1:
        median_dt = np.median(np.diff(t))
        window_samples = int(smooth_seconds / median_dt)
    else:
        window_samples = int(smooth_seconds * fs)
    
    # Ensure window requirements for Savitzky-Golay
    window_samples = max(polyorder + 1, window_samples)
    if window_samples % 2 == 0:
        window_samples += 1  # Must be odd
    window_samples = min(window_samples, len(temp))
    if window_samples < polyorder + 1:
        window_samples = polyorder + 1
        if window_samples > len(temp):
            raise ValueError(f"Signal too short for polyorder {polyorder}")
    
    # Apply Savitzky-Golay smoothing with explicit mode
    try:
        temp_smooth = scipy_signal.savgol_filter(
            temp, 
            window_length=window_samples, 
            polyorder=polyorder,
            mode='interp'
        )
    except ValueError as e:
        raise ValueError(f"Savitzky-Golay filtering failed: {e}")
    
    # Convert time to hours for slope units
    time_hours = t / 3600.0
    
    # Handle long signals for Theil-Sen by downsampling
    if use_theilsen and len(temp_smooth) > max_samples_theilsen:
        warnings.warn(
            f"Signal length ({len(temp_smooth)}) exceeds max_samples_theilsen "
            f"({max_samples_theilsen}). Downsampling for slope estimation.",
            UserWarning
        )
        downsample_factor = len(temp_smooth) // max_samples_theilsen
        indices = np.arange(0, len(temp_smooth), downsample_factor)
        temp_smooth = temp_smooth[indices]
        time_hours = time_hours[indices]
    
    # Robust slope estimation
    if use_theilsen:
        try:
            result = stats.theilslopes(temp_smooth, time_hours)
            slope = result[0]
            if return_confidence:
                # Theil-Sen confidence interval
                lower_ci, upper_ci = result[2], result[3]
                return slope, (lower_ci, upper_ci)
        except Exception as e:
            warnings.warn(f"Theil-Sen estimation failed: {e}. Falling back to least squares.", UserWarning)
            use_theilsen = False
    
    if not use_theilsen:
        # Least squares fallback
        slope, intercept = np.polyfit(time_hours, temp_smooth, 1)
        if return_confidence:
            # Simple confidence interval using standard error
            residuals = temp_smooth - (slope * time_hours + intercept)
            mse = np.mean(residuals**2)
            se = np.sqrt(mse / np.sum((time_hours - np.mean(time_hours))**2))
            # 95% confidence interval (approximate)
            margin = 1.96 * se
            return slope, (slope - margin, slope + margin)
    
    return slope


def estimate_drift_fast(
    temp: np.ndarray, 
    t: Optional[np.ndarray] = None, 
    fs: float = 4,
    method: str = 'quantile'
) -> float:
    """
    Fast temperature drift estimation for long signals.
    
    Uses quantile regression or Huber regression for O(N log N) complexity
    instead of O(N²) Theil-Sen.
    
    Parameters
    ----------
    temp : array-like
        Temperature signal data
    t : array-like, optional
        Time vector in seconds
    fs : float, default=4
        Sampling frequency in Hz
    method : str, default='quantile'
        Method: 'quantile' (median regression) or 'huber'
        
    Returns
    -------
    drift : float
        Temperature drift rate in °C/hour
    """
    temp = np.asarray(temp, dtype=float)
    
    if t is None:
        t = np.arange(len(temp)) / fs
    else:
        t = np.asarray(t, dtype=float)
    
    # Simple finite value handling
    finite_mask = np.isfinite(temp) & np.isfinite(t)
    temp = temp[finite_mask]
    t = t[finite_mask]
    
    if len(temp) < 2:
        raise ValueError("Need at least 2 finite samples")
    
    time_hours = t / 3600.0
    
    if method == 'quantile':
        # Quantile regression (median) - more robust than least squares
        # Simple implementation using weighted least squares approximation
        from scipy.optimize import minimize_scalar
        
        def quantile_loss(slope):
            residuals = temp - slope * time_hours
            return np.sum(np.abs(residuals))
        
        result = minimize_scalar(quantile_loss)
        return result.x
        
    elif method == 'huber':
        # Huber regression - robust to outliers, faster than Theil-Sen
        try:
            from sklearn.linear_model import HuberRegressor
            huber = HuberRegressor(fit_intercept=True, alpha=0.0)
            slope = huber.fit(time_hours.reshape(-1, 1), temp).coef_[0]
            return slope
        except ImportError:
            warnings.warn("scikit-learn not available, falling back to quantile method")
            return estimate_drift_fast(temp, t, fs, method='quantile')
    
    else:
        raise ValueError("method must be 'quantile' or 'huber'")


def _test_drift_estimation():
    """
    Simple test to demonstrate the improved drift estimation.
    
    Shows robustness to outliers and handling of different signal types.
    """
    # Generate test signal with known drift
    fs = 4  # Hz
    duration = 3600  # 1 hour
    t = np.arange(0, duration, 1/fs)
    
    # True drift: 0.1 °C/hour
    true_drift = 0.1
    temp_clean = 37.0 + true_drift * (t / 3600) + 0.01 * np.random.randn(len(t))
    
    # Add some outliers
    temp_with_outliers = temp_clean.copy()
    outlier_indices = np.random.choice(len(t), size=5, replace=False)
    temp_with_outliers[outlier_indices] += 3 * np.std(temp_clean) * np.random.choice([-1, 1], 5)
    
    # Test different methods
    print("Temperature Drift Estimation Test")
    print(f"True drift: {true_drift:.4f} °C/hour")
    print()
    
    # Improved method with time-based smoothing
    drift_improved = compute_temp_drift(temp_clean, fs=fs, smooth_seconds=60)
    print(f"Improved method (clean): {drift_improved:.4f} °C/hour")
    
    drift_improved_outliers = compute_temp_drift(temp_with_outliers, fs=fs, smooth_seconds=60)
    print(f"Improved method (outliers): {drift_improved_outliers:.4f} °C/hour")
    
    # With confidence interval
    drift_ci, (lower, upper) = compute_temp_drift(
        temp_with_outliers, fs=fs, smooth_seconds=60, return_confidence=True
    )
    print(f"With 95% CI: {drift_ci:.4f} [{lower:.4f}, {upper:.4f}] °C/hour")
    
    # Fast method for comparison
    drift_fast = estimate_drift_fast(temp_with_outliers, fs=fs, method='quantile')
    print(f"Fast quantile method: {drift_fast:.4f} °C/hour")


if __name__ == "__main__":
    _test_drift_estimation()