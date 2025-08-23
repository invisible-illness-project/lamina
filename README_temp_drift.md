# Temperature Drift Calculation

This module implements robust temperature drift estimation for tVNS modeling as specified in issue #4.

## Implementation: `signal/temp.py::compute_temp_drift(temp, fs=4)`

### Features

1. **Savitzky-Golay (21,2) Smoothing**: Applies a 21-point window with 2nd-order polynomial smoothing to reduce noise while preserving signal characteristics.

2. **Theil-Sen Slope Estimation**: Uses the robust Theil-Sen estimator to calculate temperature drift in °C/hour, which is resistant to outliers.

3. **Outlier Robustness**: Designed to meet the acceptance criteria of ≤10% change when adding a single 3σ outlier.

### Usage

```python
from signal.temp import compute_temp_drift
import numpy as np

# Example temperature signal
fs = 4  # 4 Hz sampling rate
temp_signal = np.array([...])  # Your temperature data
drift_rate = compute_temp_drift(temp_signal, fs=fs)
print(f"Temperature drift: {drift_rate:.4f} °C/hour")
```

### Parameters

- `temp`: Temperature signal data (array-like)
- `fs`: Sampling frequency in Hz (default: 4)

### Returns

- `drift`: Temperature drift rate in °C/hour

### Requirements

- NumPy
- SciPy (for savgol_filter and theilslopes)

### Validation

The implementation passes all acceptance criteria:
- Robust against isolated outliers (≤10% change with 3σ spikes)
- Handles signals with minimum 21 samples
- Accurately estimates positive, negative, and zero drift rates

### Testing

Run tests with:
```bash
python test_temp_drift.py
```

Tests validate:
- Basic functionality with known drift rates
- Robustness against single and multiple outliers  
- Edge cases (zero drift, negative drift, minimum length)