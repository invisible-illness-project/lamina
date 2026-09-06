use lamina::autonomic::{
    AutonomicBaseline, AutonomicEstimator, AutonomicEstimatorConfig, BaselineFeatureStats,
    FeatureDirection, NormalizationConfig, SmoothingConfig,
};
use lamina::error::SignalError;
use lamina::features::{
    CardiacFeatures, CouplingFeatures, EdaFeatures, FeatureCoverage, FeatureQuality, FeatureWindow,
    MultimodalFeatureVector, RespirationFeatures,
};

#[allow(clippy::too_many_arguments)]
fn create_mock_feature_vector(
    start_sec: f64,
    hr: Option<f64>,
    sdnn: Option<f64>,
    tonic: Option<f64>,
    phasic: Option<f64>,
    scr_rate: Option<f64>,
    rsp_rate: Option<f64>,
    rsa: Option<f64>,
) -> MultimodalFeatureVector {
    let win = FeatureWindow {
        start_time_sec: start_sec,
        end_time_sec: start_sec + 60.0,
        duration_sec: 60.0,
    };

    let cardiac = CardiacFeatures {
        mean_hr_bpm: hr,
        median_hr_bpm: hr,
        sdnn_ms: sdnn,
        rmssd_ms: sdnn,
        pnn50: sdnn.map(|v| v * 0.5),
        rr_mean_ms: hr.map(|h| 60000.0 / h),
        rr_std_ms: sdnn,
        beat_count: if hr.is_some() { 60 } else { 0 },
    };

    let eda = EdaFeatures {
        mean_tonic_us: tonic,
        median_tonic_us: tonic,
        tonic_std_us: tonic.map(|v| v * 0.05),
        mean_phasic_us: phasic,
        phasic_std_us: phasic.map(|v| v * 0.1),
        scr_count: if scr_rate.is_some() { 5 } else { 0 },
        scr_rate_per_min: scr_rate,
        mean_scr_amplitude_us: phasic,
        median_scr_amplitude_us: phasic,
        mean_scr_rise_time_sec: Some(1.2),
    };

    let respiration = RespirationFeatures {
        mean_rate_bpm: rsp_rate,
        median_rate_bpm: rsp_rate,
        rate_std_bpm: rsp_rate.map(|v| v * 0.05),
        mean_cycle_duration_sec: rsp_rate.map(|r| 60.0 / r),
        cycle_count: if rsp_rate.is_some() { 15 } else { 0 },
        mean_amplitude: Some(1.0),
        amplitude_std: Some(0.05),
    };

    let coupling = CouplingFeatures {
        rsa_amplitude_bpm: rsa,
        rsa_amplitude_rr_sec: rsa.map(|v| v * 0.01),
        cardiac_respiratory_concentration: if rsa.is_some() { Some(0.8) } else { None },
        cardiac_respiratory_mean_phase: if rsa.is_some() { Some(1.5) } else { None },
        mean_pulse_delay_sec: Some(0.25),
        pulse_delay_std_sec: Some(0.01),
        scr_cardiac_association_count: 2,
    };

    let quality = FeatureQuality {
        coverage: 1.0,
        modality_coverage: FeatureCoverage {
            overall: 1.0,
            ecg: hr.map(|_| 1.0),
            ppg: Some(1.0),
            eda: tonic.map(|_| 1.0),
            rsp: rsp_rate.map(|_| 1.0),
        },
        cardiac_valid: hr.is_some(),
        eda_valid: tonic.is_some(),
        respiration_valid: rsp_rate.is_some(),
        coupling_valid: rsa.is_some(),
        usable_feature_count: 20,
        total_feature_count: 30,
        issues: Vec::new(),
    };

    MultimodalFeatureVector {
        window: win,
        cardiac,
        eda,
        respiration,
        coupling,
        quality,
    }
}

