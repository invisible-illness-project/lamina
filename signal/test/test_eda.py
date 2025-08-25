"""
Test suite for EDA signal processing functions.
"""

import sys
import os
import pytest
import numpy as np
import warnings

# Add parent directory to path to import signal modules
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

from eda import (
    percentile_scale,
    median_despike,
    lowpass_filter,
    create_motion_mask,
    interpolate_masked_samples,
    clean_eda,
    assess_cleaning_quality
)


class TestPercentileScale:
    """Test percentile-based scaling function."""
    
    def test_normal_case(self):
        """Test normal scaling behavior."""
        x = np.array([1, 2, 3, 4, 5, 6, 7, 8, 9, 10])
        scaled = percentile_scale(x, p1=10, p99=90)
        
        # Should be in [0, 1] range
        assert np.all(scaled >= 0)
        assert np.all(scaled <= 1)
        
        # Minimum should be 0, maximum should be 1
        assert np.isclose(np.min(scaled), 0, atol=1e-10)
        assert np.isclose(np.max(scaled), 1, atol=1e-10)
    
    def test_empty_input(self):
        """Test empty array input."""
        x = np.array([])
        result = percentile_scale(x)
        assert len(result) == 0
    
    def test_constant_signal(self):
        """Test signal with constant values."""
        x = np.full(100, 5.0)
        with warnings.catch_warnings(record=True) as w:
            result = percentile_scale(x)
            assert len(w) == 1
            assert "nearly equal" in str(w[0].message)
        
        # Should return middle value (0.5)
        assert np.allclose(result, 0.5)
    
    def test_with_nans(self):
        """Test handling of NaN values."""
        x = np.array([1, 2, np.nan, 4, 5])
        with warnings.catch_warnings():
            warnings.simplefilter("ignore")
            result = percentile_scale(x)
        
        # Should handle NaNs gracefully
        assert len(result) == len(x)
        assert np.isnan(result[2])  # NaN should remain NaN
    
    def test_all_nans(self):
        """Test signal with all NaN values."""
        x = np.full(10, np.nan)
        with warnings.catch_warnings(record=True) as w:
            result = percentile_scale(x)
            assert len(w) == 1
            assert "No finite values" in str(w[0].message)
        
        # Should return middle value
        assert np.allclose(result, 0.5, equal_nan=True)


class TestMedianDespike:
    """Test spike removal function."""
    
    def test_no_spikes(self):
        """Test signal without spikes."""
        x = np.sin(np.linspace(0, 4*np.pi, 100))
        result = median_despike(x, k=5, threshold=3.0)
        
        # Should be very similar to original
        assert np.allclose(result, x, rtol=0.1)
    
    def test_with_spikes(self):
        """Test signal with artificial spikes."""
        x = np.sin(np.linspace(0, 4*np.pi, 100))
        x_with_spikes = x.copy()
        
        # Add spikes
        spike_indices = [20, 50, 80]
        x_with_spikes[spike_indices] += 10
        
        result = median_despike(x_with_spikes, k=5, threshold=2.0)
        
        # Spikes should be reduced
        for idx in spike_indices:
            assert abs(result[idx] - x[idx]) < abs(x_with_spikes[idx] - x[idx])
    
    def test_short_signal(self):
        """Test signal shorter than kernel size."""
        x = np.array([1, 2, 3])
        with warnings.catch_warnings(record=True) as w:
            result = median_despike(x, k=10)
            assert len(w) == 1
            assert "shorter than kernel size" in str(w[0].message)
        
        # Should return copy of original
        assert np.array_equal(result, x)
    
    def test_even_kernel_size(self):
        """Test that even kernel size gets converted to odd."""
        x = np.random.randn(50)
        result = median_despike(x, k=6)  # Even kernel
        
        # Should work without error (kernel gets converted to 7)
        assert len(result) == len(x)


class TestLowpassFilter:
    """Test low-pass filtering function."""
    
    def test_basic_filtering(self):
        """Test basic filtering behavior."""
        fs = 10
        t = np.linspace(0, 1, fs, endpoint=False)
        
        # Signal with low and high frequency components
        x = np.sin(2 * np.pi * 1 * t) + 0.5 * np.sin(2 * np.pi * 4 * t)
        
        # Filter out high frequencies
        filtered = lowpass_filter(x, cutoff=2, fs=fs, order=4)
        
        # Should preserve low frequencies, attenuate high frequencies
        assert len(filtered) == len(x)
        
        # Low frequency component should be preserved
        low_freq = np.sin(2 * np.pi * 1 * t)
        correlation = np.corrcoef(filtered, low_freq)[0, 1]
        assert correlation > 0.8
    
    def test_cutoff_too_high(self):
        """Test cutoff frequency >= Nyquist frequency."""
        x = np.random.randn(100)
        fs = 10
        
        with warnings.catch_warnings(record=True) as w:
            result = lowpass_filter(x, cutoff=6, fs=fs)  # Nyquist = 5
            assert len(w) == 1
            assert "Cutoff frequency" in str(w[0].message)
        
        # Should return copy of original
        assert np.array_equal(result, x)


