use lamina::ecg::ecg_findpeaks;
use lamina::eda::{
    EdaDecompositionConfig, EdaPeakDetectionConfig, ScrEvent, eda_decompose, eda_findpeaks_events,
};
use lamina::error::SignalError;
use lamina::multimodal::{
    cardiac_respiratory_phase, cardiorespiratory_phase_coupling, ecg_ppg_timing,
    eda_cardiorespiratory_association, evaluate_ecg_quality, evaluate_rsp_quality,
    multimodal_quality, respiratory_phase_at_time, rsa, sample_to_time,
};
use lamina::ppg::ppg_findpeaks;
use lamina::rsp::{RespirationCycle, rsp_clean, rsp_cycles};
use ndarray::Array1;
use std::f64::consts::PI;

// Helper to convert boolean mask to index list
fn mask_to_indices(mask: &Array1<bool>) -> Vec<usize> {
    mask.iter()
        .enumerate()
        .filter_map(|(i, &p)| if p { Some(i) } else { None })
        .collect()
}

// ============================================================================
// Group A — Synchronization Tests
// ============================================================================

#[test]
fn test_sample_to_time_same_fs() {
    let index = 100;
    let fs = 100.0;
    let offset = 0.0;

    let t = sample_to_time(index, fs, offset).unwrap();
    assert_eq!(t, 1.0);
}

#[test]
fn test_sample_to_time_different_fs_and_offsets() {
    // ECG @ 500 Hz, offset 0.0s -> index 250 => 0.50s
    let t_ecg = sample_to_time(250, 500.0, 0.0).unwrap();
    assert_eq!(t_ecg, 0.50);

    // PPG @ 100 Hz, offset 0.035s -> index 50 => 0.535s
    let t_ppg = sample_to_time(50, 100.0, 0.035).unwrap();
    assert_eq!(t_ppg, 0.535);

    // RSP @ 32 Hz, offset 0.120s -> index 16 => 0.620s
    let t_rsp = sample_to_time(16, 32.0, 0.120).unwrap();
    assert_eq!(t_rsp, 0.620);
}

#[test]
fn test_sample_to_time_input_validation() {
    assert!(matches!(
        sample_to_time(10, -100.0, 0.0),
        Err(SignalError::InvalidSamplingRate(_))
    ));
    assert!(matches!(
        sample_to_time(10, 0.0, 0.0),
        Err(SignalError::InvalidSamplingRate(_))
    ));
    assert!(matches!(
        sample_to_time(10, f64::NAN, 0.0),
        Err(SignalError::InvalidSamplingRate(_))
    ));
    assert!(matches!(
        sample_to_time(10, 100.0, f64::NAN),
        Err(SignalError::NonFiniteInput)
    ));
}

// ============================================================================
// Group B — Respiratory Phase Tests
// ============================================================================

#[test]
fn test_respiratory_phase_mapping_inspiration_and_expiration() {
    let fs = 100.0;
    // Cycle: insp1 at 100 (1.0s), exp at 200 (2.0s), insp2 at 300 (3.0s)
    let cycle = RespirationCycle {
        inspiration_index: 100,
        expiration_index: 200,
        next_inspiration_index: 300,
        duration_sec: 2.0,
        respiratory_rate_bpm: 30.0,
        amplitude: 1.0,
    };
    let cycles = vec![cycle];

    // Inspiration peak (1.0s) -> phase 0.0
    let p_start = respiratory_phase_at_time(&cycles, 1.0, fs, 0.0).unwrap();
    assert!((p_start - 0.0).abs() < 1e-10);

    // Mid-inspiration (1.5s) -> phase PI / 2
    let p_mid_insp = respiratory_phase_at_time(&cycles, 1.5, fs, 0.0).unwrap();
    assert!((p_mid_insp - PI / 2.0).abs() < 1e-10);

    // Expiration trough (2.0s) -> phase PI
    let p_exp = respiratory_phase_at_time(&cycles, 2.0, fs, 0.0).unwrap();
    assert!((p_exp - PI).abs() < 1e-10);

    // Mid-expiration (2.5s) -> phase 1.5 * PI
    let p_mid_exp = respiratory_phase_at_time(&cycles, 2.5, fs, 0.0).unwrap();
    assert!((p_mid_exp - 1.5 * PI).abs() < 1e-10);

    // End cycle (3.0s) -> phase 0.0 (normalized from 2*PI)
    let p_end = respiratory_phase_at_time(&cycles, 3.0, fs, 0.0).unwrap();
    assert!((p_end - 0.0).abs() < 1e-10);
}