fn create_mock_baseline_series(n_samples: usize) -> Vec<MultimodalFeatureVector> {
    (0..n_samples)
        .map(|i| {
            let t = i as f64 * 30.0;
            create_mock_feature_vector(
                t,
                Some(70.0 + (i % 3) as f64),
                Some(40.0 + (i % 2) as f64),
                Some(2.0 + 0.1 * (i % 2) as f64),
                Some(0.5 + 0.05 * (i % 2) as f64),
                Some(4.0 + (i % 2) as f64),
                Some(15.0 + (i % 2) as f64),
                Some(5.0 + (i % 2) as f64),
            )
        })
        .collect()
}

// ============================================================================
// Group A — Baseline Fitting & Normalization Tests
// ============================================================================

#[test]
fn test_baseline_fitting_sufficient_and_insufficient() {
    let config = NormalizationConfig {
        min_baseline_samples: 5,
        ..NormalizationConfig::default()
    };

    // Empty series -> Err(InsufficientSamples)
    assert!(matches!(
        AutonomicBaseline::fit(&[], &config),
        Err(SignalError::InsufficientSamples { .. })
    ));

    // Short series (sample count < min_baseline_samples) -> Ok with is_valid == false
    let short_series = create_mock_baseline_series(3);
    let invalid_baseline = AutonomicBaseline::fit(&short_series, &config).unwrap();
    assert!(!invalid_baseline.hr_bpm_stats.is_valid);
    assert_eq!(invalid_baseline.hr_bpm_stats.sample_count, 3);

    // Sufficient samples -> Ok with is_valid == true
    let valid_series = create_mock_baseline_series(10);
    let baseline = AutonomicBaseline::fit(&valid_series, &config).unwrap();
    assert!(baseline.hr_bpm_stats.is_valid);
    assert!(baseline.sdnn_ms_stats.is_valid);
    assert!(baseline.eda_tonic_stats.is_valid);
    assert_eq!(baseline.hr_bpm_stats.sample_count, 10);
}

#[test]
fn test_normalization_bounded_transform_and_direction() {
    let config = NormalizationConfig::default();
    let series = create_mock_baseline_series(10);
    let baseline = AutonomicBaseline::fit(&series, &config).unwrap();

    // Positive direction: higher value -> positive score in [-1.0, 1.0]
    let score_high = AutonomicBaseline::normalize_feature(
        Some(100.0),
        &baseline.hr_bpm_stats,
        &config,
        FeatureDirection::Positive,
    )
    .unwrap();

    let score_low = AutonomicBaseline::normalize_feature(
        Some(40.0),
        &baseline.hr_bpm_stats,
        &config,
        FeatureDirection::Positive,
    )
    .unwrap();

    assert!(score_high > score_low);
    assert!((-1.0..=1.0).contains(&score_high));
    assert!((-1.0..=1.0).contains(&score_low));

    // Negative direction reverses score
    let score_neg_high = AutonomicBaseline::normalize_feature(
        Some(100.0),
        &baseline.hr_bpm_stats,
        &config,
        FeatureDirection::Negative,
    )
    .unwrap();
    assert_eq!(score_neg_high, -score_high);
}

// ============================================================================
// Group B — Composite State Estimation & Directional Invariants
// ============================================================================

#[test]
fn test_state_estimator_activation_and_regulation_directional_invariants() {
    let series = create_mock_baseline_series(10);
    let config = AutonomicEstimatorConfig::default();
    let baseline = AutonomicBaseline::fit(&series, &config.normalization).unwrap();
    let estimator = AutonomicEstimator::new(config);

    let baseline_fv = create_mock_feature_vector(
        0.0,
        Some(71.0),
        Some(40.0),
        Some(2.0),
        Some(0.5),
        Some(4.0),
        Some(15.0),
        Some(5.0),
    );
    let state_base = estimator.estimate(&baseline_fv, &baseline).unwrap();

    // High HR + High EDA -> Higher Activation Score
    let high_arousal_fv = create_mock_feature_vector(
        0.0,
        Some(110.0), // High HR
        Some(40.0),
        Some(5.0),  // High EDA Tonic
        Some(2.0),  // High EDA Phasic
        Some(12.0), // High SCR Rate
        Some(15.0),
        Some(5.0),
    );
    let state_arousal = estimator.estimate(&high_arousal_fv, &baseline).unwrap();

    assert!(
        state_arousal.activation_score.unwrap() > state_base.activation_score.unwrap(),
        "Higher HR & EDA must increase activation score"
    );

    // High SDNN + High RespHRV -> Higher Regulation Score
    let high_reg_fv = create_mock_feature_vector(
        0.0,
        Some(71.0),
        Some(80.0), // High SDNN
        Some(2.0),
        Some(0.5),
        Some(4.0),
        Some(15.0),
        Some(12.0), // High RespHRV
    );
    let state_reg = estimator.estimate(&high_reg_fv, &baseline).unwrap();

    assert!(
        state_reg.regulation_score.unwrap() > state_base.regulation_score.unwrap(),
        "Higher SDNN & RespHRV must increase regulation score"
    );
}

