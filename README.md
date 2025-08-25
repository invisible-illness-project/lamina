# Lamina - Physiological Signal Processing Python Library

A robust signal processing library for processing physiological data from sensors for designed for production machine learning and physiological simulation applications.

## Overview

This project provides production-ready signal processing functions focusing on robust handling of physiological signals with comprehensive artifact removal, normalization, and quality assessment. The library is optimized for real-world data challenges including motion artifacts, sensor drift, and cross-subject variability.

## Features

- **Robust Temperature Drift Estimation** - Advanced drift calculation with outlier resistance
- **Production EDA Signal Cleaning** - Comprehensive electrodermal activity preprocessing
- **Motion Artifact Detection** - Accelerometer-based quality assessment
- **Cross-subject Normalization** - Consistent scaling for ML applications
- **Real-time Processing** - Optimized for streaming applications
- **Comprehensive Testing** - Validated performance across diverse conditions

## Quick Start

```python
from signal.temp import compute_temp_drift
from signal.eda import clean_eda
import numpy as np

# Temperature drift analysis
temp_signal = np.array([...])  # Temperature data
drift_rate = compute_temp_drift(temp_signal, fs=4)
print(f"Temperature drift: {drift_rate:.4f} °C/hour")

# EDA signal cleaning
eda_raw = np.array([...])      # Raw EDA signal
acc_mag = np.array([...])      # Accelerometer magnitude
eda_clean, mask_good = clean_eda(eda_raw, acc_mag, fs=4)
print(f"EDA cleaned: {np.mean(mask_good)*100:.1f}% good samples")
```

## Dependencies

- **NumPy** - Array operations and basic statistics
- **SciPy** - Signal processing, filtering, and robust statistics
- **scikit-learn** (optional) - Advanced regression methods

Install dependencies:
```bash
pip install numpy scipy scikit-learn
```

---

## Temperature Drift Estimation

### `signal/temp.py::compute_temp_drift()`

Robust temperature drift estimation using Savitzky-Golay smoothing and Theil-Sen regression, designed for hour-scale physiological drift analysis.

#### Key Features

1. **Time-Based Smoothing**: Configurable smoothing windows in seconds (default 30s) rather than fixed sample counts
2. **Robust Slope Estimation**: Theil-Sen estimator with automatic fallback to least squares
3. **Performance Optimization**: Automatic downsampling for long signals to avoid O(N²) complexity
4. **Flexible Data Handling**: Supports irregular sampling, NaN/inf handling, and explicit time vectors
5. **Confidence Intervals**: Optional uncertainty quantification for drift estimates

#### Usage Examples

```python
from signal.temp import compute_temp_drift, estimate_drift_fast

# Basic usage
temp_signal = np.array([...])  # Your temperature data
drift_rate = compute_temp_drift(temp_signal, fs=4)

# Advanced usage with custom parameters
drift_rate = compute_temp_drift(
    temp_signal, 
    fs=4,
    smooth_seconds=60.0,        # 1-minute smoothing window
    polyorder=3,                # Higher polynomial order
    handle_nans='interpolate',  # Interpolate over gaps
    return_confidence=True      # Include confidence interval
)

# For irregular sampling
t_seconds = np.array([...])  # Time in seconds
drift_rate = compute_temp_drift(temp_signal, t=t_seconds)

# Fast estimation for very long signals
drift_rate = estimate_drift_fast(temp_signal, fs=4, method='quantile')
```

---

## EDA Signal Processing

### `signal/eda.py::clean_eda(eda, acc_mag, fs=4)`

Production-ready EDA signal cleaning pipeline with motion artifact detection, spike removal, and cross-subject normalization.

#### Key Features

1. **Median-based Spike Removal**: 5-sample median filter with robust MAD-based threshold detection
2. **Motion Artifact Detection**: Accelerometer-based quality mask using robust z-score thresholding (|z| > 2.5)
3. **Low-pass Filtering**: 1.0 Hz Butterworth filter (4th order) for noise reduction
4. **Percentile Scaling**: Normalizes signals to [0,1] range using 1st-99th percentile bounds
5. **Quality Assessment**: Comprehensive metrics for validation and monitoring

#### Processing Pipeline

The `clean_eda()` function applies the following steps in sequence:

1. **Spike Removal**: `median_despike(eda, k=5)` - Removes isolated spikes using median filtering
2. **Motion Detection**: `create_motion_mask(acc_mag, threshold=2.5)` - Identifies high-motion periods
3. **Interpolation**: Linear interpolation over motion-contaminated samples
4. **Low-pass Filter**: 1.0 Hz Butterworth filter for noise reduction
5. **Normalization**: Percentile-based scaling to [0,1] range

#### Usage Examples

```python
from signal.eda import clean_eda, assess_cleaning_quality

# Basic usage
eda_raw = np.array([...])      # Raw EDA signal
acc_mag = np.array([...])      # Accelerometer magnitude
fs = 4                         # Sampling frequency (Hz)

# Clean the signal
eda_clean, mask_good = clean_eda(eda_raw, acc_mag, fs=fs)
# eda_clean: cleaned signal in [0,1] range
# mask_good: boolean mask (True = clean, False = motion artifact)

# Quality assessment
quality_metrics = assess_cleaning_quality(eda_raw, eda_clean, mask_good)
print(f"Motion artifacts: {quality_metrics['motion_percentage']:.1f}%")
print(f"Noise reduction: {quality_metrics['noise_reduction_db']:.1f} dB")
```

---

## Testing & Validation

### Temperature Drift Tests

```python
from signal.temp import _test_drift_estimation
_test_drift_estimation()
```

### EDA Processing Tests

```python
# Run comprehensive EDA test suite
cd signal/test && python test_eda.py
```

### Temperature Drift Tests

```python
# Run temperature drift test suite
cd signal/test && python test_temp_drift.py
```

---

## API Reference

### Temperature Processing

| Function | Purpose | Key Parameters |
|----------|---------|----------------|
| `compute_temp_drift()` | Main drift estimation | `temp, fs, smooth_seconds, use_theilsen` |
| `estimate_drift_fast()` | Fast estimation for long signals | `temp, fs, method` |

### EDA Processing

| Function | Purpose | Key Parameters |
|----------|---------|----------------|
| `clean_eda()` | Complete cleaning pipeline | `eda, acc_mag, fs` |
| `median_despike()` | Spike removal | `x, k, threshold` |
| `create_motion_mask()` | Motion detection | `acc_mag, threshold` |
| `percentile_scale()` | Signal normalization | `x, p1, p99` |
| `assess_cleaning_quality()` | Quality metrics | `eda_raw, eda_clean, mask_good` |

---

## Project Structure

```
tVNS-Modeling-Playground/
├── README.md                 
├── signal/                   # Main signal processing module
│   ├── __init__.py          # Package initialization
│   ├── temp.py              # Temperature drift estimation
│   ├── eda.py               # EDA signal cleaning
│   └── test/                # Test suite
│       ├── __init__.py      # Test package initialization
│       ├── test_eda.py      # EDA processing tests
│       └── test_temp_drift.py # Temperature drift tests
```

---

## Contributing

When contributing to this project:

1. **Maintain robustness**: All functions should handle edge cases gracefully
2. **Include comprehensive tests**: New features require validation against known conditions
3. **Document performance characteristics**: Include timing and accuracy metrics
4. **Follow the production-ready philosophy**: Code should be ready for real-world deployment

---

## License

This project is part of the Roeh Health ANS Initiative 

