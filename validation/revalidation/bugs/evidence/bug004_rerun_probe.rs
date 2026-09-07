// VERBATIM copy of validation/results/bugs-repro/repro_fs100_hardcoded_probe.rs
// (as checked in at remediation commit fec2668 — already ported to the new fs-explicit API)
use lamina::eda::{eda_findpeaks, eda_findpeaks_config, EdaPeakDetectionConfig};
use lamina::rsp::{rsp_findpeaks, rsp_findpeaks_config, RspProcessingConfig};
use ndarray::Array1;

fn gaussian_train(n: usize, centers: &[usize], width: f64, amp: f64) -> Array1<f64> {
    let mut x = Array1::<f64>::zeros(n);
    for i in 0..n {
        let mut v = 0.0;
        for &c in centers {
            let d = i as f64 - c as f64;
            v += amp * (-0.5 * (d / width).powi(2)).exp();
        }
        x[i] = v;
    }
    x
}

fn main() {
    // ---- EDA: true fs = 250 Hz. SCR peaks 0.6 s apart (150 samples).
    // min_distance_sec default = 1.0 s.
    // - eda_findpeaks (hardcoded fs=100): 1.0 s -> 100 samples -> 150 >= 100, keeps both.
    // - eda_findpeaks_config(fs=250):     1.0 s -> 250 samples -> 150 < 250, drops one.
    let fs_eda = 250.0;
    let n = (30.0 * fs_eda) as usize;
    let centers: Vec<usize> = (0..10).map(|k| ((5.0 + 0.6 * k as f64) * fs_eda) as usize).collect();
    let phasic = gaussian_train(n, &centers, 0.05 * fs_eda, 1.0);

    let mask_default = eda_findpeaks(&phasic, fs_eda).expect("eda_findpeaks");
    let n_default = mask_default.iter().filter(|&&v| v).count();
    let cfg = EdaPeakDetectionConfig::default();
    let peaks_truefs = eda_findpeaks_config(&phasic, fs_eda, &cfg).expect("eda_findpeaks_config");
    println!("EDA  true_fs={} Hz, {} SCR pulses spaced 0.6 s", fs_eda, centers.len());
    println!("  eda_findpeaks(fs={})                    -> {} peaks", fs_eda, n_default);
    println!("  eda_findpeaks_config(fs={})            -> {} peaks", fs_eda, peaks_truefs.len());

    // ---- RSP: true fs = 25 Hz. Breaths 2.0 s apart (50 samples).
    // min_breath_interval_sec default = 1.2 s.
    let fs_rsp = 25.0;
    let n = (60.0 * fs_rsp) as usize;
    let centers: Vec<usize> = (0..25).map(|k| ((2.0 + 2.0 * k as f64) * fs_rsp) as usize).collect();
    let cleaned = gaussian_train(n, &centers, 0.3 * fs_rsp, 1.0);

    let mask_default = rsp_findpeaks(&cleaned, fs_rsp).expect("rsp_findpeaks");
    let n_default = mask_default.iter().filter(|&&v| v).count();
    let cfg = RspProcessingConfig::default();
    let peaks_truefs = rsp_findpeaks_config(&cleaned, fs_rsp, &cfg).expect("rsp_findpeaks_config");
    println!("RSP  true_fs={} Hz, {} breaths spaced 2.0 s", fs_rsp, centers.len());
    println!("  rsp_findpeaks (hardcoded fs=100)        -> {} peaks", n_default);
    println!("  rsp_findpeaks_config(fs={})             -> {} peaks", fs_rsp, peaks_truefs.len());
}