// ============================================================================
// Group C — RespHRV Respiratory Context Requirement (Buron 2026 / Gevonden 2025)
// ============================================================================

#[test]
fn test_resphrv_respiratory_context_requirement() {
    let series = create_mock_baseline_series(10);
    let config = AutonomicEstimatorConfig::default();
    let baseline = AutonomicBaseline::fit(&series, &config.normalization).unwrap();
    let estimator = AutonomicEstimator::new(config);

    // Feature vector with valid RSA BUT MISSING respiration signal
    let fv_no_rsp = create_mock_feature_vector(
        0.0,
        Some(70.0),
        Some(40.0),
        Some(2.0),
        Some(0.5),
        Some(4.0),
        None,      // Missing Respiration Rate
        Some(5.0), // RSA present
    );

    let state = estimator.estimate(&fv_no_rsp, &baseline).unwrap();

    // Per Buron & Menuet (2026) and Gevonden et al. (2025), RespHRV is excluded when respiration context is missing
    assert!(
        state.coupling.resphr_coupling_index.is_none(),
        "RespHRV coupling index must be None when direct respiration context is missing"
    );
    assert!(
        state.confidence.coupling.unwrap() < 0.8,
        "Coupling confidence must be degraded when respiration context is missing"
    );
}

// ============================================================================
// Group D — Missing Modality Degradation & Quality Gating
// ============================================================================

#[test]
fn test_missing_modality_degradation() {
    let series = create_mock_baseline_series(10);
    let config = AutonomicEstimatorConfig::default();
    let baseline = AutonomicBaseline::fit(&series, &config.normalization).unwrap();
    let estimator = AutonomicEstimator::new(config);

    // ECG-only feature vector
    let fv_ecg_only = create_mock_feature_vector(
        0.0,
        Some(70.0),
        Some(40.0),
        None, // No EDA
        None,
        None,
        None, // No Respiration
        None, // No Coupling
    );

    let state = estimator.estimate(&fv_ecg_only, &baseline).unwrap();

    assert!(state.cardiac.heart_rate_index.is_some());
    assert!(state.electrodermal.tonic_level_index.is_none());
    assert!(state.respiratory.rate_index.is_none());
    assert!(state.coupling.resphr_coupling_index.is_none());

    // Confidence decreases due to missing modalities
    assert!(state.confidence.cardiac.is_some());
    assert!(state.confidence.electrodermal.is_none());
    assert!(state.confidence.respiratory.is_none());
    assert!(state.confidence.overall.unwrap() < 1.0);
}

// ============================================================================
// Group E — Trajectory State Series & EMA Temporal Smoothing
// ============================================================================