#[test]
fn test_respiratory_phase_out_of_bounds_returns_none() {
    let cycle = RespirationCycle {
        inspiration_index: 100,
        expiration_index: 200,
        next_inspiration_index: 300,
        duration_sec: 2.0,
        respiratory_rate_bpm: 30.0,
        amplitude: 1.0,
    };
    let cycles = vec![cycle];

    assert!(respiratory_phase_at_time(&cycles, 0.5, 100.0, 0.0).is_none());
    assert!(respiratory_phase_at_time(&cycles, 3.5, 100.0, 0.0).is_none());
}

// ============================================================================
// Group C — RSA Tests
// ============================================================================

#[test]
fn test_rsa_synthetic_modulation() {
    let ecg_fs = 100.0;
    let rsp_fs = 100.0;

    // Respiration cycle: insp 1.0s (idx 100), exp 3.0s (idx 300), next insp 5.0s (idx 500)
    let cycle = RespirationCycle {
        inspiration_index: 100,
        expiration_index: 300,
        next_inspiration_index: 500,
        duration_sec: 4.0,
        respiratory_rate_bpm: 15.0,
        amplitude: 1.0,
    };
    let cycles = vec![cycle];

    // Cardiac beats:
    // Inspiration (1.0s to 3.0s): fast HR (80 BPM -> RR = 0.75s)
    // Expiration (3.0s to 5.0s): slow HR (60 BPM -> RR = 1.00s)
    let r_peaks = vec![
        100, // 1.0s (insp) -> RR to next = 0.75s (80 BPM)
        175, // 1.75s (insp) -> RR to next = 0.75s (80 BPM)
        250, // 2.50s (insp) -> RR to next = 0.75s (80 BPM)
        325, // 3.25s (exp) -> RR to next = 1.00s (60 BPM)
        425, // 4.25s (exp) -> RR to next = 1.00s (60 BPM)
        525, // 5.25s (outside)
    ];

    let result = rsa(&r_peaks, ecg_fs, 0.0, &cycles, rsp_fs, 0.0).unwrap();

    assert_eq!(result.valid_cycles, 1);
    assert!((result.amplitude_bpm - 20.0).abs() < 1e-5);
    assert!((result.amplitude_rr_sec - 0.25).abs() < 1e-5);
}

#[test]
fn test_rsa_insufficient_events() {
    let cycle = RespirationCycle {
        inspiration_index: 100,
        expiration_index: 300,
        next_inspiration_index: 500,
        duration_sec: 4.0,
        respiratory_rate_bpm: 15.0,
        amplitude: 1.0,
    };
    let r_peaks = vec![150]; // Only 1 peak

    let res = rsa(&r_peaks, 100.0, 0.0, &[cycle], 100.0, 0.0);
    assert!(matches!(res, Err(SignalError::InsufficientPeaks { .. })));
}

// ============================================================================
// Group D — Circular Phase Coupling Tests
// ============================================================================

#[test]
fn test_phase_coupling_concentrated_phases() {
    // All phases identical at PI / 4
    let phases = vec![PI / 4.0, PI / 4.0, PI / 4.0, PI / 4.0];
    let res = cardiorespiratory_phase_coupling(&phases).unwrap();

    assert!((res.concentration - 1.0).abs() < 1e-10);
    assert!((res.mean_phase - PI / 4.0).abs() < 1e-10);
    assert_eq!(res.sample_count, 4);
}

#[test]
fn test_phase_coupling_uniform_phases() {
    // 4 symmetric phases around unit circle
    let phases = vec![0.0, PI / 2.0, PI, 1.5 * PI];
    let res = cardiorespiratory_phase_coupling(&phases).unwrap();

    assert!(res.concentration < 1e-10);
    assert_eq!(res.sample_count, 4);
}

#[test]
fn test_phase_coupling_rotation_invariance() {
    let original_phases = vec![0.1, 0.2, 0.3, 0.4];
    let shift = 1.0;
    let shifted_phases: Vec<f64> = original_phases.iter().map(|p| p + shift).collect();

    let res_orig = cardiorespiratory_phase_coupling(&original_phases).unwrap();
    let res_shift = cardiorespiratory_phase_coupling(&shifted_phases).unwrap();

    // Resultant vector length R must remain invariant under constant rotation
    assert!((res_orig.concentration - res_shift.concentration).abs() < 1e-10);
    // Mean phase shifts by shift
    let diff = (res_shift.mean_phase - res_orig.mean_phase - shift).abs();
    assert!(diff < 1e-10 || (diff - 2.0 * PI).abs() < 1e-10);
}

// ============================================================================
// Group E — ECG-PPG Pulse Timing Tests
// ============================================================================

