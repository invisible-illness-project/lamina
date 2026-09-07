use lamina::hrv::intervals::peaks_to_intervals;
use lamina::hrv::time::{hrv_mean_nn, hrv_rmssd};
use ndarray::{Array1, array};

#[test]
fn test_peaks_to_intervals() {
    let mut peaks = Array1::<bool>::from_elem(1000, false);
    peaks[0] = true;
    peaks[100] = true;
    peaks[300] = true;
    peaks[600] = true;

    let intervals = peaks_to_intervals(&peaks, 100.0).expect("Interval calculation failed");

    let expected = array![1000.0, 2000.0, 3000.0];
    assert_eq!(intervals, expected);
}

#[test]
fn test_hrv_rmssd_mean() {
    let intervals = array![800.0, 850.0, 820.0, 900.0];

    let mean = hrv_mean_nn(&intervals).expect("Mean NN failed");
    assert!((mean - 842.5).abs() < 1e-6);

    let rmssd = hrv_rmssd(&intervals).expect("RMSSD failed");
    assert!((rmssd - 57.15476).abs() < 1e-4);
}

#[test]
fn test_hrv_interval_quality_and_correction_policies() {
    use lamina::hrv::{CorrectionPolicy, IntervalQuality, classify_intervals, clean_rr_intervals};

    // Series containing ectopic beat (400ms premature + 1600ms compensatory) and artifact (100ms)
    let raw_rr = array![800.0, 805.0, 400.0, 1600.0, 795.0, 800.0, 100.0, 810.0];

    let q = classify_intervals(&raw_rr, Some(0.20));
    assert_eq!(q[0], IntervalQuality::NormalNN);
    assert_eq!(q[1], IntervalQuality::NormalNN);
    assert_eq!(q[2], IntervalQuality::EctopicRR);
    assert_eq!(q[3], IntervalQuality::EctopicRR);
    assert_eq!(q[4], IntervalQuality::NormalNN);
    assert_eq!(q[5], IntervalQuality::NormalNN);
    assert_eq!(q[6], IntervalQuality::ArtifactRR);
    assert_eq!(q[7], IntervalQuality::NormalNN);

    // Policy: RejectInvalid
    let clean_reject = clean_rr_intervals(&raw_rr, &CorrectionPolicy::RejectInvalid)
        .expect("clean_rr_intervals reject failed");
    let expected_reject = array![800.0, 805.0, 795.0, 800.0, 810.0];
    assert_eq!(clean_reject, expected_reject);

    // Policy: InterpolateLinear
    let clean_interp = clean_rr_intervals(&raw_rr, &CorrectionPolicy::InterpolateLinear)
        .expect("clean_rr_intervals interp failed");
    assert_eq!(clean_interp.len(), raw_rr.len());
    assert!((clean_interp[2] - 801.666).abs() < 1e-2);

    // Policy: InterpolateCubic
    let clean_cubic = clean_rr_intervals(&raw_rr, &CorrectionPolicy::InterpolateCubic)
        .expect("clean_rr_intervals cubic failed");
    assert_eq!(clean_cubic.len(), raw_rr.len());
    assert!(clean_cubic[2].is_finite() && clean_cubic[3].is_finite());

    // HRV calculations on clean N-N intervals
    let rmssd_raw = hrv_rmssd(&raw_rr).expect("raw rmssd");
    let rmssd_clean = hrv_rmssd(&clean_reject).expect("clean rmssd");
    assert!(
        rmssd_clean < rmssd_raw,
        "Cleaning ectopic/artifact intervals should lower RMSSD towards physiological baseline"
    );
}

#[test]
fn test_hrv_7case_counterexample_matrix() {
    use lamina::hrv::{IntervalQuality, classify_intervals};

    // Case 1: Normal sinus rhythm
    let c1 = array![800.0, 805.0, 798.0, 802.0, 800.0];
    let q1 = classify_intervals(&c1, Some(0.20));
    assert!(q1.iter().all(|&q| q == IntervalQuality::NormalNN));

    // Case 2: Physiological Respiratory Sinus Arrhythmia (RSA)
    let c2 = array![750.0, 780.0, 810.0, 840.0, 820.0, 790.0, 760.0];
    let q2 = classify_intervals(&c2, Some(0.20));
    assert!(
        q2.iter().all(|&q| q == IntervalQuality::NormalNN),
        "Physiological RSA gradual variation must not be falsely rejected as ectopic"
    );

    // Case 3: Isolated PVC (premature + compensatory)
    let c3 = array![800.0, 800.0, 450.0, 1150.0, 800.0, 800.0];
    let q3 = classify_intervals(&c3, Some(0.20));
    assert_eq!(q3[2], IntervalQuality::EctopicRR);
    assert_eq!(q3[3], IntervalQuality::EctopicRR);

    // Case 4: Sustained Bigeminy
    let c4 = array![600.0, 1000.0, 600.0, 1000.0, 600.0, 1000.0, 600.0];
    let q4 = classify_intervals(&c4, Some(0.20));
    assert!(
        q4.iter()
            .filter(|&&q| q == IntervalQuality::EctopicRR)
            .count()
            >= 4,
        "Sustained bigeminy alternating sequence must be classified as EctopicRR"
    );

    // Case 5: Trigeminy
    let c5 = array![800.0, 800.0, 500.0, 800.0, 800.0, 500.0, 800.0];
    let q5 = classify_intervals(&c5, Some(0.20));
    assert_eq!(q5[2], IntervalQuality::EctopicRR);
    assert_eq!(q5[5], IntervalQuality::EctopicRR);

    // Case 6: Out-of-bounds Motion Artifact (<300ms or >2000ms)
    let c6 = array![800.0, 150.0, 800.0, 2500.0, 800.0];
    let q6 = classify_intervals(&c6, Some(0.20));
    assert_eq!(q6[1], IntervalQuality::ArtifactRR);
    assert_eq!(q6[3], IntervalQuality::ArtifactRR);

    // Case 7: Missing / Non-finite sample
    let c7 = array![800.0, f64::NAN, 800.0];
    let q7 = classify_intervals(&c7, Some(0.20));
    assert_eq!(q7[1], IntervalQuality::Missing);
}