#[test]
fn test_trajectory_series_and_ema_smoothing() {
    let baseline_series = create_mock_baseline_series(10);
    let config_raw = AutonomicEstimatorConfig {
        smoothing: None,
        ..AutonomicEstimatorConfig::default()
    };
    let baseline = AutonomicBaseline::fit(&baseline_series, &config_raw.normalization).unwrap();

    let estimator_raw = AutonomicEstimator::new(config_raw);

    // Create a step response input feature series
    let mut step_series = Vec::new();
    for i in 0..5 {
        step_series.push(create_mock_feature_vector(
            i as f64 * 30.0,
            Some(70.0),
            Some(40.0),
            Some(2.0),
            Some(0.5),
            Some(4.0),
            Some(15.0),
            Some(5.0),
        ));
    }
    for i in 5..10 {
        step_series.push(create_mock_feature_vector(
            i as f64 * 30.0,
            Some(110.0), // Step increase in HR
            Some(40.0),
            Some(5.0), // Step increase in EDA
            Some(2.0),
            Some(10.0),
            Some(15.0),
            Some(5.0),
        ));
    }

    let trajectory_raw = estimator_raw
        .estimate_series(&step_series, &baseline)
        .unwrap();
    assert_eq!(trajectory_raw.states.len(), 10);

    // Unsmoothed step change occurs instantly at step 5
    let raw_act_4 = trajectory_raw.states[4].activation_score.unwrap();
    let raw_act_5 = trajectory_raw.states[5].activation_score.unwrap();
    assert!(raw_act_5 > raw_act_4);

    // Apply EMA smoothing config
    let config_smooth = AutonomicEstimatorConfig {
        smoothing: Some(SmoothingConfig { alpha: 0.3 }),
        ..AutonomicEstimatorConfig::default()
    };
    let estimator_smooth = AutonomicEstimator::new(config_smooth);
    let trajectory_smooth = estimator_smooth
        .estimate_series(&step_series, &baseline)
        .unwrap();

    // Raw states remain unmutated in trajectory_smooth.states
    assert_eq!(trajectory_smooth.states, trajectory_raw.states);
    assert!(trajectory_smooth.smoothed_states.is_some());

    // Smoothed transition at step 5 is gradual compared to raw
    let smoothed_vec = trajectory_smooth.smoothed_states.as_ref().unwrap();
    let smooth_act_5 = smoothed_vec[5].activation_score.unwrap();
    assert!(smooth_act_5 < raw_act_5);
    assert!(smooth_act_5 > raw_act_4);
}

// ============================================================================
// Group F — Determinism and Finite Output Invariants
// ============================================================================

#[test]
fn test_determinism_and_finite_output_invariants() {
    let series = create_mock_baseline_series(10);
    let config = AutonomicEstimatorConfig::default();
    let baseline = AutonomicBaseline::fit(&series, &config.normalization).unwrap();
    let estimator = AutonomicEstimator::new(config);

    let fv = create_mock_feature_vector(
        0.0,
        Some(75.0),
        Some(45.0),
        Some(3.0),
        Some(0.8),
        Some(6.0),
        Some(16.0),
        Some(6.0),
    );

    let state1 = estimator.estimate(&fv, &baseline).unwrap();
    let state2 = estimator.estimate(&fv, &baseline).unwrap();

    // 100% Deterministic match
    assert_eq!(state1, state2);

    // Finite output safety
    if let Some(act) = state1.activation_score {
        assert!(act.is_finite());
        assert!((-1.0..=1.0).contains(&act));
    }
    if let Some(reg) = state1.regulation_score {
        assert!(reg.is_finite());
        assert!((-1.0..=1.0).contains(&reg));
    }
    if let Some(conf) = state1.confidence.overall {
        assert!(conf.is_finite());
        assert!((0.0..=1.0).contains(&conf));
    }
}

// ============================================================================
// Group G — Task 8.1 Targeted Integrity Tests
// ============================================================================

#[test]
fn test_zero_variance_exact_baseline_returns_zero() {
    let config = NormalizationConfig {
        min_scale: 1e-6,
        ..NormalizationConfig::default()
    };
    let stats = BaselineFeatureStats {
        mean: Some(70.0),
        std: Some(0.0),
        median: Some(70.0),
        mad: Some(0.0),
        sample_count: 10,
        is_valid: true,
    };
    let result = AutonomicBaseline::normalize_feature(
        Some(70.0),
        &stats,
        &config,
        FeatureDirection::Positive,
    );
    assert_eq!(result, Some(0.0));
}

#[test]
fn test_zero_variance_different_value_returns_none() {
    let config = NormalizationConfig {
        min_scale: 1e-6,
        ..NormalizationConfig::default()
    };
    let stats = BaselineFeatureStats {
        mean: Some(70.0),
        std: Some(0.0),
        median: Some(70.0),
        mad: Some(0.0),
        sample_count: 10,
        is_valid: true,
    };
    let result = AutonomicBaseline::normalize_feature(
        Some(75.0),
        &stats,
        &config,
        FeatureDirection::Positive,
    );
    assert_eq!(result, None);
}

