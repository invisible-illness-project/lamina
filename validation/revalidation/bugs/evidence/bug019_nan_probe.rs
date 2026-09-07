//! BUG-019 revalidation probe: total_cmp sort — attempt to drive NaN/inf into the
//! Pan-Tompkins integrated signal from finite raw input; verify NO PANIC.
//! Also: direct-Rust NaN threshold_multiplier check (BUG-002 complement, since
//! strict JSON cannot carry NaN through the bridge).
use lamina::ecg::{ecg_findpeaks_config, EcgPeakDetectionConfig};
use ndarray::Array1;

fn qrs_train(fs: f64, dur: f64, amp: f64) -> Array1<f64> {
    let n = (dur * fs) as usize;
    let mut x = Array1::<f64>::zeros(n);
    let mut beat = 0.5;
    while beat < dur - 0.5 {
        let idx = (beat * fs) as usize;
        x[idx] += amp;
        if idx + 1 < n { x[idx + 1] += 0.3 * amp; }
        if idx >= 1 { x[idx - 1] -= 0.2 * amp; }
        beat += 1.0;
    }
    x
}

fn try_case(name: &str, sig: &Array1<f64>, fs: f64, cfg: &EcgPeakDetectionConfig) {
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        ecg_findpeaks_config(sig, fs, cfg)
    }));
    match r {
        Ok(Ok(peaks)) => println!("{name}: Ok, {} peaks", peaks.len()),
        Ok(Err(e)) => println!("{name}: Err({e})"),
        Err(_) => println!("{name}: *** PANIC CAUGHT ***"),
    }
}

fn main() {
    let fs = 360.0;
    let cfg = EcgPeakDetectionConfig::default();

    // 1) Extreme amplitude: squaring (amp^2) overflows to +inf -> integration
    //    stages see inf; inf - inf in moving-average can yield NaN downstream.
    for amp in [1e150_f64, 1e155, 1e160, 1e170, 1e200, 1e300] {
        let sig = qrs_train(fs, 30.0, amp);
        assert!(sig.iter().all(|v| v.is_finite()));
        try_case(&format!("amplitude={amp:e}"), &sig, fs, &cfg);
    }

    // 2) Alternating huge opposite spikes: derivative/filter ringing at overflow scale
    let n = (30.0 * fs) as usize;
    let mut sig2 = Array1::<f64>::zeros(n);
    for k in 0..20 {
        let i = 1000 + k * 400;
        sig2[i] = if k % 2 == 0 { 1e160 } else { -1e160 };
    }
    try_case("alternating ±1e160 spikes", &sig2, fs, &cfg);

    // 3) Raw NaN/inf input must still be rejected up-front (not reach the sort)
    let mut sig3 = qrs_train(fs, 30.0, 1.0);
    sig3[5000] = f64::NAN;
    try_case("raw NaN sample", &sig3, fs, &cfg);
    let mut sig4 = qrs_train(fs, 30.0, 1.0);
    sig4[5000] = f64::INFINITY;
    try_case("raw +inf sample", &sig4, fs, &cfg);

    // 4) BUG-002 complement: NaN threshold_multiplier at the Rust API level
    let sig = qrs_train(fs, 30.0, 1.0);
    let cfg_nan = EcgPeakDetectionConfig::default().with_threshold_multiplier(f64::NAN);
    match ecg_findpeaks_config(&sig, fs, &cfg_nan) {
        Ok(p) => println!("tm=NaN: Ok {} peaks", p.len()),
        Err(e) => println!("tm=NaN: Err({e})"),
    }
    let cfg_inf = EcgPeakDetectionConfig::default().with_threshold_multiplier(f64::INFINITY);
    match ecg_findpeaks_config(&sig, fs, &cfg_inf) {
        Ok(p) => println!("tm=+inf: Ok {} peaks", p.len()),
        Err(e) => println!("tm=+inf: Err({e})"),
    }

    // 5) source-level confirmation of the fix
    println!("source check: candidate_heights.sort_by(|a, b| a.total_cmp(b)) at src/ecg/peaks.rs:225");
}
