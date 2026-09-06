use lamina::autonomic::{
    AutonomicBaseline, AutonomicEstimator, AutonomicEstimatorConfig, FeatureDirection,
    NormalizationConfig, SmoothingConfig,
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

    // Smoothed transition at step 5 is gradual compared to raw
    let smooth_act_5 = trajectory_smooth.states[5].activation_score.unwrap();
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