#[test]
fn test_ecg_ppg_timing_fixed_delay() {
    let ecg_fs = 100.0;
    let ppg_fs = 100.0;

    // ECG peaks every 1.0s (indices 100, 200, 300)
    let ecg_peaks = vec![100, 200, 300];
    // PPG peaks 0.20s later (indices 120, 220, 320)
    let ppg_peaks = vec![120, 220, 320];

    let matches = ecg_ppg_timing(&ecg_peaks, ecg_fs, 0.0, &ppg_peaks, ppg_fs, 0.0).unwrap();

    assert_eq!(matches.len(), 3);
    for m in &matches {
        assert!((m.pulse_delay_sec - 0.20).abs() < 1e-10);
    }
}

#[test]
fn test_ecg_ppg_timing_one_to_one_matching_and_rejection() {
    let ecg_fs = 100.0;
    let ppg_fs = 100.0;

    let ecg_peaks = vec![100, 200];
    // Multiple candidate PPG peaks: 105 (0.05s - too fast < 0.10s), 120 (0.20s - valid match), 125 (0.25s - extra beat, must be rejected)
    let ppg_peaks = vec![105, 120, 125, 220];

    let matches = ecg_ppg_timing(&ecg_peaks, ecg_fs, 0.0, &ppg_peaks, ppg_fs, 0.0).unwrap();

    assert_eq!(matches.len(), 2);
    assert_eq!(matches[0].ecg_peak_index, 100);
    assert_eq!(matches[0].ppg_peak_index, 120);
    assert_eq!(matches[1].ecg_peak_index, 200);
    assert_eq!(matches[1].ppg_peak_index, 220);
}

#[test]
fn test_ecg_ppg_timing_different_fs_and_offsets() {
    // ECG @ 500 Hz, offset 0.0s -> index 500 = 1.0s
    let ecg_peaks = vec![500];
    // PPG @ 100 Hz, offset 0.05s -> index 115 => 0.05 + 1.15 = 1.20s (pulse delay = 0.20s)
    let ppg_peaks = vec![115];

    let matches = ecg_ppg_timing(&ecg_peaks, 500.0, 0.0, &ppg_peaks, 100.0, 0.05).unwrap();

    assert_eq!(matches.len(), 1);
    assert!((matches[0].pulse_delay_sec - 0.20).abs() < 1e-10);
}

// ============================================================================
// Group F — EDA Associations
// ============================================================================

#[test]
fn test_eda_cardiorespiratory_association() {
    let scr_event = ScrEvent {
        onset_index: 50,
        peak_index: 100, // 1.0s @ 100 Hz
        amplitude: 2.5,
        rise_time_sec: 0.5,
    };
    let scr_events = vec![scr_event];

    let r_peaks = vec![80]; // 0.8s @ 100 Hz -> nearest ECG beat 0.2s before SCR
    let cycle = RespirationCycle {
        inspiration_index: 50,
        expiration_index: 100,
        next_inspiration_index: 150,
        duration_sec: 1.0,
        respiratory_rate_bpm: 60.0,
        amplitude: 1.0,
    };

    let assocs = eda_cardiorespiratory_association(
        &scr_events,
        100.0,
        0.0,
        &r_peaks,
        100.0,
        0.0,
        &[cycle],
        100.0,
        0.0,
    )
    .unwrap();

    assert_eq!(assocs.len(), 1);
    assert_eq!(assocs[0].scr_peak_index, 100);
    assert!((assocs[0].scr_peak_time_sec - 1.0).abs() < 1e-10);
    assert!((assocs[0].nearest_r_peak_time_sec.unwrap() - 0.8).abs() < 1e-10);
    assert!((assocs[0].cardiac_delay_sec.unwrap() - 0.2).abs() < 1e-10);
    assert!((assocs[0].respiratory_phase_rad.unwrap() - PI).abs() < 1e-10);
}

// ============================================================================
// Group G — Sampling-Rate Invariance
// ============================================================================

#[test]
fn test_multi_sampling_rate_timing_invariance() {
    let sampling_rates = [32.0, 64.0, 100.0, 250.0, 500.0, 1000.0];

    for &fs in &sampling_rates {
        let ecg_idx = (1.0f64 * fs).round() as usize;
        let ppg_idx = (1.2f64 * fs).round() as usize;

        let matches = ecg_ppg_timing(&[ecg_idx], fs, 0.0, &[ppg_idx], fs, 0.0).unwrap();
        assert_eq!(matches.len(), 1);
        assert!((matches[0].pulse_delay_sec - 0.20).abs() < 1.0 / fs);
    }
}