#[test]
fn test_invalid_min_scale_rejected() {
    let config_zero = NormalizationConfig {
        min_scale: 0.0,
        ..NormalizationConfig::default()
    };
    assert!(config_zero.validate().is_err());

    let config_neg = NormalizationConfig {
        min_scale: -1e-6,
        ..NormalizationConfig::default()
    };
    assert!(config_neg.validate().is_err());

    let config_nan = NormalizationConfig {
        min_scale: f64::NAN,
        ..NormalizationConfig::default()
    };
    assert!(config_nan.validate().is_err());
}

#[test]
fn test_recovery_both_features_weighted() {
    let mut config = AutonomicEstimatorConfig::default();
    config.recovery.variability_weight = 2.0;
    config.recovery.heart_rate_weight = 1.0;
    let estimator = AutonomicEstimator::new(config);

    let series = create_mock_baseline_series(10);
    let baseline = AutonomicBaseline::fit(&series, &estimator.config.normalization).unwrap();

    let fv = create_mock_feature_vector(
        0.0,
        Some(70.0),
        Some(40.0),
        Some(2.0),
        Some(0.5),
        Some(4.0),
        Some(15.0),
        Some(5.0),
    );

    let state = estimator.estimate(&fv, &baseline).unwrap();
    let var_idx = state.cardiac.variability_index.unwrap();
    let hr_idx = state.cardiac.heart_rate_index.unwrap();
    let expected = (2.0 * var_idx - 1.0 * hr_idx) / 3.0;

    let actual = state.cardiac.recovery_evidence.unwrap();
    assert!(
        (actual - expected).abs() < 1e-10,
        "Recovery must equal (2*var - 1*hr)/3, got {}, expected {}",
        actual,
        expected
    );
}

#[test]
fn test_recovery_directionality() {
    let series = create_mock_baseline_series(10);
    let config = AutonomicEstimatorConfig::default();
    let baseline = AutonomicBaseline::fit(&series, &config.normalization).unwrap();
    let estimator = AutonomicEstimator::new(config);

    let fv_recovered = create_mock_feature_vector(
        0.0,
        Some(60.0),
        Some(80.0),
        Some(2.0),
        Some(0.5),
        Some(4.0),
        Some(15.0),
        Some(5.0),
    );
    let state_rec = estimator.estimate(&fv_recovered, &baseline).unwrap();

    let fv_stressed = create_mock_feature_vector(
        0.0,
        Some(100.0),
        Some(20.0),
        Some(2.0),
        Some(0.5),
        Some(4.0),
        Some(15.0),
        Some(5.0),
    );
    let state_str = estimator.estimate(&fv_stressed, &baseline).unwrap();

    assert!(
        state_rec.cardiac.recovery_evidence.unwrap() > state_str.cardiac.recovery_evidence.unwrap(),
        "Higher variability combined with lower HR must produce higher recovery evidence"
    );
}

#[test]
fn test_resphrv_missing_respiration_is_unavailable() {
    let series = create_mock_baseline_series(10);
    let config = AutonomicEstimatorConfig::default();
    let baseline = AutonomicBaseline::fit(&series, &config.normalization).unwrap();
    let estimator = AutonomicEstimator::new(config);

    let fv_no_rsp = create_mock_feature_vector(
        0.0,
        Some(70.0),
        Some(40.0),
        Some(2.0),
        Some(0.5),
        Some(4.0),
        None,
        Some(5.0),
    );

    let state = estimator.estimate(&fv_no_rsp, &baseline).unwrap();
    assert_eq!(state.coupling.resphr_coupling_index, None);
}

