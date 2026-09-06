use lamina::error::SignalError;
use lamina::signal::filter::{FilterSpec, SosFilter, signal_filtfilt};
use ndarray::Array1;
use serde::Deserialize;
use std::f64::consts::PI;
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

/// Helper function to compute the complex frequency response H(e^{j omega}) of an SosFilter.
fn eval_freq_response(filter: &SosFilter, freq: f64, fs: f64) -> (f64, f64) {
    let w = 2.0 * PI * freq / fs;
    let mut h_re = 1.0;
    let mut h_im = 0.0;

    let cos1 = w.cos();
    let sin1 = -w.sin(); // e^{-j w} = cos(w) - j sin(w)
    let cos2 = (2.0 * w).cos();
    let sin2 = -(2.0 * w).sin();

    for sec in &filter.sections {
        // Num: b0 + b1 e^{-jw} + b2 e^{-j2w}
        let num_re = sec.b0 + sec.b1 * cos1 + sec.b2 * cos2;
        let num_im = sec.b1 * sin1 + sec.b2 * sin2;

        // Den: 1 + a1 e^{-jw} + a2 e^{-j2w}
        let den_re = 1.0 + sec.a1 * cos1 + sec.a2 * cos2;
        let den_im = sec.a1 * sin1 + sec.a2 * sin2;

        let den_mag2 = den_re * den_re + den_im * den_im;
        let sec_re = (num_re * den_re + num_im * den_im) / den_mag2;
        let sec_im = (num_im * den_re - num_re * den_im) / den_mag2;

        let next_re = h_re * sec_re - h_im * sec_im;
        let next_im = h_re * sec_im + h_im * sec_re;
        h_re = next_re;
        h_im = next_im;
    }

    (h_re, h_im)
}

fn eval_magnitude(filter: &SosFilter, freq: f64, fs: f64) -> f64 {
    let (re, im) = eval_freq_response(filter, freq, fs);
    re.hypot(im)
}

// -----------------------------------------------------------------------------
// Group A: Mathematical Invariants (SciPy-independent)
// -----------------------------------------------------------------------------

#[test]
fn test_group_a_mathematical_invariants() {
    let fs = 100.0;

    // Test across orders N = 1..=6
    for order in 1..=6 {
        // 1. Lowpass / Highpass: expected sections = ceil(order / 2)
        let lp_spec = FilterSpec::lowpass(fs, 10.0, order);
        let lp_filter = SosFilter::from_spec(&lp_spec).expect("LP design failed");
        let expected_lp_sections = (order + 1) / 2;
        assert_eq!(
            lp_filter.sections.len(),
            expected_lp_sections,
            "LP order {} should produce {} SOS sections",
            order,
            expected_lp_sections
        );

        // Verify digital pole stability for all sections: poles p satisfy |p| < 1.0
        for (i, sec) in lp_filter.sections.iter().enumerate() {
            assert!(sec.b0.is_finite());
            assert!(sec.b1.is_finite());
            assert!(sec.b2.is_finite());
            assert!(sec.a1.is_finite());
            assert!(sec.a2.is_finite());

            // Roots of 1 + a1 z^-1 + a2 z^-2 = z^2 + a1 z + a2 = 0
            if sec.a2.abs() < 1e-12 {
                // 1st order section: z + a1 = 0 => pole at -a1
                let pole_mag = sec.a1.abs();
                assert!(
                    pole_mag < 1.0,
                    "Order {} sec {} pole magnitude {} must be < 1",
                    order,
                    i,
                    pole_mag
                );
            } else {
                // 2nd order section
                let disc = sec.a1 * sec.a1 - 4.0 * sec.a2;
                if disc < 0.0 {
                    let pole_mag = sec.a2.sqrt();
                    assert!(
                        pole_mag < 1.0,
                        "Order {} sec {} pole mag {} must be < 1",
                        order,
                        i,
                        pole_mag
                    );
                } else {
                    let p1 = (-sec.a1 + disc.sqrt()) / 2.0;
                    let p2 = (-sec.a1 - disc.sqrt()) / 2.0;
                    assert!(
                        p1.abs() < 1.0 && p2.abs() < 1.0,
                        "Order {} sec {} poles ({}, {}) must be < 1",
                        order,
                        i,
                        p1,
                        p2
                    );
                }
            }
        }

        // Odd order real pole / real zero representation check
        if order % 2 == 1 {
            assert!(
                lp_filter.sections.iter().any(|s| s.a2 == 0.0),
                "Odd order {} must have a section with real pole (a2 = 0.0)",
                order
            );
            assert!(
                lp_filter.sections.iter().any(|s| s.b2 == 0.0),
                "Odd order {} must have a section with real zero (b2 = 0.0)",
                order
            );
        }

        // 2. Bandpass / Notch: expected sections = order (2N poles)
        let bp_spec = FilterSpec::bandpass(fs, 5.0, 15.0, order);
        let bp_filter = SosFilter::from_spec(&bp_spec).expect("BP design failed");
        assert_eq!(
            bp_filter.sections.len(),
            order,
            "BP order {} must produce {} SOS sections (2N poles)",
            order,
            order
        );

        let notch_spec = FilterSpec::notch(fs, 18.0, 22.0, order);
        let notch_filter = SosFilter::from_spec(&notch_spec).expect("Notch design failed");
        assert_eq!(
            notch_filter.sections.len(),
            order,
            "Notch order {} must produce {} SOS sections (2N poles)",
            order,
            order
        );
    }
}