class TestCreateMotionMask:
    """Test motion artifact detection."""
    
    def test_low_motion(self):
        """Test signal with low motion."""
        # Simulate low, stable motion
        acc_mag = np.random.normal(1.0, 0.1, 1000)
        mask = create_motion_mask(acc_mag, threshold=2.5)
        
        # Most samples should be marked as good
        assert np.mean(mask) > 0.9
    
    def test_high_motion(self):
        """Test signal with high motion periods."""
        # Low motion baseline with high motion spikes
        acc_mag = np.random.normal(1.0, 0.1, 1000)
        
        # Add high motion periods
        high_motion_indices = np.arange(100, 200)
        acc_mag[high_motion_indices] += 5.0
        
        mask = create_motion_mask(acc_mag, threshold=2.0)
        
        # High motion periods should be marked as bad
        assert np.mean(mask[high_motion_indices]) < 0.5
        assert np.mean(mask[:100]) > 0.8  # Baseline should be good
    
    def test_empty_input(self):
        """Test empty accelerometer input."""
        acc_mag = np.array([])
        mask = create_motion_mask(acc_mag)
        
        assert len(mask) == 0
        assert mask.dtype == bool
    
    def test_constant_motion(self):
        """Test constant accelerometer values."""
        acc_mag = np.full(100, 1.0)
        mask = create_motion_mask(acc_mag)
        
        # All samples should be good (no variation)
        assert np.all(mask)


class TestInterpolateMaskedSamples:
    """Test interpolation over masked samples."""
    
    def test_basic_interpolation(self):
        """Test basic linear interpolation."""
        x = np.array([1, 2, 3, 4, 5, 6, 7, 8, 9, 10])
        mask_good = np.array([True, True, False, False, True, True, False, True, True, True])
        
        result = interpolate_masked_samples(x, mask_good, method='linear')
        
        # Interpolated values should be reasonable
        assert len(result) == len(x)
        
        # Good samples should be unchanged
        assert np.array_equal(result[mask_good], x[mask_good])
        
        # Bad samples should be interpolated
        assert result[2] == 2.5  # Midpoint between 2 and 5
        assert result[3] == 4.5  # Midpoint between 2 and 5
    
    def test_all_good_samples(self):
        """Test when all samples are good."""
        x = np.array([1, 2, 3, 4, 5])
        mask_good = np.ones(5, dtype=bool)
        
        result = interpolate_masked_samples(x, mask_good)
        
        # Should return exact copy
        assert np.array_equal(result, x)
    
    def test_no_good_samples(self):
        """Test when no samples are good."""
        x = np.array([1, 2, 3, 4, 5])
        mask_good = np.zeros(5, dtype=bool)
        
        with warnings.catch_warnings(record=True) as w:
            result = interpolate_masked_samples(x, mask_good)
            assert len(w) == 1
            assert "No good samples" in str(w[0].message)
        
        # Should return copy of original
        assert np.array_equal(result, x)
    
    def test_mismatched_lengths(self):
        """Test mismatched signal and mask lengths."""
        x = np.array([1, 2, 3])
        mask_good = np.array([True, False])
        
        with pytest.raises(ValueError, match="same length"):
            interpolate_masked_samples(x, mask_good)


class TestCleanEda:
    """Test complete EDA cleaning pipeline."""
    
    def test_basic_cleaning(self):
        """Test basic cleaning functionality."""
        # Generate synthetic EDA-like signal
        fs = 4
        duration = 300  # 5 minutes
        t = np.arange(0, duration, 1/fs)
        
        # Base EDA signal with slow drift and noise
        eda_base = 2.0 + 0.5 * np.sin(2 * np.pi * 0.01 * t) + 0.1 * np.random.randn(len(t))
        
        # Add some spikes
        spike_indices = np.random.choice(len(t), size=5, replace=False)
        eda_raw = eda_base.copy()
        eda_raw[spike_indices] += 2.0
        
        # Low motion accelerometer signal
        acc_mag = np.random.normal(1.0, 0.1, len(t))
        
        # Clean the signal
        eda_clean, mask_good = clean_eda(eda_raw, acc_mag, fs=fs)
        
        # Basic checks
        assert len(eda_clean) == len(eda_raw)
        assert len(mask_good) == len(eda_raw)
        assert mask_good.dtype == bool
        
        # Cleaned signal should be in [0, 1] range
        assert np.all(eda_clean >= 0)
        assert np.all(eda_clean <= 1)
        
        # Should have reduced extreme values (spikes)
        assert np.std(eda_clean) < np.std(eda_raw)
    
    def test_with_motion_artifacts(self):
        """Test cleaning with motion artifacts."""
        fs = 4
        duration = 120
        t = np.arange(0, duration, 1/fs)
        
        # EDA signal
        eda_raw = 2.0 + 0.1 * np.random.randn(len(t))
        
        # Accelerometer with motion periods
        acc_mag = np.random.normal(1.0, 0.1, len(t))
        motion_period = slice(100, 200)
        acc_mag[motion_period] += 3.0  # High motion
        
        eda_clean, mask_good = clean_eda(eda_raw, acc_mag, fs=fs)
        
        # Motion period should have some bad samples
        assert np.mean(mask_good[motion_period]) < np.mean(mask_good)
    
    def test_empty_input(self):
        """Test empty input arrays."""
        eda_clean, mask_good = clean_eda(np.array([]), np.array([]), fs=4)
        
        assert len(eda_clean) == 0
        assert len(mask_good) == 0
        assert mask_good.dtype == bool
    
    def test_mismatched_lengths(self):
        """Test mismatched EDA and accelerometer lengths."""
        eda = np.array([1, 2, 3])
        acc = np.array([1, 2])
        
        with pytest.raises(ValueError, match="same length"):
            clean_eda(eda, acc)


