"""
Unit tests for temperature drift calculation.

Tests the robustness of compute_temp_drift against outliers as specified
in the acceptance criteria.
"""

import numpy as np
import sys
import os
import importlib.util

# Import temp module directly to avoid signal module conflicts
repo_path = "/home/runner/work/tVNS-Modeling-Playground/tVNS-Modeling-Playground"
spec = importlib.util.spec_from_file_location("temp", os.path.join(repo_path, "signal", "temp.py"))
temp_module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(temp_module)
compute_temp_drift = temp_module.compute_temp_drift


class TestComputeTempDrift:
    """Test suite for compute_temp_drift function."""
    
    def test_basic_functionality(self):
        """Test basic drift calculation on clean signal."""
        # Create a temperature signal with known drift (0.1°C/hour)
        fs = 4  # 4 Hz sampling
        duration_hours = 2
        n_samples = int(fs * duration_hours * 3600)
        time_hours = np.linspace(0, duration_hours, n_samples)
        
        # Linear trend: 0.1°C/hour + noise
        np.random.seed(42)
        temp_signal = 37.0 + 0.1 * time_hours + 0.01 * np.random.randn(n_samples)
        
        drift = compute_temp_drift(temp_signal, fs=fs)
        
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
        
        # Compute baseline drift
        baseline_drift = compute_temp_drift(base_temp, fs=fs)
        
        # Add 3σ outlier at middle of signal
        temp_std = np.std(base_temp)
        outlier_temp = base_temp.copy()
        outlier_idx = len(outlier_temp) // 2
        outlier_temp[outlier_idx] += 3 * temp_std  # 3σ spike
        
        # Compute drift with outlier
        outlier_drift = compute_temp_drift(outlier_temp, fs=fs)
        
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
        duration_hours = 2
        n_samples = int(fs * duration_hours * 3600)
        time_hours = np.linspace(0, duration_hours, n_samples)
        
        # Create baseline signal
        np.random.seed(42)
        base_temp = 37.0 + 0.02 * time_hours + 0.005 * np.random.randn(n_samples)
        
        baseline_drift = compute_temp_drift(base_temp, fs=fs)
        
        # Add multiple 3σ outliers (but isolated)
        temp_std = np.std(base_temp)
        outlier_temp = base_temp.copy()
        outlier_indices = [n_samples // 4, n_samples // 2, 3 * n_samples // 4]
        
        for idx in outlier_indices:
            outlier_temp[idx] += 3 * temp_std
        
        outlier_drift = compute_temp_drift(outlier_temp, fs=fs)
        
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
        """Test that function validates minimum signal length."""
        short_signal = np.array([37.0, 37.1, 37.2])  # Only 3 samples
        
        try:
            compute_temp_drift(short_signal)
            assert False, "Should have raised ValueError"
        except ValueError as e:
            assert "at least 21 samples" in str(e)
    
    def test_zero_drift_signal(self):
        """Test with signal having no drift."""
        fs = 4
        n_samples = 1000
        
        # Constant temperature with noise
        np.random.seed(42)
        temp_signal = 37.0 + 0.01 * np.random.randn(n_samples)
        
        drift = compute_temp_drift(temp_signal, fs=fs)
        
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
        
        drift = compute_temp_drift(temp_signal, fs=fs)
        
        # Should detect negative drift (within reasonable tolerance)
        assert drift < -0.15, f"Expected negative drift around -0.2°C/hour, got {drift:.4f}"
        return drift


if __name__ == "__main__":
    # Run basic test to verify implementation
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
    
    print("\nRunning zero drift test...")
    drift_zero = test_suite.test_zero_drift_signal()
    print(f"✓ Zero drift test passed. Drift: {drift_zero:.6f} °C/hour")
    
    print("\nRunning negative drift test...")
    drift_neg = test_suite.test_negative_drift()
    print(f"✓ Negative drift test passed. Drift: {drift_neg:.4f} °C/hour")
    
    print("\nAll tests passed! 🎉")