// ============================================================================
// Group H — Quality & Robustness Tests
// ============================================================================

#[test]
fn test_multimodal_quality_assessment() {
    let r_peaks = vec![100, 200, 300, 400, 500]; // 60 BPM @ 100 Hz over 5 seconds
    let ecg_q = evaluate_ecg_quality(&r_peaks, 100.0, 5.0);
    assert_eq!(ecg_q.score, 1.0);
    assert!(ecg_q.valid);

    let rsp_q = evaluate_rsp_quality(2, 10.0); // 12 BPM over 10 seconds
    assert_eq!(rsp_q.score, 1.0);
    assert!(rsp_q.valid);

    let mm_q = multimodal_quality(Some(ecg_q), None, None, Some(rsp_q)).unwrap();
    assert_eq!(mm_q.overall_quality, 1.0);
}

#[test]
fn test_multimodal_edge_cases_empty_and_non_finite() {
    assert!(matches!(
        cardiorespiratory_phase_coupling(&[]),
        Err(SignalError::EmptySignal)
    ));
    assert!(matches!(
        cardiorespiratory_phase_coupling(&[f64::NAN]),
        Err(SignalError::NonFiniteInput)
    ));

    let empty_timing = ecg_ppg_timing(&[], 100.0, 0.0, &[100], 100.0, 0.0).unwrap();
    assert!(empty_timing.is_empty());
}

// ============================================================================
// Group I — End-to-End Integration Pipeline Test
// ============================================================================

#[test]
fn test_multimodal_end_to_end_pipeline() {
    let fs = 100.0;
    let n_samples = 1000;

    // Synthetic ECG with peaks
    let mut ecg_signal = Array1::<f64>::zeros(n_samples);
    for i in (50..n_samples).step_by(100) {
        ecg_signal[i] = 4.0;
    }
    let r_peaks_mask = ecg_findpeaks(&ecg_signal, fs).unwrap();
    let r_peaks = mask_to_indices(&r_peaks_mask);
    assert!(!r_peaks.is_empty());

    // Synthetic RSP with clean sinus cycles
    let mut rsp_signal = Array1::<f64>::zeros(n_samples);
    for i in 0..n_samples {
        let t = i as f64 / fs;
        rsp_signal[i] = (2.0 * PI * 0.25 * t).sin();
    }
    let cleaned_rsp = rsp_clean(&rsp_signal, fs).unwrap();
    let rsp_cycle_list = rsp_cycles(&cleaned_rsp, fs).unwrap();

    // Synthetic PPG with peaks delayed by 0.2s (20 samples) relative to ECG
    let mut ppg_signal = Array1::<f64>::zeros(n_samples);
    for &r_idx in &r_peaks {
        let p_idx = r_idx + 20;
        if p_idx < n_samples {
            ppg_signal[p_idx] = 2.0;
        }
    }
    let ppg_peaks_mask = ppg_findpeaks(&ppg_signal, fs).unwrap();
    let ppg_peaks_list = mask_to_indices(&ppg_peaks_mask);

    // Synthetic EDA
    let mut eda_signal = Array1::<f64>::zeros(n_samples);
    for i in 0..n_samples {
        eda_signal[i] = 2.0 + 0.001 * i as f64;
    }
    eda_signal[300] += 1.0;
    let decomp = eda_decompose(&eda_signal, fs, &EdaDecompositionConfig::default()).unwrap();
    let scr_events =
        eda_findpeaks_events(&decomp.phasic, fs, &EdaPeakDetectionConfig::default()).unwrap();

    // Execute Multimodal Layer Pipeline
    let cardiac_phase_events =
        cardiac_respiratory_phase(&r_peaks, fs, 0.0, &rsp_cycle_list, fs, 0.0).unwrap();
    assert!(!cardiac_phase_events.is_empty());

    let phases: Vec<f64> = cardiac_phase_events
        .iter()
        .map(|e| e.respiratory_phase)
        .collect();
    let coupling_res = cardiorespiratory_phase_coupling(&phases).unwrap();
    assert!(coupling_res.concentration >= 0.0 && coupling_res.concentration <= 1.0);

    let timing_res = ecg_ppg_timing(&r_peaks, fs, 0.0, &ppg_peaks_list, fs, 0.0).unwrap();
    assert!(!timing_res.is_empty());

    let eda_assocs = eda_cardiorespiratory_association(
        &scr_events,
        fs,
        0.0,
        &r_peaks,
        fs,
        0.0,
        &rsp_cycle_list,
        fs,
        0.0,
    )
    .unwrap();
    assert_eq!(eda_assocs.len(), scr_events.len());
}
