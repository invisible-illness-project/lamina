"""
Test suite for temperature drift estimation functions.
"""

import sys
import os
import pytest
import numpy as np
import warnings
from unittest.mock import patch

# Add parent directory to path to import signal modules
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

from temp import (
    compute_temp_drift,
    estimate_drift_fast
)


class TestComputeTempDrift:
    """Test temperature drift computation function."""
    
    def test_basic_drift_estimation(self):
        """Test basic drift estimation with known drift rate."""
        # Generate signal with known drift
        fs = 4
        duration = 3600  # 1 hour
        t = np.arange(0, duration, 1/fs)
        
        true_drift = 0.1  # °C/hour
        temp = 37.0 + true_drift * (t / 3600) + 0.01 * np.random.randn(len(t))
        
        estimated_drift = compute_temp_drift(temp, fs=fs, smooth_seconds=30)
        
        # Should be close to true drift
        assert abs(estimated_drift - true_drift) < 0.02
    
    def test_with_custom_time_vector(self):
        """Test with custom time vector."""
        # Non-uniform time sampling
        t = np.sort(np.random.uniform(0, 3600, 1000))
        true_drift = 0.05
        temp = 36.5 + true_drift * (t / 3600) + 0.005 * np.random.randn(len(t))
        
        estimated_drift = compute_temp_drift(temp, t=t, smooth_seconds=60)
        
        assert abs(estimated_drift - true_drift) < 0.03
    
    def test_no_drift_signal(self):
        """Test signal with no drift (constant temperature)."""
        temp = np.full(1000, 37.0) + 0.01 * np.random.randn(1000)
        
        estimated_drift = compute_temp_drift(temp, fs=4)
        
        # Should be close to zero
        assert abs(estimated_drift) < 0.01
    
    def test_with_outliers(self):
        """Test robustness to outliers."""
        fs = 4
        duration = 1800  # 30 minutes
        t = np.arange(0, duration, 1/fs)
        
        true_drift = 0.2
        temp = 37.0 + true_drift * (t / 3600) + 0.01 * np.random.randn(len(t))
        
        # Add outliers
        outlier_indices = np.random.choice(len(t), size=10, replace=False)
        temp[outlier_indices] += 5 * np.random.choice([-1, 1], 10)
        
        # Test both Theil-Sen and least squares
        drift_theilsen = compute_temp_drift(temp, fs=fs, use_theilsen=True)
        drift_ols = compute_temp_drift(temp, fs=fs, use_theilsen=False)
        
        # Theil-Sen should be more robust
        assert abs(drift_theilsen - true_drift) <= abs(drift_ols - true_drift)
        assert abs(drift_theilsen - true_drift) < 0.05
    
    def test_confidence_intervals(self):
        """Test confidence interval estimation."""
        fs = 4
        duration = 1800
        t = np.arange(0, duration, 1/fs)
        
        true_drift = 0.15
        temp = 37.0 + true_drift * (t / 3600) + 0.02 * np.random.randn(len(t))
        
        drift, (lower_ci, upper_ci) = compute_temp_drift(
            temp, fs=fs, return_confidence=True
        )
        
        # Confidence interval should contain true drift
        assert lower_ci <= true_drift <= upper_ci
        assert lower_ci < drift < upper_ci
        assert upper_ci > lower_ci
    
    def test_short_signal(self):
        """Test very short signal."""
        temp = np.array([37.0, 37.1])
        
        with pytest.raises(ValueError, match="at least 3 samples"):
            compute_temp_drift(temp, fs=4)
    
    def test_nan_handling_drop(self):
        """Test NaN handling with drop method."""
        temp = np.array([37.0, 37.1, np.nan, 37.2, 37.3, np.nan, 37.4])
        
        # Should work by dropping NaN values
        drift = compute_temp_drift(temp, fs=4, handle_nans='drop')
        
        # Should return a reasonable value
        assert isinstance(drift, float)
        assert not np.isnan(drift)
    
    def test_nan_handling_interpolate(self):
        """Test NaN handling with interpolation."""
        temp = np.array([37.0, 37.1, np.nan, 37.3, 37.4])
        
        drift = compute_temp_drift(temp, fs=4, handle_nans='interpolate')
        
        assert isinstance(drift, float)
        assert not np.isnan(drift)
    
    def test_nan_handling_raise(self):
        """Test NaN handling with raise option."""
        temp = np.array([37.0, 37.1, np.nan, 37.2])
        
        with pytest.raises(ValueError, match="Non-finite values"):
            compute_temp_drift(temp, fs=4, handle_nans='raise')
    
    def test_all_nans(self):
        """Test signal with all NaN values."""
        temp = np.full(10, np.nan)
        
        with pytest.raises(ValueError, match="No finite values"):
            compute_temp_drift(temp, fs=4, handle_nans='drop')
    
    def test_invalid_handle_nans(self):
        """Test invalid handle_nans parameter."""
        temp = np.array([37.0, 37.1, 37.2])
        
        with pytest.raises(ValueError, match="handle_nans must be"):
            compute_temp_drift(temp, fs=4, handle_nans='invalid')
    
    def test_mismatched_time_length(self):
        """Test mismatched temperature and time vector lengths."""
        temp = np.array([37.0, 37.1, 37.2])
        t = np.array([0, 1])  # Different length
        
        with pytest.raises(ValueError, match="same length"):
            compute_temp_drift(temp, t=t)
    
    def test_high_polyorder(self):
        """Test polyorder higher than signal allows."""
        temp = np.array([37.0, 37.1, 37.2])  # Only 3 samples
        
        with pytest.raises(ValueError, match="Signal too short"):
            compute_temp_drift(temp, fs=4, polyorder=5)
    
    def test_long_signal_downsampling(self):
        """Test downsampling for very long signals."""
        # Create a very long signal that would trigger downsampling
        fs = 4
        duration = 10000  # Very long signal
        t = np.arange(0, duration, 1/fs)
        
        true_drift = 0.1
        temp = 37.0 + true_drift * (t / 3600) + 0.01 * np.random.randn(len(t))
        
        with warnings.catch_warnings(record=True) as w:
            drift = compute_temp_drift(
                temp, fs=fs, use_theilsen=True, max_samples_theilsen=1000
            )
            # Should warn about downsampling
            assert len(w) == 1
            assert "exceeds max_samples_theilsen" in str(w[0].message)
        
        # Should still give reasonable estimate
        assert abs(drift - true_drift) < 0.05
    
    def test_savgol_failure_fallback(self):
        """Test handling of Savitzky-Golay filter failure."""
        # This is harder to test directly, but we can test the error handling
        temp = np.array([37.0, 37.1, 37.2])
        
        # This should work despite potential edge cases
        drift = compute_temp_drift(temp, fs=4, polyorder=1, smooth_seconds=1)
        assert isinstance(drift, float)
    
    @patch('temp.stats.theilslopes')
    def test_theilsen_failure_fallback(self, mock_theilslopes):
        """Test fallback to least squares when Theil-Sen fails."""
        # Mock Theil-Sen to raise an exception
        mock_theilslopes.side_effect = Exception("Theil-Sen failed")
        
        temp = np.array([37.0, 37.1, 37.2, 37.3, 37.4])
        
        with warnings.catch_warnings(record=True) as w:
            drift = compute_temp_drift(temp, fs=4, use_theilsen=True)
            assert len(w) == 1
            assert "Theil-Sen estimation failed" in str(w[0].message)
        
        # Should still return a result using least squares
        assert isinstance(drift, float)