#[test]
fn test_confidence_excludes_unavailable_resphrv() {
    let series = create_mock_baseline_series(10);
    let config = AutonomicEstimatorConfig::default();
    let baseline = AutonomicBaseline::fit(&series, &config.normalization).unwrap();
    let estimator = AutonomicEstimator::new(config);

    let fv_full = create_mock_feature_vector(
        0.0,
        Some(70.0),
        Some(40.0),
        Some(2.0),
        Some(0.5),
        Some(4.0),
        Some(15.0),
        Some(5.0),
    );
    let state_full = estimator.estimate(&fv_full, &baseline).unwrap();

    let fv_no_rsp = create_mock_feature_vector(
        0.0,
        Some(70.0),
        Some(40.0),
        Some(2.0),
        Some(0.5),
        Some(4.0),
        None,
        Some(5.0),
    );
    let state_no_rsp = estimator.estimate(&fv_no_rsp, &baseline).unwrap();

    assert!(
        state_no_rsp.confidence.coupling.unwrap() < state_full.confidence.coupling.unwrap(),
        "Coupling confidence must decrease when RespHRV evidence is excluded"
    );
}

#[test]
fn test_respiratory_regularity_directionality() {
    let series = create_mock_baseline_series(10);
    let config = AutonomicEstimatorConfig::default();
    let baseline = AutonomicBaseline::fit(&series, &config.normalization).unwrap();
    let estimator = AutonomicEstimator::new(config);

    let mut fv_regular = create_mock_feature_vector(
        0.0,
        Some(70.0),
        Some(40.0),
        Some(2.0),
        Some(0.5),
        Some(4.0),
        Some(15.0),
        Some(5.0),
    );
    fv_regular.respiration.rate_std_bpm = Some(0.1);

    let mut fv_irregular = fv_regular.clone();
    fv_irregular.respiration.rate_std_bpm = Some(3.0);

    let state_reg = estimator.estimate(&fv_regular, &baseline).unwrap();
    let state_irreg = estimator.estimate(&fv_irregular, &baseline).unwrap();

    assert!(
        state_reg.respiratory.regularity_index.unwrap()
            > state_irreg.respiratory.regularity_index.unwrap(),
        "Lower rate_std_bpm must yield higher regularity_index"
    );
}

#[test]
fn test_population_standard_deviation_hand_calculated() {
    let config = NormalizationConfig::default();
    let samples = vec![10.0, 20.0, 30.0];
    let stats = BaselineFeatureStats::from_samples(&samples, &config);

    assert_eq!(stats.mean, Some(20.0));
    let expected_pop_std = (200.0f64 / 3.0f64).sqrt();
    let actual_std = stats.std.unwrap();
    assert!(
        (actual_std - expected_pop_std).abs() < 1e-10,
        "Baseline stats must use population SD (N=3), got {}, expected {}",
        actual_std,
        expected_pop_std
    );
    assert_ne!(actual_std, 10.0, "Must NOT use sample SD (N-1=2)");
}

#[test]
fn test_temporal_validation_test_suite() {
    let series = create_mock_baseline_series(10);
    let config = AutonomicEstimatorConfig::default();
    let baseline = AutonomicBaseline::fit(&series, &config.normalization).unwrap();
    let estimator = AutonomicEstimator::new(config);

    let mut bad_series = vec![
        create_mock_feature_vector(
            0.0,
            Some(70.0),
            Some(40.0),
            Some(2.0),
            Some(0.5),
            Some(4.0),
            Some(15.0),
            Some(5.0),
        ),
        create_mock_feature_vector(
            60.0,
            Some(70.0),
            Some(40.0),
            Some(2.0),
            Some(0.5),
            Some(4.0),
            Some(15.0),
            Some(5.0),
        ),
        create_mock_feature_vector(
            30.0,
            Some(70.0),
            Some(40.0),
            Some(2.0),
            Some(0.5),
            Some(4.0),
            Some(15.0),
            Some(5.0),
        ),
    ];
    assert!(estimator.estimate_series(&bad_series, &baseline).is_err());

    bad_series[2] = create_mock_feature_vector(
        f64::NAN,
        Some(70.0),
        Some(40.0),
        Some(2.0),
        Some(0.5),
        Some(4.0),
        Some(15.0),
        Some(5.0),
    );
    assert!(estimator.estimate_series(&bad_series, &baseline).is_err());

    let inconsistent_series = vec![
        create_mock_feature_vector(
            0.0,
            Some(70.0),
            Some(40.0),
            Some(2.0),
            Some(0.5),
            Some(4.0),
            Some(15.0),
            Some(5.0),
        ),
        create_mock_feature_vector(
            30.0,
            Some(70.0),
            Some(40.0),
            Some(2.0),
            Some(0.5),
            Some(4.0),
            Some(15.0),
            Some(5.0),
        ),
        create_mock_feature_vector(
            90.0,
            Some(70.0),
            Some(40.0),
            Some(2.0),
            Some(0.5),
            Some(4.0),
            Some(15.0),
            Some(5.0),
        ),
    ];
    assert!(
        estimator
            .estimate_series(&inconsistent_series, &baseline)
            .is_err()
    );
}