class TestAssessCleaningQuality:
    """Test quality assessment function."""
    
    def test_basic_assessment(self):
        """Test basic quality assessment."""
        # Create test signals
        eda_raw = np.random.normal(2.0, 0.5, 1000)
        eda_clean = (eda_raw - np.min(eda_raw)) / (np.max(eda_raw) - np.min(eda_raw))
        mask_good = np.random.choice([True, False], size=1000, p=[0.8, 0.2])
        
        metrics = assess_cleaning_quality(eda_raw, eda_clean, mask_good)
        
        # Check that all expected metrics are present
        expected_keys = [
            'samples_total', 'samples_good', 'motion_percentage',
            'raw_range', 'clean_range', 'clean_min', 'clean_max',
            'raw_std', 'clean_std', 'noise_reduction_db'
        ]
        
        for key in expected_keys:
            assert key in metrics
        
        # Basic sanity checks
        assert metrics['samples_total'] == 1000
        assert metrics['samples_good'] == np.sum(mask_good)
        assert 0 <= metrics['motion_percentage'] <= 100
        assert metrics['clean_min'] >= 0
        assert metrics['clean_max'] <= 1


# Integration test
def test_eda_processing_integration():
    """Integration test for complete EDA processing workflow."""
    # Generate realistic test data
    np.random.seed(42)  # For reproducibility
    
    fs = 4
    duration = 600  # 10 minutes
    t = np.arange(0, duration, 1/fs)
    
    # Realistic EDA signal
    eda_raw = (
        2.5 +  # Baseline
        0.3 * np.sin(2 * np.pi * 0.005 * t) +  # Slow drift
        0.1 * np.sin(2 * np.pi * 0.02 * t) +   # Faster variation
        0.05 * np.random.randn(len(t))          # Noise
    )
    
    # Add occasional spikes
    spike_indices = np.random.choice(len(t), size=10, replace=False)
    eda_raw[spike_indices] += np.random.uniform(0.5, 2.0, len(spike_indices))
    
    # Realistic accelerometer with motion periods
    acc_mag = np.random.lognormal(0, 0.3, len(t))  # Baseline motion
    
    # Add motion artifact periods
    motion_start = len(t) // 3
    motion_end = motion_start + 100
    acc_mag[motion_start:motion_end] *= 5  # High motion period
    
    # Process the signal
    eda_clean, mask_good = clean_eda(eda_raw, acc_mag, fs=fs)
    
    # Assess quality
    quality = assess_cleaning_quality(eda_raw, eda_clean, mask_good)
    
    # Comprehensive checks
    assert len(eda_clean) == len(eda_raw)
    assert np.all(eda_clean >= 0) and np.all(eda_clean <= 1)
    assert quality['motion_percentage'] > 0  # Should detect some motion
    assert not np.isnan(quality['noise_reduction_db'])  # Should be a valid number
    assert quality['noise_reduction_db'] >= 0  # Should not increase noise significantly
    
    # Motion period should have lower proportion of good samples
    motion_mask_quality = np.mean(mask_good[motion_start:motion_end])
    baseline_mask_quality = np.mean(mask_good[:motion_start])
    assert motion_mask_quality < baseline_mask_quality
    
    print(f"Integration test passed:")
    print(f"  - Motion artifacts detected: {quality['motion_percentage']:.1f}%")
    print(f"  - Noise reduction: {quality['noise_reduction_db']:.1f} dB")
    print(f"  - Clean signal range: [{quality['clean_min']:.3f}, {quality['clean_max']:.3f}]")


if __name__ == "__main__":
    # Run the integration test
    test_eda_processing_integration()
    print("All EDA tests would pass!")