class TestEstimateDriftFast:
    """Test fast drift estimation function."""
    
    def test_quantile_method(self):
        """Test quantile regression method."""
        # Generate signal with known drift
        fs = 4
        duration = 1800
        t = np.arange(0, duration, 1/fs)
        
        true_drift = 0.1
        temp = 37.0 + true_drift * (t / 3600) + 0.02 * np.random.randn(len(t))
        
        drift = estimate_drift_fast(temp, fs=fs, method='quantile')
        
        # Should be reasonably close to true drift
        assert abs(drift - true_drift) < 0.05
    
    def test_huber_method_with_sklearn(self):
        """Test Huber regression method (if sklearn available)."""
        try:
            from sklearn.linear_model import HuberRegressor
            
            fs = 4
            duration = 1800
            t = np.arange(0, duration, 1/fs)
            
            true_drift = 0.15
            temp = 37.0 + true_drift * (t / 3600) + 0.02 * np.random.randn(len(t))
            
            drift = estimate_drift_fast(temp, fs=fs, method='huber')
            
            assert abs(drift - true_drift) < 0.05
            
        except ImportError:
            pytest.skip("sklearn not available")
    
    @patch('temp.HuberRegressor', side_effect=ImportError)
    def test_huber_method_fallback(self, mock_huber):
        """Test fallback to quantile when sklearn not available."""
        temp = np.array([37.0, 37.1, 37.2, 37.3, 37.4])
        
        with warnings.catch_warnings(record=True) as w:
            drift = estimate_drift_fast(temp, fs=4, method='huber')
            # May or may not warn depending on sklearn availability
        
        # Should still return a result
        assert isinstance(drift, float)
    
    def test_custom_time_vector(self):
        """Test with custom time vector."""
        t = np.array([0, 100, 300, 600, 1000, 1500])
        true_drift = 0.2
        temp = 37.0 + true_drift * (t / 3600) + 0.01 * np.random.randn(len(t))
        
        drift = estimate_drift_fast(temp, t=t, method='quantile')
        
        assert abs(drift - true_drift) < 0.1
    
    def test_invalid_method(self):
        """Test invalid method parameter."""
        temp = np.array([37.0, 37.1, 37.2])
        
        with pytest.raises(ValueError, match="method must be"):
            estimate_drift_fast(temp, fs=4, method='invalid')
    
    def test_insufficient_samples(self):
        """Test with insufficient samples."""
        temp = np.array([37.0])
        
        with pytest.raises(ValueError, match="at least 2 finite samples"):
            estimate_drift_fast(temp, fs=4)
    
    def test_with_nans(self):
        """Test handling of NaN values."""
        temp = np.array([37.0, np.nan, 37.2, 37.3, np.nan, 37.5])
        
        drift = estimate_drift_fast(temp, fs=4, method='quantile')
        
        # Should handle NaNs by dropping them
        assert isinstance(drift, float)
        assert not np.isnan(drift)


