use lamina::signal::resample::signal_resample_poly;
use ndarray::Array1;
use serde::Deserialize;
use std::f64::consts::PI;
use std::fs::File;
use std::io::BufReader;

#[derive(Deserialize)]
struct GoldenResampleConfig {
    up: usize,
    down: usize,
    signal_name: String,
    input_signal: Vec<f64>,
    expected_output: Vec<f64>,
}

#[test]
fn test_resample_poly_scipy_parity() {
    let file = File::open("tests/golden_resample.json")
        .expect("Failed to open golden_resample.json");
    let reader = BufReader::new(file);
    let configs: Vec<GoldenResampleConfig> =
        serde_json::from_reader(reader).expect("Failed to parse golden_resample.json");

    for config in configs {
        let input = Array1::from_vec(config.input_signal);
        let resampled = signal_resample_poly(&input, config.up, config.down)
            .expect("signal_resample_poly failed");

        assert_eq!(
            resampled.len(),
            config.expected_output.len(),
            "Output length mismatch for {} (up={}, down={})",
            config.signal_name,
            config.up,
            config.down
        );

        let mut max_err = 0.0f64;
        for (a, b) in resampled.iter().zip(config.expected_output.iter()) {
            let err = (a - b).abs();
            if err > max_err {
                max_err = err;
            }
        }

        assert!(
            max_err < 1e-4,
            "Max error {} exceeds tolerance 1e-4 for {} (up={}, down={})",
            max_err,
            config.signal_name,
            config.up,
            config.down
        );
    }
}

#[test]
fn test_resample_anti_aliasing_suppression() {
    // 500 Hz input sampling rate, 2 seconds = 1000 samples
    let fs_in = 500.0;
    let n = (fs_in * 2.0) as usize;
    let mut signal = Vec::with_capacity(n);

    // Desired 1.5 Hz pulse + 100 Hz high-frequency tone (above 62.5 Hz Nyquist of 125 Hz target)
    for i in 0..n {
        let t = i as f64 / fs_in;
        let pulse = (2.0 * PI * 1.5 * t).sin();
        let high_tone = 0.5 * (2.0 * PI * 100.0 * t).sin();
        signal.push(pulse + high_tone);
    }

    let input_arr = Array1::from_vec(signal);
    // Downsample 500 Hz -> 125 Hz (up=1, down=4)
    let resampled = signal_resample_poly(&input_arr, 1, 4)
        .expect("Downsampling failed");

    assert_eq!(resampled.len(), 250);

    // Compute DFT magnitude of 100 Hz alias frequency in resampled signal
    // 100 Hz folded at 62.5 Hz Nyquist would appear at 25 Hz in 125 Hz grid
    let fs_out = 125.0;
    let target_alias_freq = 25.0; // 125 - 100 = 25 Hz folded
    let target_pulse_freq = 1.5;

    let mut pulse_mag_re = 0.0;
    let mut pulse_mag_im = 0.0;
    let mut alias_mag_re = 0.0;
    let mut alias_mag_im = 0.0;

    for (i, &val) in resampled.iter().enumerate() {
        let t = i as f64 / fs_out;
        let w_pulse = 2.0 * PI * target_pulse_freq * t;
        pulse_mag_re += val * w_pulse.cos();
        pulse_mag_im -= val * w_pulse.sin();

        let w_alias = 2.0 * PI * target_alias_freq * t;
        alias_mag_re += val * w_alias.cos();
        alias_mag_im -= val * w_alias.sin();
    }

    let pulse_amp = (pulse_mag_re.hypot(pulse_mag_im) * 2.0) / resampled.len() as f64;
    let alias_amp = (alias_mag_re.hypot(alias_mag_im) * 2.0) / resampled.len() as f64;

    assert!(
        pulse_amp > 0.9,
        "Desired 1.5 Hz pulse amplitude degraded: got {}",
        pulse_amp
    );
    assert!(
        alias_amp < 0.01,
        "Spectral aliasing failed: 100 Hz tone folded with amplitude {}",
        alias_amp
    );
}

#[test]
fn test_resample_identity_and_edge_cases() {
    let arr = Array1::from_vec(vec![1.0, 2.0, 3.0, 4.0, 5.0]);
    let identity = signal_resample_poly(&arr, 4, 4).unwrap();
    assert_eq!(identity, arr);

    let empty = Array1::from_vec(vec![]);
    assert!(signal_resample_poly(&empty, 1, 2).is_err());
}