// -----------------------------------------------------------------------------
// Group B: Independent Frequency-Domain Response (SciPy-independent)
// -----------------------------------------------------------------------------

#[test]
fn test_group_b_frequency_domain_responses() {
    let fs = 100.0;
    let target_cutoff_mag = 1.0 / 2.0_f64.sqrt(); // 1/sqrt(2) approx 0.70710678 (-3.0103 dB)

    for order in [1, 2, 3, 4, 5, 6] {
        // 1. Lowpass (fc = 10 Hz)
        let fc = 10.0;
        let lp_spec = FilterSpec::lowpass(fs, fc, order);
        let lp_filter = SosFilter::from_spec(&lp_spec).unwrap();

        let mag_dc = eval_magnitude(&lp_filter, 0.0, fs);
        let mag_fc = eval_magnitude(&lp_filter, fc, fs);
        let mag_high = eval_magnitude(&lp_filter, 40.0, fs);

        assert!(
            (mag_dc - 1.0).abs() < 1e-4,
            "LP order {} DC mag = {}, expected 1.0",
            order,
            mag_dc
        );
        assert!(
            (mag_fc - target_cutoff_mag).abs() < 1e-3,
            "LP order {} fc mag = {}, expected 1/sqrt(2) = {}",
            order,
            mag_fc,
            target_cutoff_mag
        );
        let expected_stopband_limit = if order == 1 { 0.25 } else { 0.1 };
        assert!(
            mag_high < expected_stopband_limit,
            "LP order {} stopband mag = {}, expected < {}",
            order,
            mag_high,
            expected_stopband_limit
        );

        // 2. Highpass (fc = 10 Hz)
        let hp_spec = FilterSpec::highpass(fs, fc, order);
        let hp_filter = SosFilter::from_spec(&hp_spec).unwrap();

        let mag_low = eval_magnitude(&hp_filter, 1.0, fs);
        let mag_fc_hp = eval_magnitude(&hp_filter, fc, fs);
        let mag_nyq = eval_magnitude(&hp_filter, 49.0, fs);

        assert!(
            mag_low < 0.1,
            "HP order {} stopband mag = {}, expected < 0.1",
            order,
            mag_low
        );
        assert!(
            (mag_fc_hp - target_cutoff_mag).abs() < 1e-3,
            "HP order {} fc mag = {}, expected 1/sqrt(2)",
            order,
            mag_fc_hp
        );
        assert!(
            (mag_nyq - 1.0).abs() < 1e-3,
            "HP order {} passband mag = {}, expected 1.0",
            order,
            mag_nyq
        );

        // 3. Bandpass (5 - 15 Hz)
        let f_low = 5.0_f64;
        let f_high = 15.0_f64;
        let f0 = (f_low * f_high).sqrt(); // 8.66 Hz geometric center
        let bp_spec = FilterSpec::bandpass(fs, f_low, f_high, order);
        let bp_filter = SosFilter::from_spec(&bp_spec).unwrap();

        let mag_below = eval_magnitude(&bp_filter, 1.0, fs);
        let mag_flow = eval_magnitude(&bp_filter, f_low, fs);
        let mag_center = eval_magnitude(&bp_filter, f0, fs);
        let mag_fhigh = eval_magnitude(&bp_filter, f_high, fs);
        let mag_above = eval_magnitude(&bp_filter, 40.0, fs);

        let expected_bp_stopband_limit = if order == 1 { 0.2 } else { 0.1 };
        assert!(
            mag_below < expected_bp_stopband_limit,
            "BP order {} below stopband = {}, expected < {}",
            order,
            mag_below,
            expected_bp_stopband_limit
        );
        assert!(
            (mag_flow - target_cutoff_mag).abs() < 1e-3,
            "BP order {} flow mag = {}, expected 1/sqrt(2)",
            order,
            mag_flow
        );
        assert!(
            mag_center > 0.8,
            "BP order {} center mag = {}, expected > 0.8",
            order,
            mag_center
        );
        assert!(
            (mag_fhigh - target_cutoff_mag).abs() < 1e-3,
            "BP order {} fhigh mag = {}, expected 1/sqrt(2)",
            order,
            mag_fhigh
        );
        assert!(
            mag_above < expected_bp_stopband_limit,
            "BP order {} above stopband = {}, expected < {}",
            order,
            mag_above,
            expected_bp_stopband_limit
        );

        // 4. Notch / Bandstop (18 - 22 Hz)
        let fn_low = 18.0_f64;
        let fn_high = 22.0_f64;
        let fn0 = (fn_low * fn_high).sqrt();
        let notch_spec = FilterSpec::notch(fs, fn_low, fn_high, order);
        let notch_filter = SosFilter::from_spec(&notch_spec).unwrap();

        let mag_pass1 = eval_magnitude(&notch_filter, 2.0, fs);
        let mag_notch_center = eval_magnitude(&notch_filter, fn0, fs);
        let mag_pass2 = eval_magnitude(&notch_filter, 40.0, fs);

        assert!(
            (mag_pass1 - 1.0).abs() < 0.05,
            "Notch order {} pass1 mag = {}, expected ~1.0",
            order,
            mag_pass1
        );
        assert!(
            mag_notch_center < 0.1,
            "Notch order {} center mag = {}, expected < 0.1",
            order,
            mag_notch_center
        );
        assert!(
            (mag_pass2 - 1.0).abs() < 0.05,
            "Notch order {} pass2 mag = {}, expected ~1.0",
            order,
            mag_pass2
        );
    }
}

