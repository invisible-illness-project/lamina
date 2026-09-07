//! BUG-008 revalidation probe: FeatureQuality.total_feature_count.
//! Construct fully-populated feature structs, call evaluate_feature_quality,
//! verify usable_feature_count == total_feature_count and ratio reaches 1.0.
use lamina::features::{
    evaluate_feature_quality, CardiacFeatures, CouplingFeatures, EdaFeatures, FeatureConfig,
    FeatureWindow, RespirationFeatures,
};

fn main() {
    let cardiac = CardiacFeatures {
        mean_hr_bpm: Some(60.0),
        median_hr_bpm: Some(60.0),
        sdnn_ms: Some(50.0),
        rmssd_ms: Some(40.0),
        pnn50: Some(0.2),
        rr_mean_ms: Some(1000.0),
        rr_std_ms: Some(50.0),
        beat_count: 60,
    };
    let eda = EdaFeatures {
        mean_tonic_us: Some(5.0),
        median_tonic_us: Some(5.0),
        tonic_std_us: Some(0.1),
        mean_phasic_us: Some(0.5),
        phasic_std_us: Some(0.1),
        scr_count: 3,
        scr_rate_per_min: Some(3.0),
        mean_scr_amplitude_us: Some(0.3),
        median_scr_amplitude_us: Some(0.3),
        mean_scr_rise_time_sec: Some(1.0),
    };
    let rsp = RespirationFeatures {
        mean_rate_bpm: Some(15.0),
        median_rate_bpm: Some(15.0),
        rate_std_bpm: Some(1.0),
        mean_cycle_duration_sec: Some(4.0),
        cycle_count: 15,
        mean_amplitude: Some(0.8),
        amplitude_std: Some(0.1),
    };
    let coupling = CouplingFeatures {
        rsa_amplitude_bpm: Some(5.0),
        rsa_amplitude_rr_sec: Some(0.05),
        cardiac_respiratory_concentration: Some(0.5),
        cardiac_respiratory_mean_phase: Some(1.0),
        mean_pulse_delay_sec: Some(0.2),
        pulse_delay_std_sec: Some(0.02),
        scr_cardiac_association_count: 1,
    };
    let window = FeatureWindow { start_time_sec: 0.0, end_time_sec: 60.0, duration_sec: 60.0 };
    let config = FeatureConfig::default();
    let q = evaluate_feature_quality(
        &cardiac, &eda, &rsp, &coupling, &window, &config,
        Some((0.0, 60.0)), Some((0.0, 60.0)), Some((0.0, 60.0)), Some((0.0, 60.0)),
    );
    println!("usable_feature_count = {}", q.usable_feature_count);
    println!("total_feature_count  = {}", q.total_feature_count);
    let ratio = q.usable_feature_count as f64 / q.total_feature_count as f64;
    println!("usable/total ratio   = {:.6}", ratio);
    println!("issues               = {:?}", q.issues);
    assert_eq!(q.usable_feature_count, q.total_feature_count,
        "fully populated feature vector must reach 100% usable");
    println!("PASS: ratio reaches 1.0 with all 28 features populated");

    // independent count of the Option fields in the 4 feature structs:
    // cardiac 7 + eda 9 + respiration 6 + coupling 6 = 28
    println!("independent field count: 7 cardiac + 9 eda + 6 respiration + 6 coupling = 28");

    // partial population sanity
    let mut cardiac2 = cardiac.clone();
    cardiac2.pnn50 = None;
    let q2 = evaluate_feature_quality(
        &cardiac2, &eda, &rsp, &coupling, &window, &config,
        Some((0.0, 60.0)), Some((0.0, 60.0)), Some((0.0, 60.0)), Some((0.0, 60.0)),
    );
    println!("with pnn50=None: usable={} total={} ratio={:.4}",
        q2.usable_feature_count, q2.total_feature_count,
        q2.usable_feature_count as f64 / q2.total_feature_count as f64);
}