#[test]
fn test_separate_raw_vs_smoothed_trajectory_immutability() {
    let baseline_series = create_mock_baseline_series(10);
    let config = AutonomicEstimatorConfig {
        smoothing: Some(SmoothingConfig { alpha: 0.3 }),
        ..AutonomicEstimatorConfig::default()
    };
    let baseline = AutonomicBaseline::fit(&baseline_series, &config.normalization).unwrap();
    let estimator = AutonomicEstimator::new(config);

    let step_series = vec![
        create_mock_feature_vector(
            0.0,
            Some(70.0),
            Some(40.0),
            Some(2.0),
            Some(0.5),
            Some(4.0),
            Some(15.0),
            Some(5.0),
        ),
        create_mock_feature_vector(
            30.0,
            Some(110.0),
            Some(40.0),
            Some(5.0),
            Some(2.0),
            Some(10.0),
            Some(15.0),
            Some(5.0),
        ),
    ];

    let series_res = estimator.estimate_series(&step_series, &baseline).unwrap();

    assert_eq!(series_res.states.len(), 2);
    let smoothed = series_res.smoothed_states.unwrap();
    assert_eq!(smoothed.len(), 2);

    assert_ne!(
        series_res.states[1].activation_score, smoothed[1].activation_score,
        "Smoothed state must differ from raw state after step change"
    );
}

#[test]
fn test_recovery_variability_only() {
    let series = create_mock_baseline_series(10);
    let config = AutonomicEstimatorConfig::default();
    let baseline = AutonomicBaseline::fit(&series, &config.normalization).unwrap();
    let estimator = AutonomicEstimator::new(config);

    let mut fv_var_only = create_mock_feature_vector(
        0.0,
        Some(70.0),
        Some(40.0),
        Some(2.0),
        Some(0.5),
        Some(4.0),
        Some(15.0),
        Some(5.0),
    );
    fv_var_only.cardiac.mean_hr_bpm = None;
    fv_var_only.quality.cardiac_valid = true;

    let state = estimator.estimate(&fv_var_only, &baseline).unwrap();
    assert!(state.cardiac.variability_index.is_some());
    assert!(state.cardiac.heart_rate_index.is_none());
    assert_eq!(
        state.cardiac.recovery_evidence, state.cardiac.variability_index,
        "Variability-only recovery must equal variability_index"
    );
}

#[test]
fn test_recovery_heart_rate_only() {
    let series = create_mock_baseline_series(10);
    let config = AutonomicEstimatorConfig::default();
    let baseline = AutonomicBaseline::fit(&series, &config.normalization).unwrap();
    let estimator = AutonomicEstimator::new(config);

    let mut fv_hr_only = create_mock_feature_vector(
        0.0,
        Some(70.0),
        None,
        Some(2.0),
        Some(0.5),
        Some(4.0),
        Some(15.0),
        Some(5.0),
    );
    fv_hr_only.cardiac.rmssd_ms = None;
    fv_hr_only.cardiac.pnn50 = None;

    let state = estimator.estimate(&fv_hr_only, &baseline).unwrap();
    assert!(state.cardiac.variability_index.is_none());
    assert!(state.cardiac.heart_rate_index.is_some());
    let hr_idx = state.cardiac.heart_rate_index.unwrap();
    assert_eq!(
        state.cardiac.recovery_evidence,
        Some(-hr_idx),
        "Heart-rate-only recovery must equal -heart_rate_index"
    );
}