// -----------------------------------------------------------------------------
// Group C: SOS Coefficient Parity (where canonical)
// -----------------------------------------------------------------------------

#[test]
fn test_group_c_sos_coefficient_parity() {
    let file = File::open("tests/golden_filter.json").expect("Failed to open golden filter JSON");
    let reader = BufReader::new(file);
    let configs: Vec<GoldenFilterConfig> =
        serde_json::from_reader(reader).expect("Failed to parse golden filter JSON");

    for cfg in &configs {
        let spec = match cfg.filter_type.as_str() {
            "lowpass" => FilterSpec::lowpass(cfg.fs, cfg.cutoff[0], cfg.order),
            "highpass" => FilterSpec::highpass(cfg.fs, cfg.cutoff[0], cfg.order),
            "bandpass" => FilterSpec::bandpass(cfg.fs, cfg.cutoff[0], cfg.cutoff[1], cfg.order),
            "notch" => FilterSpec::notch(cfg.fs, cfg.cutoff[0], cfg.cutoff[1], cfg.order),
            _ => panic!("Unknown filter type: {}", cfg.filter_type),
        };

        let lamina_filter = SosFilter::from_spec(&spec).expect("Lamina filter design failed");

        assert_eq!(
            lamina_filter.sections.len(),
            cfg.sos_coefficients.len(),
            "Section count mismatch for {} order {}",
            cfg.filter_type,
            cfg.order
        );

        for (sec, scipy_row) in lamina_filter
            .sections
            .iter()
            .zip(cfg.sos_coefficients.iter())
        {
            let b0_err = (sec.b0 - scipy_row[0]).abs();
            let b1_err = (sec.b1 - scipy_row[1]).abs();
            let b2_err = (sec.b2 - scipy_row[2]).abs();
            let a1_err = (sec.a1 - scipy_row[4]).abs();
            let a2_err = (sec.a2 - scipy_row[5]).abs();

            let max_coeff_err = b0_err.max(b1_err).max(b2_err).max(a1_err).max(a2_err);

            assert!(
                max_coeff_err < 1e-6,
                "Coefficient mismatch for {} order {}: max err = {:.6e}",
                cfg.filter_type,
                cfg.order,
                max_coeff_err
            );
        }
    }
}