#[test]
fn test_hrv_empty_container_and_post_filter_contract() {
    use lamina::error::SignalError;
    use lamina::hrv::{CorrectionPolicy, clean_rr_intervals};

    // 1. True empty input container (0 samples) -> Err(EmptySignal)
    let empty_container = Array1::<f64>::zeros(0);
    assert!(matches!(
        clean_rr_intervals(&empty_container, &CorrectionPolicy::RejectInvalid),
        Err(SignalError::EmptySignal)
    ));
    assert!(matches!(
        clean_rr_intervals(&empty_container, &CorrectionPolicy::InterpolateLinear),
        Err(SignalError::EmptySignal)
    ));
    assert!(matches!(
        clean_rr_intervals(&empty_container, &CorrectionPolicy::InterpolateCubic),
        Err(SignalError::EmptySignal)
    ));

    // 2. Non-empty input container (4 samples) where 100% are invalid -> Ok(empty Array1)
    let all_ectopic = array![600.0, 1000.0, 600.0, 1000.0];
    let cleaned_reject = clean_rr_intervals(&all_ectopic, &CorrectionPolicy::RejectInvalid)
        .expect("RejectInvalid on 100% ectopic should return Ok(empty)");
    assert_eq!(
        cleaned_reject.len(),
        0,
        "Post-filter result should be empty Array1"
    );

    let cleaned_interp = clean_rr_intervals(&all_ectopic, &CorrectionPolicy::InterpolateLinear)
        .expect("InterpolateLinear on 100% ectopic should return Ok(empty)");
    assert_eq!(cleaned_interp.len(), 0);

    let cleaned_cubic = clean_rr_intervals(&all_ectopic, &CorrectionPolicy::InterpolateCubic)
        .expect("InterpolateCubic on 100% ectopic should return Ok(empty)");
    assert_eq!(cleaned_cubic.len(), 0);

    // 3. Metric calls on empty post-filter array -> InsufficientPeaks { provided: 0 }
    assert!(matches!(
        hrv_mean_nn(&cleaned_reject),
        Err(SignalError::InsufficientPeaks {
            required: 1,
            provided: 0
        })
    ));

    assert!(matches!(
        hrv_rmssd(&cleaned_reject),
        Err(SignalError::InsufficientPeaks {
            required: 2,
            provided: 0
        })
    ));

    // 4. Metric call on single-element array -> InsufficientPeaks { provided: 1 } for RMSSD
    let single_val = array![800.0];
    assert_eq!(
        hrv_mean_nn(&single_val).expect("Mean NN of 1 element"),
        800.0
    );
    assert!(matches!(
        hrv_rmssd(&single_val),
        Err(SignalError::InsufficientPeaks {
            required: 2,
            provided: 1
        })
    ));
}

#[test]
fn test_natural_cubic_spline_contract() {
    use lamina::hrv::{CorrectionPolicy, clean_rr_intervals};

    // 1. Verification of linear fallback when valid points < 4
    let three_valid_raw = array![800.0, 805.0, 100.0, 795.0]; // 1 artifact @ idx 2, 3 valid NN
    let clean_cubic_fallback =
        clean_rr_intervals(&three_valid_raw, &CorrectionPolicy::InterpolateCubic)
            .expect("Cubic fallback test failed");
    let clean_linear = clean_rr_intervals(&three_valid_raw, &CorrectionPolicy::InterpolateLinear)
        .expect("Linear compare failed");
    assert_eq!(
        clean_cubic_fallback, clean_linear,
        "Cubic interpolation must fall back to linear when valid points < 4"
    );

    // 2. Verification of cubic curvature on 4+ valid points
    let curved_raw = array![800.0, 850.0, 100.0, 840.0, 800.0, 770.0]; // idx 2 is artifact
    let clean_c = clean_rr_intervals(&curved_raw, &CorrectionPolicy::InterpolateCubic).unwrap();
    let clean_l = clean_rr_intervals(&curved_raw, &CorrectionPolicy::InterpolateLinear).unwrap();

    assert_eq!(clean_c.len(), curved_raw.len());
    assert!(clean_c.iter().all(|v| v.is_finite()));
    // Cubic and linear should differ on curved points
    assert!((clean_c[2] - clean_l[2]).abs() > 1e-4);

    // 3. Endpoint clamping check: first or last element invalid
    let end_invalid_raw = array![100.0, 800.0, 810.0, 805.0, 800.0, 795.0]; // idx 0 invalid
    let clean_end =
        clean_rr_intervals(&end_invalid_raw, &CorrectionPolicy::InterpolateCubic).unwrap();
    assert_eq!(
        clean_end[0], 800.0,
        "Endpoint should clamp to nearest valid point"
    );
}