#[test]
fn test_recovery_zero_variability_weight() {
    let mut config = AutonomicEstimatorConfig::default();
    config.recovery.variability_weight = 0.0;
    config.recovery.heart_rate_weight = 1.0;
    let estimator = AutonomicEstimator::new(config);

    let series = create_mock_baseline_series(10);
    let baseline = AutonomicBaseline::fit(&series, &estimator.config.normalization).unwrap();

    let fv = create_mock_feature_vector(
        0.0,
        Some(70.0),
        Some(40.0),
        Some(2.0),
        Some(0.5),
        Some(4.0),
        Some(15.0),
        Some(5.0),
    );

    let state = estimator.estimate(&fv, &baseline).unwrap();
    let hr_idx = state.cardiac.heart_rate_index.unwrap();
    assert_eq!(
        state.cardiac.recovery_evidence,
        Some(-hr_idx),
        "When w_var = 0, recovery must equal -heart_rate_index"
    );
}

#[test]
fn test_recovery_zero_heart_rate_weight() {
    let mut config = AutonomicEstimatorConfig::default();
    config.recovery.variability_weight = 1.0;
    config.recovery.heart_rate_weight = 0.0;
    let estimator = AutonomicEstimator::new(config);

    let series = create_mock_baseline_series(10);
    let baseline = AutonomicBaseline::fit(&series, &estimator.config.normalization).unwrap();

    let fv = create_mock_feature_vector(
        0.0,
        Some(70.0),
        Some(40.0),
        Some(2.0),
        Some(0.5),
        Some(4.0),
        Some(15.0),
        Some(5.0),
    );

    let state = estimator.estimate(&fv, &baseline).unwrap();
    let var_idx = state.cardiac.variability_index.unwrap();
    assert_eq!(
        state.cardiac.recovery_evidence,
        Some(var_idx),
        "When w_hr = 0, recovery must equal variability_index"
    );
}

#[test]
fn test_recovery_unavailable_zero_weight_returns_none() {
    let series = create_mock_baseline_series(10);

    let mut config_a = AutonomicEstimatorConfig::default();
    config_a.recovery.variability_weight = 0.0;
    config_a.recovery.heart_rate_weight = 1.0;
    let baseline_a = AutonomicBaseline::fit(&series, &config_a.normalization).unwrap();
    let estimator_a = AutonomicEstimator::new(config_a);

    let mut fv_var_only = create_mock_feature_vector(
        0.0,
        Some(70.0),
        Some(40.0),
        Some(2.0),
        Some(0.5),
        Some(4.0),
        Some(15.0),
        Some(5.0),
    );
    fv_var_only.cardiac.mean_hr_bpm = None;
    fv_var_only.quality.cardiac_valid = true;
    let state_a = estimator_a.estimate(&fv_var_only, &baseline_a).unwrap();
    assert_eq!(
        state_a.cardiac.recovery_evidence, None,
        "Variability present but w_var = 0 with HR unavailable must yield recovery_evidence = None"
    );

    let mut config_b = AutonomicEstimatorConfig::default();
    config_b.recovery.variability_weight = 1.0;
    config_b.recovery.heart_rate_weight = 0.0;
    let baseline_b = AutonomicBaseline::fit(&series, &config_b.normalization).unwrap();
    let estimator_b = AutonomicEstimator::new(config_b);

    let mut fv_hr_only = create_mock_feature_vector(
        0.0,
        Some(70.0),
        None,
        Some(2.0),
        Some(0.5),
        Some(4.0),
        Some(15.0),
        Some(5.0),
    );
    fv_hr_only.cardiac.rmssd_ms = None;
    fv_hr_only.cardiac.pnn50 = None;

    let state_b = estimator_b.estimate(&fv_hr_only, &baseline_b).unwrap();
    assert_eq!(
        state_b.cardiac.recovery_evidence, None,
        "HR present but w_hr = 0 with variability unavailable must yield recovery_evidence = None"
    );
}

#[test]
fn test_recovery_both_weights_zero_rejected() {
    let mut config = AutonomicEstimatorConfig::default();
    config.recovery.variability_weight = 0.0;
    config.recovery.heart_rate_weight = 0.0;
    assert!(config.validate().is_err());
}
