"""
Unit tests for temperature drift calculation.

Tests the robustness of the improved compute_temp_drift function against 
outliers and various signal conditions.
"""

import numpy as np
import sys
import os

# Add the parent signal directory to Python path
current_dir = os.path.dirname(os.path.abspath(__file__))
signal_dir = os.path.dirname(current_dir)  # Go up one level to signal/
sys.path.insert(0, signal_dir)

# Now import the modules normally
from temp import compute_temp_drift, estimate_drift_fast


class TestComputeTempDrift:
    """Test suite for compute_temp_drift function."""
    
    def test_basic_functionality(self):
        """Test basic drift calculation on clean signal."""
        # Create a temperature signal with known drift (0.1°C/hour)
        fs = 4  # 4 Hz sampling
        duration_hours = 1  # Shorter for faster tests
        n_samples = int(fs * duration_hours * 3600)
        time_hours = np.linspace(0, duration_hours, n_samples)
        
        # Linear trend: 0.1°C/hour + noise
        np.random.seed(42)
        temp_signal = 37.0 + 0.1 * time_hours + 0.01 * np.random.randn(n_samples)
        
        # Use 60-second smoothing for better accuracy on longer signals
        drift = compute_temp_drift(temp_signal, fs=fs, smooth_seconds=60.0)
        
        # Should be close to 0.1°C/hour (within reasonable tolerance due to noise and smoothing)
        assert abs(drift - 0.1) < 0.02, f"Expected ~0.1°C/hour, got {drift:.4f}"
        return drift
    
    def test_outlier_robustness(self):
        """Test robustness against 3σ outliers."""
        fs = 4
        duration_hours = 1
        n_samples = int(fs * duration_hours * 3600)
        time_hours = np.linspace(0, duration_hours, n_samples)
        
        # Create baseline signal with 0.05°C/hour drift
        np.random.seed(42)
        base_temp = 37.0 + 0.05 * time_hours + 0.01 * np.random.randn(n_samples)
        
        # Compute baseline drift with appropriate smoothing
        baseline_drift = compute_temp_drift(base_temp, fs=fs, smooth_seconds=30.0)
        
        # Add 3σ outlier at middle of signal
        temp_std = np.std(base_temp)
        outlier_temp = base_temp.copy()
        outlier_idx = len(outlier_temp) // 2
        outlier_temp[outlier_idx] += 3 * temp_std  # 3σ spike
        
        # Compute drift with outlier
        outlier_drift = compute_temp_drift(outlier_temp, fs=fs, smooth_seconds=30.0)
        
        # Change should be ≤10% as per acceptance criteria
        if abs(baseline_drift) > 1e-6:  # Avoid division by very small numbers
            percent_change = abs(outlier_drift - baseline_drift) / abs(baseline_drift) * 100
        else:
            percent_change = abs(outlier_drift - baseline_drift) * 100  # Absolute change for near-zero baselines
        
        assert percent_change <= 10, (
            f"Drift changed by {percent_change:.1f}% with outlier, "
            f"should be ≤10%. Baseline: {baseline_drift:.4f}, "
            f"Outlier: {outlier_drift:.4f}"
        )
        return baseline_drift, outlier_drift, percent_change
    
    def test_multiple_outliers_robustness(self):
        """Test robustness against multiple outliers."""
        fs = 4
        duration_hours = 1  # Shorter for faster tests
        n_samples = int(fs * duration_hours * 3600)
        time_hours = np.linspace(0, duration_hours, n_samples)
        
        # Create baseline signal
        np.random.seed(42)
        base_temp = 37.0 + 0.02 * time_hours + 0.005 * np.random.randn(n_samples)
        
        baseline_drift = compute_temp_drift(base_temp, fs=fs, smooth_seconds=60.0)
        
        # Add multiple 3σ outliers (but isolated)
        temp_std = np.std(base_temp)
        outlier_temp = base_temp.copy()
        outlier_indices = [n_samples // 4, n_samples // 2, 3 * n_samples // 4]
        
        for idx in outlier_indices:
            outlier_temp[idx] += 3 * temp_std
        
        outlier_drift = compute_temp_drift(outlier_temp, fs=fs, smooth_seconds=60.0)
        
        # Even with multiple outliers, should remain robust
        if abs(baseline_drift) > 1e-6:
            percent_change = abs(outlier_drift - baseline_drift) / abs(baseline_drift) * 100
        else:
            percent_change = abs(outlier_drift - baseline_drift) * 100
        
        assert percent_change <= 15, (  # Slightly more lenient for multiple outliers
            f"Drift changed by {percent_change:.1f}% with multiple outliers"
        )
        return baseline_drift, outlier_drift, percent_change
    
    def test_minimum_length_validation(self):
        """Test that function handles short signals appropriately."""
        short_signal = np.array([37.0, 37.1, 37.2])  # Only 3 samples
        
        # The new implementation should handle short signals more gracefully
        # It will use the minimum valid window size
        try:
            drift = compute_temp_drift(short_signal, fs=4, smooth_seconds=1.0)
            # Should complete without error, though result may not be meaningful
            print(f"Short signal drift: {drift:.6f} °C/hour")
        except ValueError as e:
            # If it does raise an error, it should be informative
            assert "Signal too short" in str(e) or "Need at least" in str(e)
    
    def test_nan_handling(self):
        """Test NaN handling options."""
        fs = 4
        n_samples = 1000
        
        # Create signal with some NaN values
        np.random.seed(42)
        temp_signal = 37.0 + 0.05 * np.arange(n_samples) / (fs * 3600) + 0.01 * np.random.randn(n_samples)
        temp_signal[100:105] = np.nan  # Add some NaNs
        
        # Test 'drop' method
        drift_drop = compute_temp_drift(temp_signal, fs=fs, handle_nans='drop')
        
        # Test 'interpolate' method  
        drift_interp = compute_temp_drift(temp_signal, fs=fs, handle_nans='interpolate')
        
        # Both should produce reasonable results
        assert abs(drift_drop - 0.05) < 0.02, f"Drop method drift: {drift_drop:.4f}"
        assert abs(drift_interp - 0.05) < 0.02, f"Interpolate method drift: {drift_interp:.4f}"
        
        return drift_drop, drift_interp
    
    def test_confidence_intervals(self):
        """Test confidence interval functionality."""
        fs = 4
        n_samples = 2000
        
        # Create signal with known drift
        np.random.seed(42)
        temp_signal = 37.0 + 0.08 * np.arange(n_samples) / (fs * 3600) + 0.01 * np.random.randn(n_samples)
        
        drift, (lower_ci, upper_ci) = compute_temp_drift(
            temp_signal, fs=fs, return_confidence=True
        )
        
        # Check that confidence interval contains the estimate
        assert lower_ci <= drift <= upper_ci, f"CI [{lower_ci:.4f}, {upper_ci:.4f}] doesn't contain drift {drift:.4f}"
        
        # Check that CI has reasonable width
        ci_width = upper_ci - lower_ci
        assert 0 < ci_width < 0.1, f"CI width {ci_width:.4f} seems unreasonable"
        
        return drift, lower_ci, upper_ci
    
    def test_zero_drift_signal(self):
        """Test with signal having no drift."""
        fs = 4
        n_samples = 1000
        
        # Constant temperature with noise
        np.random.seed(42)
        temp_signal = 37.0 + 0.01 * np.random.randn(n_samples)
        
        drift = compute_temp_drift(temp_signal, fs=fs, smooth_seconds=30.0)
        
        # Should be close to zero (within reasonable tolerance)
        assert abs(drift) < 0.05, f"Expected near-zero drift, got {drift:.4f}"
        return drift
    
    def test_negative_drift(self):
        """Test with cooling trend."""
        fs = 4
        duration_hours = 1
        n_samples = int(fs * duration_hours * 3600)
        time_hours = np.linspace(0, duration_hours, n_samples)
        
        # Cooling trend: -0.2°C/hour
        np.random.seed(42)
        temp_signal = 37.0 - 0.2 * time_hours + 0.01 * np.random.randn(n_samples)
        
        drift = compute_temp_drift(temp_signal, fs=fs, smooth_seconds=60.0)
        
        # Should detect negative drift (within reasonable tolerance)
        assert drift < -0.15, f"Expected negative drift around -0.2°C/hour, got {drift:.4f}"
        return drift
    
    def test_fast_estimation(self):
        """Test fast estimation method."""
        fs = 4
        duration_hours = 0.25  # Even shorter signal for speed
        n_samples = int(fs * duration_hours * 3600)
        time_hours = np.linspace(0, duration_hours, n_samples)
        
        # Create signal with known drift
        np.random.seed(42)
        temp_signal = 37.0 + 0.1 * time_hours + 0.01 * np.random.randn(n_samples)
        
        # Test both methods
        drift_robust = compute_temp_drift(temp_signal, fs=fs, use_theilsen=True)
        drift_fast = estimate_drift_fast(temp_signal, fs=fs, method='quantile')
        
        # Should be reasonably close - more lenient tolerance for fast method
        assert abs(drift_robust - drift_fast) < 0.1, (
            f"Robust and fast methods differ too much: {drift_robust:.4f} vs {drift_fast:.4f}"
        )
        
        return drift_robust, drift_fast
    
    def test_irregular_sampling(self):
        """Test with irregular time sampling."""
        # Create irregular time vector
        n_samples = 500
        t_irregular = np.sort(np.random.uniform(0, 3600, n_samples))  # Random times over 1 hour
        
        # Create signal with known drift at irregular times
        np.random.seed(42)
        true_drift = 0.05  # °C/hour
        temp_signal = 37.0 + true_drift * (t_irregular / 3600) + 0.01 * np.random.randn(n_samples)
        
        drift = compute_temp_drift(temp_signal, t=t_irregular, smooth_seconds=60.0)
        
        # Should estimate drift reasonably well despite irregular sampling
        assert abs(drift - true_drift) < 0.03, f"Expected ~{true_drift}°C/hour, got {drift:.4f}"
        return drift


if __name__ == "__main__":
    # Run comprehensive test suite to verify implementation
    test_suite = TestComputeTempDrift()
    
    print("Running basic functionality test...")
    drift = test_suite.test_basic_functionality()
    print(f"✓ Basic functionality test passed. Drift: {drift:.4f} °C/hour")
    
    print("\nRunning outlier robustness test...")
    baseline, outlier, change = test_suite.test_outlier_robustness()
    print(f"✓ Outlier robustness test passed")
    print(f"  Baseline drift: {baseline:.4f} °C/hour")
    print(f"  Outlier drift: {outlier:.4f} °C/hour")
    print(f"  Change: {change:.1f}% (≤10% required)")
    
    print("\nRunning multiple outliers test...")
    baseline, outlier, change = test_suite.test_multiple_outliers_robustness()
    print(f"✓ Multiple outliers test passed")
    print(f"  Baseline drift: {baseline:.4f} °C/hour")
    print(f"  Outlier drift: {outlier:.4f} °C/hour")
    print(f"  Change: {change:.1f}% (≤15% acceptable for multiple outliers)")
    
    print("\nRunning minimum length validation test...")
    test_suite.test_minimum_length_validation()
    print("✓ Minimum length validation test passed")
    
    print("\nRunning NaN handling test...")
    drift_drop, drift_interp = test_suite.test_nan_handling()
    print(f"✓ NaN handling test passed")
    print(f"  Drop method: {drift_drop:.4f} °C/hour")
    print(f"  Interpolate method: {drift_interp:.4f} °C/hour")
    
    print("\nRunning confidence interval test...")
    drift_ci, lower, upper = test_suite.test_confidence_intervals()
    print(f"✓ Confidence interval test passed")
    print(f"  Drift: {drift_ci:.4f} °C/hour")
    print(f"  95% CI: [{lower:.4f}, {upper:.4f}] °C/hour")
    
    print("\nRunning zero drift test...")
    drift_zero = test_suite.test_zero_drift_signal()
    print(f"✓ Zero drift test passed. Drift: {drift_zero:.6f} °C/hour")
    
    print("\nRunning negative drift test...")
    drift_neg = test_suite.test_negative_drift()
    print(f"✓ Negative drift test passed. Drift: {drift_neg:.4f} °C/hour")
    
    print("\nRunning fast estimation test...")
    drift_robust, drift_fast = test_suite.test_fast_estimation()
    print(f"✓ Fast estimation test passed")
    print(f"  Robust method: {drift_robust:.4f} °C/hour")
    print(f"  Fast method: {drift_fast:.4f} °C/hour")
    
    print("\nRunning irregular sampling test...")
    drift_irreg = test_suite.test_irregular_sampling()
    print(f"✓ Irregular sampling test passed. Drift: {drift_irreg:.4f} °C/hour")
    
    print("\nAll tests passed! 🎉")
    print("\nThe improved compute_temp_drift function demonstrates:")
    print("- Robust outlier handling with Theil-Sen estimation")
    print("- Flexible time-based smoothing windows")
    print("- Multiple NaN handling strategies")
    print("- Confidence interval support")
    print("- Fast estimation for long signals")
    print("- Support for irregular sampling")
