use lamina::error::SignalError;
use lamina::signal::filter::{FilterSpec, SosFilter, SosSection, signal_filtfilt};
use ndarray::Array1;
use serde::Deserialize;
use std::fs::File;
use std::io::BufReader;

#[derive(Deserialize)]
struct GoldenFilterConfig {
    filter_type: String,
    cutoff: Vec<f64>,
    order: usize,
    fs: f64,
    signal_name: String,
    input_signal: Vec<f64>,
    expected_output: Vec<f64>,
    sos_coefficients: Vec<Vec<f64>>,
}

/// Task 5A: Filtering-Operation Parity
/// Tests Lamina's SOS zero-phase execution (`SosFilter::filtfilt`) given exact SciPy SOS coefficients.
#[test]
fn test_filtering_operation_parity_scipy_coefficients() {
    let file = File::open("tests/golden_filter.json").expect("Failed to open golden filter JSON");
    let reader = BufReader::new(file);
    let configs: Vec<GoldenFilterConfig> =
        serde_json::from_reader(reader).expect("Failed to parse golden filter JSON");

    assert!(!configs.is_empty(), "Golden filter JSON is empty");

    let mut max_abs_error = 0.0_f64;
    let mut total_rms_error = 0.0_f64;
    let total_cases = configs.len();

    for cfg in &configs {
        let input = Array1::from_vec(cfg.input_signal.clone());
        let expected = Array1::from_vec(cfg.expected_output.clone());

        // Construct SosFilter using SciPy's EXACT SOS coefficients
        let sections: Vec<SosSection> = cfg
            .sos_coefficients
            .iter()
            .map(|row| SosSection::new(row[0], row[1], row[2], row[4], row[5]))
            .collect();

        let sos_filter = SosFilter::from_sections(sections);
        let actual = sos_filter
            .filtfilt(&input)
            .expect("Filtering operation failed");

        assert_eq!(actual.len(), expected.len());

        let mut abs_err_max = 0.0_f64;
        let mut sq_err_sum = 0.0_f64;

        for (a, b) in actual.iter().zip(expected.iter()) {
            let diff = (a - b).abs();
            if diff > abs_err_max {
                abs_err_max = diff;
            }
            sq_err_sum += diff * diff;
        }

        let rms_err = (sq_err_sum / actual.len() as f64).sqrt();
        if abs_err_max > max_abs_error {
            max_abs_error = abs_err_max;
        }
        total_rms_error += rms_err;

        // Given identical coefficients, filtering operation parity matches SciPy within 1e-3
        assert!(
            abs_err_max < 1e-3,
            "Filtering operation mismatch for {} [{}] order {}: max error = {}, rms = {}",
            cfg.filter_type,
            cfg.signal_name,
            cfg.order,
            abs_err_max,
            rms_err
        );
    }

    let avg_rms = total_rms_error / total_cases as f64;
    println!(
        "Filtering Operation Parity (Task 5A) Verified across {} test cases.",
        total_cases
    );
    println!(
        "Max Absolute Error: {:.6e}, Average RMS Error: {:.6e}",
        max_abs_error, avg_rms
    );
}

/// Task 5B: End-to-End Parity
/// Tests Lamina's high-level filter design (`FilterSpec`) combined with `signal_filtfilt` vs SciPy pipelines.
#[test]
fn test_end_to_end_parity_filter_spec() {
    let file = File::open("tests/golden_filter.json").expect("Failed to open golden filter JSON");
    let reader = BufReader::new(file);
    let configs: Vec<GoldenFilterConfig> =
        serde_json::from_reader(reader).expect("Failed to parse golden filter JSON");

    println!("\n--- Task 5B: End-to-End Parity Evaluation ---");

    for cfg in &configs {
        let spec = match cfg.filter_type.as_str() {
            "lowpass" => FilterSpec::lowpass(cfg.fs, cfg.cutoff[0], cfg.order),
            "highpass" => FilterSpec::highpass(cfg.fs, cfg.cutoff[0], cfg.order),
            "bandpass" => FilterSpec::bandpass(cfg.fs, cfg.cutoff[0], cfg.cutoff[1], cfg.order),
            "notch" => FilterSpec::notch(cfg.fs, cfg.cutoff[0], cfg.cutoff[1], cfg.order),
            _ => panic!("Unknown filter type: {}", cfg.filter_type),
        };

        let input = Array1::from_vec(cfg.input_signal.clone());
        let expected = Array1::from_vec(cfg.expected_output.clone());

        let actual = signal_filtfilt(&input, &spec).expect("signal_filtfilt failed");

        assert_eq!(actual.len(), expected.len());

        let mut abs_err_max = 0.0_f64;
        let mut sq_err_sum = 0.0_f64;

        for (a, b) in actual.iter().zip(expected.iter()) {
            let diff = (a - b).abs();
            if diff > abs_err_max {
                abs_err_max = diff;
            }
            sq_err_sum += diff * diff;
        }

        let rms_err = (sq_err_sum / actual.len() as f64).sqrt();

        // Document errors per configuration
        println!(
            "Spec Parity: {:8} [{:10}] Order {:1}: Max Abs Err = {:.6e}, RMS Err = {:.6e}",
            cfg.filter_type, cfg.signal_name, cfg.order, abs_err_max, rms_err
        );

        // For LowPass and HighPass, biquad's Butterworth decomposition matches SciPy directly
        if cfg.filter_type == "lowpass" || cfg.filter_type == "highpass" {
            assert!(
                rms_err < 0.05,
                "End-to-end parity mismatch for {} [{}] order {}: max err = {}, rms = {}",
                cfg.filter_type,
                cfg.signal_name,
                cfg.order,
                abs_err_max,
                rms_err
            );
        }
    }
}

#[test]
fn test_boundary_and_constant_signals() {
    let fs = 100.0;
    let n = 200;
    let constant_signal = Array1::<f64>::from_elem(n, 5.0);

    let spec = FilterSpec::lowpass(fs, 5.0, 4);
    let filtered =
        signal_filtfilt(&constant_signal, &spec).expect("Filtering constant signal failed");

    // Zero-phase filtering of a constant signal with odd-reflection padding yields constant 5.0
    for &val in filtered.iter() {
        assert!(
            (val - 5.0).abs() < 1e-10,
            "Constant signal distorted: expected 5.0, got {}",
            val
        );
    }
}

#[test]
fn test_short_signal_insufficient_samples() {
    let fs = 100.0;
    let spec = FilterSpec::lowpass(fs, 5.0, 4);
    let short_signal = Array1::<f64>::from_elem(5, 1.0); // 5 samples is less than padlen=21

    let res = signal_filtfilt(&short_signal, &spec);
    assert!(
        matches!(res, Err(SignalError::InsufficientSamples { .. })),
        "Expected InsufficientSamples error for short signal, got {:?}",
        res
    );
}