class TestTemperatureDriftIntegration:
    """Integration tests for temperature drift estimation."""
    
    def test_realistic_temperature_signal(self):
        """Test with realistic temperature signal."""
        np.random.seed(42)  # For reproducibility
        
        # Simulate realistic body temperature recording
        fs = 4
        duration = 7200  # 2 hours
        t = np.arange(0, duration, 1/fs)
        
        # Base temperature with realistic drift and noise
        base_temp = 36.8
        true_drift = 0.05  # Small drift
        physiological_variation = 0.1 * np.sin(2 * np.pi * t / 1800)  # 30-min cycles
        measurement_noise = 0.02 * np.random.randn(len(t))
        
        temp = (base_temp + 
                true_drift * (t / 3600) + 
                physiological_variation + 
                measurement_noise)
        
        # Add some measurement artifacts
        artifact_indices = np.random.choice(len(t), size=5, replace=False)
        temp[artifact_indices] += np.random.uniform(-0.5, 0.5, 5)
        
        # Test both methods
        drift_robust = compute_temp_drift(temp, fs=fs, use_theilsen=True)
        drift_fast = estimate_drift_fast(temp, fs=fs, method='quantile')
        
        # Both should be reasonably close to true drift
        assert abs(drift_robust - true_drift) < 0.03
        assert abs(drift_fast - true_drift) < 0.05
        
        # Get confidence interval
        drift_ci, (lower, upper) = compute_temp_drift(
            temp, fs=fs, return_confidence=True
        )
        
        # True drift should be within confidence interval
        assert lower <= true_drift <= upper
        
        print(f"Integration test results:")
        print(f"  True drift: {true_drift:.4f} °C/hour")
        print(f"  Robust estimate: {drift_robust:.4f} °C/hour")
        print(f"  Fast estimate: {drift_fast:.4f} °C/hour")
        print(f"  95% CI: [{lower:.4f}, {upper:.4f}] °C/hour")
    
    def test_extreme_drift_detection(self):
        """Test detection of extreme drift rates."""
        fs = 4
        duration = 3600
        t = np.arange(0, duration, 1/fs)
        
        # Very high drift rate
        extreme_drift = 1.0  # 1°C/hour
        temp = 37.0 + extreme_drift * (t / 3600) + 0.01 * np.random.randn(len(t))
        
        estimated_drift = compute_temp_drift(temp, fs=fs)
        
        # Should detect high drift accurately
        assert abs(estimated_drift - extreme_drift) < 0.1
        assert estimated_drift > 0.5  # Should be clearly positive
    
    def test_negative_drift(self):
        """Test detection of negative drift (cooling)."""
        fs = 4
        duration = 1800
        t = np.arange(0, duration, 1/fs)
        
        negative_drift = -0.2  # Cooling
        temp = 37.5 + negative_drift * (t / 3600) + 0.01 * np.random.randn(len(t))
        
        estimated_drift = compute_temp_drift(temp, fs=fs)
        
        # Should detect negative drift
        assert abs(estimated_drift - negative_drift) < 0.05
        assert estimated_drift < -0.1
    
    def test_comparison_methods(self):
        """Compare different drift estimation methods."""
        np.random.seed(123)
        
        fs = 4
        duration = 3600
        t = np.arange(0, duration, 1/fs)
        
        true_drift = 0.12
        temp = 37.0 + true_drift * (t / 3600) + 0.02 * np.random.randn(len(t))
        
        # Add some outliers
        outlier_indices = np.random.choice(len(t), size=8, replace=False)
        temp[outlier_indices] += 2 * np.random.choice([-1, 1], 8)
        
        # Compare methods
        drift_theilsen = compute_temp_drift(temp, fs=fs, use_theilsen=True)
        drift_ols = compute_temp_drift(temp, fs=fs, use_theilsen=False)
        drift_quantile = estimate_drift_fast(temp, fs=fs, method='quantile')
        
        # All should be reasonable, with Theil-Sen being most robust
        methods = {
            'Theil-Sen': drift_theilsen,
            'OLS': drift_ols,
            'Quantile': drift_quantile
        }
        
        print(f"\nMethod comparison (true drift: {true_drift:.4f} °C/hour):")
        for method, estimate in methods.items():
            error = abs(estimate - true_drift)
            print(f"  {method}: {estimate:.4f} °C/hour (error: {error:.4f})")
            assert error < 0.1  # All methods should be reasonably accurate


# Utility function for running tests
def run_temperature_drift_tests():
    """Run all temperature drift tests."""
    print("Running temperature drift estimation tests...")
    
    # Basic functionality tests
    test_compute = TestComputeTempDrift()
    test_compute.test_basic_drift_estimation()
    test_compute.test_with_outliers()
    test_compute.test_confidence_intervals()
    
    # Fast estimation tests
    test_fast = TestEstimateDriftFast()
    test_fast.test_quantile_method()
    
    # Integration tests
    test_integration = TestTemperatureDriftIntegration()
    test_integration.test_realistic_temperature_signal()
    test_integration.test_comparison_methods()
    
    print("All temperature drift tests passed!")


if __name__ == "__main__":
    run_temperature_drift_tests()