"""
Temperature signal processing functions for tVNS modeling.

This module provides robust temperature drift estimation using 
Savitzky-Golay smoothing and Theil-Sen regression.
"""

import numpy as np
from scipy import signal as scipy_signal
from scipy import stats


def compute_temp_drift(temp, fs=4):
    """
    Compute temperature drift rate using robust methods.
    
    Applies Savitzky-Golay (21,2) smoothing followed by Theil-Sen slope estimation
    to provide drift estimates stable w.r.t. isolated outliers.
    
    Parameters
    ----------
    temp : array-like
        Temperature signal data
    fs : float, default=4
        Sampling frequency in Hz
        
    Returns
    -------
    drift : float
        Temperature drift rate in °C/hour
        
    Notes
    -----
    The function is designed to be robust against outliers, with drift estimates
    changing by ≤10% when a single spike (3σ outlier) is added to the signal.
    """
    temp = np.asarray(temp)
    
    if len(temp) < 21:
        raise ValueError("Temperature signal must have at least 21 samples for Savitzky-Golay filtering")
    
    # Apply Savitzky-Golay (21,2) smoothing
    # Window length must be odd and ≥ polyorder + 1
    window_length = min(21, len(temp) if len(temp) % 2 == 1 else len(temp) - 1)
    if window_length < 3:
        window_length = 3
    
    temp_smooth = scipy_signal.savgol_filter(temp, window_length=window_length, polyorder=2)
    
    # Create time vector in hours
    time_hours = np.arange(len(temp_smooth)) / (fs * 3600.0)  # Convert samples to hours
    
    # Use Theil-Sen estimator for robust slope estimation
    # Scipy's implementation of Theil-Sen estimator
    slope, intercept, _, _ = stats.theilslopes(temp_smooth, time_hours)
    
    # Return slope in °C/hour
    return slope