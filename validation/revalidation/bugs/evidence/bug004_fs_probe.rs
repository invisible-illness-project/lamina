//! BUG-004 revalidation probe: explicit-fs eda_findpeaks / rsp_findpeaks.
//! Behavioral check: same underlying synthetic signal sampled at fs=25 vs fs=100
//! must give consistent peak TIMING (seconds), plus fs validation errors.
use lamina::eda::eda_findpeaks;
use lamina::rsp::rsp_findpeaks;
use ndarray::Array1;

fn gaussian_train_sec(fs: f64, dur_sec: f64, centers_sec: &[f64], width_sec: f64, amp: f64) -> Array1<f64> {
    let n = (dur_sec * fs) as usize;
    let mut x = Array1::<f64>::zeros(n);
    for i in 0..n {
        let t = i as f64 / fs;
        let mut v = 0.0;
        for &c in centers_sec {
            let d = t - c;
            v += amp * (-0.5 * (d / width_sec).powi(2)).exp();
        }
        x[i] = v;
    }
    x
}

fn mask_peak_times(mask: &Array1<bool>, fs: f64) -> Vec<f64> {
    mask.iter().enumerate().filter(|(_, &v)| v).map(|(i, _)| i as f64 / fs).collect()
}

fn main() {
    // ---- RSP timing consistency: 12 breaths, 3.0 s apart, true times 2..35 s
    let breath_times: Vec<f64> = (0..12).map(|k| 2.0 + 3.0 * k as f64).collect();
    for fs in [25.0_f64, 100.0] {
        let sig = gaussian_train_sec(fs, 40.0, &breath_times, 0.6, 1.0);
        let mask = rsp_findpeaks(&sig, fs).expect("rsp_findpeaks");
        let times = mask_peak_times(&mask, fs);
        println!("RSP fs={fs:6.1}: {} peaks at {:?}", times.len(),
                 times.iter().map(|t| format!("{t:.2}")).collect::<Vec<_>>());
    }
    // ---- EDA timing consistency: 6 SCR pulses, 2.5 s apart
    let scr_times: Vec<f64> = (0..6).map(|k| 3.0 + 2.5 * k as f64).collect();
    for fs in [25.0_f64, 100.0] {
        let sig = gaussian_train_sec(fs, 20.0, &scr_times, 0.10, 1.0);
        let mask = eda_findpeaks(&sig, fs).expect("eda_findpeaks");
        let times = mask_peak_times(&mask, fs);
        println!("EDA fs={fs:6.1}: {} peaks at {:?}", times.len(),
                 times.iter().map(|t| format!("{t:.2}")).collect::<Vec<_>>());
    }

    // ---- fs validation: 0, negative, NaN, inf -> InvalidSamplingRate
    let sig = gaussian_train_sec(25.0, 40.0, &breath_times, 0.6, 1.0);
    for bad in [0.0_f64, -25.0, f64::NAN, f64::INFINITY] {
        let r = rsp_findpeaks(&sig, bad);
        println!("rsp_findpeaks fs={bad}: {}", match r {
            Ok(m) => format!("OK?! {} peaks", m.iter().filter(|&&v| v).count()),
            Err(e) => format!("Err({e})"),
        });
        let e = eda_findpeaks(&sig, bad);
        println!("eda_findpeaks fs={bad}: {}", match e {
            Ok(m) => format!("OK?! {} peaks", m.iter().filter(|&&v| v).count()),
            Err(e) => format!("Err({e})"),
        });
    }

    // ---- Regression check vs the OLD buggy semantics: at true fs=25, the old
    // hardcoded-100Hz rsp wrapper kept 1 of 25 breaths. New wrapper with fs=25
    // must keep them all (min_breath_interval 1.2 s -> 30 samples, not 120).
    let breaths25: Vec<f64> = (0..25).map(|k| 2.0 + 2.0 * k as f64).collect();
    let sig25 = gaussian_train_sec(25.0, 60.0, &breaths25, 0.3, 1.0);
    let m = rsp_findpeaks(&sig25, 25.0).expect("rsp");
    println!("RSP regression: true_fs=25, 25 breaths 2s apart -> {} peaks (old hardcoded-100 gave 1)",
             m.iter().filter(|&&v| v).count());
    // And the EDA case: true fs=250, 10 pulses 0.6 s apart, default min_distance 1.0 s
    let pulses: Vec<f64> = (0..10).map(|k| 5.0 + 0.6 * k as f64).collect();
    let sig250 = gaussian_train_sec(250.0, 30.0, &pulses, 0.05, 1.0);
    let m = eda_findpeaks(&sig250, 250.0).expect("eda");
    println!("EDA regression: true_fs=250, 10 pulses 0.6s apart -> {} peaks (old hardcoded-100 gave 10; config-correct = 5)",
             m.iter().filter(|&&v| v).count());
}
