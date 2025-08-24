# Temperature Drift Calculation

This module implements robust temperature drift estimation for tVNS modeling with significant improvements over the original implementation.

## Implementation: `signal/temp.py::compute_temp_drift()`

### Key Features

1. **Time-Based Smoothing**: Uses configurable smoothing windows in seconds (default 30s) rather than fixed sample counts, making it appropriate for hour-scale drift analysis.

2. **Robust Slope Estimation**: Theil-Sen estimator with automatic fallback to least squares, plus confidence interval support.

3. **Performance Optimization**: Automatic downsampling for long signals to avoid O(N²) complexity, with fast alternatives available.

4. **Flexible Data Handling**: Supports irregular sampling, NaN/inf handling, and explicit time vectors.

5. **Configurable Robustness**: Adjustable polynomial order, smoothing duration, and robust vs. fast estimation modes.

### Usage

```python
from signal.temp import compute_temp_drift, estimate_drift_fast
import numpy as np

# Basic usage with default parameters
fs = 4  # 4 Hz sampling rate
temp_signal = np.array([...])  # Your temperature data
drift_rate = compute_temp_drift(temp_signal, fs=fs)
print(f"Temperature drift: {drift_rate:.4f} °C/hour")

# Advanced usage with custom parameters
drift_rate = compute_temp_drift(
    temp_signal, 
    fs=fs,
    smooth_seconds=60.0,        # 1-minute smoothing window
    polyorder=3,                # Higher polynomial order
    handle_nans='interpolate',  # Interpolate over gaps
    return_confidence=True      # Include confidence interval
)

# For irregular sampling, provide explicit time vector
t_seconds = np.array([...])  # Time in seconds
drift_rate = compute_temp_drift(temp_signal, t=t_seconds)

# Fast estimation for very long signals
drift_rate = estimate_drift_fast(temp_signal, fs=fs, method='quantile')
```

### Parameters

**Main Function: `compute_temp_drift()`**
- `temp`: Temperature signal data (array-like)
- `fs`: Sampling frequency in Hz (default: 4, ignored if `t` provided)
- `t`: Optional explicit time vector in seconds
- `smooth_seconds`: Smoothing window duration in seconds (default: 30.0)
- `polyorder`: Polynomial order for Savitzky-Golay filter (default: 2)
- `use_theilsen`: Use Theil-Sen estimator vs. least squares (default: True)
- `max_samples_theilsen`: Max samples before downsampling (default: 10000)
- `return_confidence`: Return confidence interval (default: False)
- `handle_nans`: NaN handling: 'drop', 'interpolate', or 'raise' (default: 'drop')

**Fast Function: `estimate_drift_fast()`**
- `temp`: Temperature signal data
- `t`: Optional time vector in seconds
- `fs`: Sampling frequency in Hz (default: 4)
- `method`: 'quantile' or 'huber' regression (default: 'quantile')

### Returns

- `drift`: Temperature drift rate in °C/hour
- If `return_confidence=True`: `(drift, (lower_ci, upper_ci))` tuple

### Performance Considerations

- **Short signals (< 10k samples)**: Use default `compute_temp_drift()` with Theil-Sen
- **Long signals (> 10k samples)**: Function automatically downsamples, or use `estimate_drift_fast()`
- **Very long signals (> 100k samples)**: Use `estimate_drift_fast()` with quantile regression

### Robustness Features

- **Outlier resistance**: Theil-Sen estimator provides robust slope estimation
- **NaN handling**: Multiple strategies for missing data
- **Boundary effects**: Explicit interpolation mode for Savitzky-Golay filtering
- **Time-based smoothing**: Window size adapts to signal duration, not sample count
- **Irregular sampling**: Handles non-uniform time spacing

### Requirements

- NumPy
- SciPy (for savgol_filter, theilslopes, and optimize)
- scikit-learn (optional, for Huber regression in fast estimation)

### Validation & Testing

The implementation provides empirical robustness against outliers with configurable trade-offs between accuracy and performance.

### Testing

Run the built-in test to see the function in action:
```python
from signal.temp import _test_drift_estimation
_test_drift_estimation()
```

This demonstrates:
- Drift estimation accuracy with clean and outlier-contaminated signals
- Confidence interval reporting
- Performance comparison between robust and fast methods

### Migration from Legacy Code

The new implementation is **not** backward compatible due to significant API improvements. Key changes:
- Time-based smoothing windows instead of fixed sample counts
- Additional parameters for robustness and performance control
- Better handling of edge cases and irregular sampling
- Confidence interval support

For equivalent behavior to a 21-sample window at 4 Hz:
```python
# Old: compute_temp_drift(temp, fs=4)  # 21 samples ≈ 5.25 seconds
# New: 
drift = compute_temp_drift(temp, fs=4, smooth_seconds=5.25, handle_nans='raise')
```