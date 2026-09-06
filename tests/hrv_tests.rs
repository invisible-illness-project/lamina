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

    // HRV calculations on clean N-N intervals
    let rmssd_raw = hrv_rmssd(&raw_rr).expect("raw rmssd");
    let rmssd_clean = hrv_rmssd(&clean_reject).expect("clean rmssd");
    assert!(
        rmssd_clean < rmssd_raw,
        "Cleaning ectopic/artifact intervals should lower RMSSD towards physiological baseline"
    );
}
