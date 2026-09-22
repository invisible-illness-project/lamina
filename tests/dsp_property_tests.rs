use lamina::signal::dc::signal_remove_dc;
use lamina::signal::filter::{FilterSpec, signal_filtfilt};
use lamina::signal::normalize::{DegeneratePolicy, signal_minmax};
use lamina::signal::resample::signal_resample_poly;
use lamina::signal::segment::{IncompleteTailPolicy, signal_segment};
use ndarray::Array1;
use std::f64::consts::PI;

#[test]
fn test_dsp_analytical_frequency_response() {
    let fs = 125.0;
    let n = (fs * 4.0) as usize; // 4 seconds of data
    let test_freqs = vec![0.5, 1.0, 4.0, 8.0, 10.0, 12.0, 16.0, 20.0, 40.0];

    let spec = FilterSpec::lowpass(fs, 8.0, 4); // 4th order lowpass @ 8 Hz

    for &freq in &test_freqs {
        let mut sig = Vec::with_capacity(n);
        for i in 0..n {
            let t = i as f64 / fs;
            sig.push((2.0 * PI * freq * t).sin());
        }

        let input_arr = Array1::from_vec(sig);
        let filtered = signal_filtfilt(&input_arr, &spec).unwrap();

        // Calculate steady-state amplitude in center slice [N/4 .. 3N/4]
        let slice = filtered.slice(ndarray::s![n / 4..3 * n / 4]);
        let mut max_amp = 0.0f64;
        for &v in slice.iter() {
            if v.abs() > max_amp {
                max_amp = v.abs();
            }
        }

        // Theoretical gain |H(f)| for zero-phase (double pass) 4th-order Butterworth lowpass at f_c=8 Hz
        // Single pass |H_1(f)| = 1 / sqrt(1 + (f/fc)^8)
        // Double pass |H_2(f)| = |H_1(f)|^2 = 1 / (1 + (f/fc)^8)
        let expected_gain = 1.0 / (1.0 + (freq / 8.0).powi(8));

        assert!(
            (max_amp - expected_gain).abs() < 0.05,
            "Frequency response deviation at {} Hz: got {}, expected {}",
            freq,
            max_amp,
            expected_gain
        );
    }
}

#[test]
fn test_dsp_property_invariants() {
    let sig: Vec<f64> = (0..5000).map(|i| (i as f64 * 0.1).sin() + 10.0).collect();
    let arr = Array1::from_vec(sig);

    // 1. Resampling property
    let resampled = signal_resample_poly(&arr, 1, 2).unwrap();
    assert_eq!(resampled.len(), 2500);

    // 2. Segmentation property
    let segs = signal_segment(&resampled, 1250, 1250, IncompleteTailPolicy::DropIncomplete).unwrap();
    assert_eq!(segs.len(), 2);
    for seg in &segs {
        assert_eq!(seg.len(), 1250);

        // 3. DC removal invariant: mean(seg) == 0
        let dc_free = signal_remove_dc(seg).unwrap();
        let mean: f64 = dc_free.iter().sum::<f64>() / dc_free.len() as f64;
        assert!(mean.abs() < 1e-12);

        // 4. Min-max normalization invariant: min >= 0, max <= 1
        let norm = signal_minmax(&dc_free, (0.0, 1.0), DegeneratePolicy::Error).unwrap();
        for &v in norm.iter() {
            assert!(v >= -1e-12 && v <= 1.0 + 1e-12);
        }
    }
}
