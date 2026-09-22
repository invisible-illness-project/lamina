use lamina::ppg::pulse_lm::{PulseLmPipelineSpec, ppg_preprocess_pulselm};
use ndarray::Array1;
use std::f64::consts::PI;

#[test]
fn test_pulselm_pipeline_spec_sha256_hash() {
    let spec = PulseLmPipelineSpec::default();
    let hash1 = spec.compute_sha256_hash();
    assert_eq!(hash1.len(), 64);

    let spec2 = PulseLmPipelineSpec::default();
    let hash2 = spec2.compute_sha256_hash();
    assert_eq!(hash1, hash2); // Deterministic

    let mut spec3 = PulseLmPipelineSpec::default();
    spec3.filter_cutoff_hz = 10.0; // modified spec
    let hash3 = spec3.compute_sha256_hash();
    assert_ne!(hash1, hash3); // Hash reflects spec changes
}

#[test]
fn test_pulselm_pipeline_end_to_end_synthetic_fixture() {
    // 250 Hz sampling rate, 35 seconds = 8750 input samples
    let fs_in = 250.0;
    let duration_sec = 35.0;
    let n_in = (fs_in * duration_sec) as usize;

    let mut raw_ppg = Vec::with_capacity(n_in);

    // Multi-tone synthetic signal:
    // - DC baseline offset = +15.0
    // - 1.2 Hz pulse wave (72 BPM) = 2.0 * sin(2*pi*1.2*t)
    // - 20.0 Hz high-frequency noise = 0.8 * sin(2*pi*20*t)
    for i in 0..n_in {
        let t = i as f64 / fs_in;
        let dc = 15.0;
        let pulse = 2.0 * (2.0 * PI * 1.2 * t).sin();
        let noise = 0.8 * (2.0 * PI * 20.0 * t).sin();
        raw_ppg.push(dc + pulse + noise);
    }

    let input_arr = Array1::from_vec(raw_ppg);
    let segments = ppg_preprocess_pulselm(&input_arr, fs_in)
        .expect("PulseLM preprocessing pipeline failed");

    // 35s resampled to 125 Hz = 4375 samples.
    // 10s windows = 1250 samples per window -> 3 complete windows (30s), trailing 5s dropped.
    assert_eq!(segments.len(), 3);

    for (idx, seg) in segments.iter().enumerate() {
        assert_eq!(
            seg.len(),
            1250,
            "Segment {} must have exactly 1250 samples (10s @ 125 Hz)",
            idx
        );

        // 1. Check Min-Max scaling bounds: strictly in [0.0, 1.0]
        let mut min_val = f64::MAX;
        let mut max_val = f64::MIN;

        for &v in seg.iter() {
            assert!(v >= -1e-12 && v <= 1.0 + 1e-12);
            if v < min_val {
                min_val = v;
            }
            if v > max_val {
                max_val = v;
            }
        }

        assert!((min_val - 0.0).abs() < 1e-6);
        assert!((max_val - 1.0).abs() < 1e-6);

        // 2. High-frequency noise suppression check (20 Hz noise attenuated by 4th-order lowpass @ 8 Hz)
        // Check power spectral amplitude of 20 Hz noise relative to 1.2 Hz pulse in segment
        let fs_target = 125.0;
        let mut pulse_re = 0.0;
        let mut pulse_im = 0.0;
        let mut noise_re = 0.0;
        let mut noise_im = 0.0;

        for (i, &val) in seg.iter().enumerate() {
            let t = i as f64 / fs_target;
            let w_pulse = 2.0 * PI * 1.2 * t;
            pulse_re += val * w_pulse.cos();
            pulse_im -= val * w_pulse.sin();

            let w_noise = 2.0 * PI * 20.0 * t;
            noise_re += val * w_noise.cos();
            noise_im -= val * w_noise.sin();
        }

        let pulse_amp = pulse_re.hypot(pulse_im);
        let noise_amp = noise_re.hypot(noise_im);

        assert!(
            noise_amp / pulse_amp < 0.05,
            "20 Hz high-frequency noise not sufficiently attenuated in segment {}: noise_amp={}, pulse_amp={}",
            idx,
            noise_amp,
            pulse_amp
        );
    }
}

#[test]
fn test_legacy_ppg_clean_compatibility_preserved() {
    let arr = Array1::from_elem(100, 1.0);
    // Legacy ppg_clean contract: returns single 1D Array1<f64> cleaned via 0.5-8.0 Hz bandpass filter
    let cleaned = lamina::ppg::ppg_clean(&arr, 100.0).unwrap();
    assert_eq!(cleaned.len(), arr.len());
}