// -----------------------------------------------------------------------------
// Group D: End-to-End Filtering Parity (Primary Acceptance Criterion)
// -----------------------------------------------------------------------------

#[test]
fn test_group_d_end_to_end_filtering_parity() {
    let file = File::open("tests/golden_filter.json").expect("Failed to open golden filter JSON");
    let reader = BufReader::new(file);
    let configs: Vec<GoldenFilterConfig> =
        serde_json::from_reader(reader).expect("Failed to parse golden filter JSON");

    assert!(!configs.is_empty(), "Golden filter JSON is empty");

    let mut max_abs_error = 0.0_f64;
    let mut total_rms_error = 0.0_f64;
    let total_cases = configs.len();

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
        if abs_err_max > max_abs_error {
            max_abs_error = abs_err_max;
        }
        total_rms_error += rms_err;

        // Primary acceptance criterion: end-to-end signal parity vs SciPy butter -> sosfiltfilt
        assert!(
            abs_err_max < 0.05,
            "End-to-end parity mismatch for {} [{}] order {}: max error = {}, rms = {}",
            cfg.filter_type,
            cfg.signal_name,
            cfg.order,
            abs_err_max,
            rms_err
        );
    }

    let avg_rms = total_rms_error / total_cases as f64;
    println!(
        "End-to-End Filtering Parity (Group D) Verified across {} test cases.",
        total_cases
    );
    println!(
        "Max Absolute Error: {:.6e}, Average RMS Error: {:.6e}",
        max_abs_error, avg_rms
    );
}

#[test]
fn test_boundary_and_constant_signals() {
    let fs = 100.0;
    let n = 200;
    let constant_signal = Array1::<f64>::from_elem(n, 5.0);

    let spec = FilterSpec::lowpass(fs, 5.0, 4);
    let filtered =
        signal_filtfilt(&constant_signal, &spec).expect("Filtering constant signal failed");

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
    let short_signal = Array1::<f64>::from_elem(5, 1.0);

    let res = signal_filtfilt(&short_signal, &spec);
    assert!(
        matches!(res, Err(SignalError::InsufficientSamples { .. })),
        "Expected InsufficientSamples error for short signal, got {:?}",
        res
    );
}
