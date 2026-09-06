use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use lamina::complexity::entropy::sample_entropy;
use lamina::ecg::{EcgPeakDetectionConfig, ecg_findpeaks_config};
use lamina::eda::{
    EdaDecompositionConfig, EdaPeakDetectionConfig, eda_clean, eda_decompose, eda_findpeaks_events,
};
use lamina::features::{
    FeatureConfig, MultimodalInput, WindowConfig, extract_features, extract_features_naive,
};
use lamina::multimodal::{cardiorespiratory_phase_coupling, ecg_ppg_timing};
use lamina::ppg::{PpgPeakDetectionConfig, ppg_findpeaks_config};
use lamina::rsp::{
    RspCleaningConfig, RspProcessingConfig, rsp_clean_config, rsp_cycles_config, rsp_rate_config,
};
use lamina::signal::filter::{FilterSpec, SosFilter, signal_filtfilt};
use lamina::signal::peaks::{PeakDetectionConfig, signal_findpeaks_config};
use lamina::signal::smooth::signal_smooth_moving_average;
use ndarray::Array1;

fn bench_filter_design(c: &mut Criterion) {
    let mut group = c.benchmark_group("filter_design_from_spec");

    let specs = vec![
        ("lowpass_ord4", FilterSpec::lowpass(100.0, 5.0, 4)),
        ("highpass_ord4", FilterSpec::highpass(100.0, 0.5, 4)),
        ("bandpass_ord4", FilterSpec::bandpass(100.0, 5.0, 15.0, 4)),
        ("notch_ord4", FilterSpec::notch(100.0, 18.0, 22.0, 4)),
    ];

    for (name, spec) in specs {
        group.bench_function(name, |b| {
            b.iter(|| SosFilter::from_spec(&spec).unwrap());
        });
    }
    group.finish();
}

fn bench_filtfilt(c: &mut Criterion) {
    let mut group = c.benchmark_group("signal_filtfilt_lowpass_order4");
    for size in [1000, 10000, 100000].iter() {
        let signal = Array1::from_elem(*size, 1.0);
        let spec = FilterSpec::lowpass(100.0, 5.0, 4);

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| signal_filtfilt(&signal, &spec).unwrap());
        });
    }
    group.finish();
}

fn bench_moving_average(c: &mut Criterion) {
    let mut group = c.benchmark_group("signal_smooth_moving_average");
    for size in [1000, 10000, 100000].iter() {
        let signal = Array1::from_elem(*size, 1.0);
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| signal_smooth_moving_average(&signal, 51).unwrap());
        });
    }
    group.finish();
}

fn bench_findpeaks(c: &mut Criterion) {
    let mut group = c.benchmark_group("signal_findpeaks_configured");
    let config = PeakDetectionConfig::new()
        .with_min_height(2.0)
        .with_min_distance(30)
        .with_min_prominence(1.0);

    for size in [1000, 10000, 100000].iter() {
        let mut signal = Array1::<f64>::zeros(*size);
        for i in (0..*size).step_by(50) {
            signal[i] = 5.0;
        }
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| signal_findpeaks_config(&signal, &config).unwrap());
        });
    }
    group.finish();
}

fn bench_ecg_pipeline(c: &mut Criterion) {
    let mut group = c.benchmark_group("ecg_findpeaks_pan_tompkins");
    let fs = 100.0;
    let config = EcgPeakDetectionConfig::default();

    for size in [1000, 5000].iter() {
        let mut signal = Array1::<f64>::zeros(*size);
        for i in (50..*size).step_by(100) {
            signal[i] = 4.0;
        }
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| ecg_findpeaks_config(&signal, fs, &config).unwrap());
        });
    }
    group.finish();
}

fn bench_ppg_pipeline(c: &mut Criterion) {
    let mut group = c.benchmark_group("ppg_findpeaks_elgendi");
    let fs = 100.0;
    let config = PpgPeakDetectionConfig::default();

    for size in [1000, 5000].iter() {
        let mut signal = Array1::<f64>::zeros(*size);
        for i in (50..*size).step_by(100) {
            for offset in -5..=5 {
                let idx = (i as i64 + offset) as usize;
                if idx < *size {
                    signal[idx] = 2.0;
                }
            }
        }
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| ppg_findpeaks_config(&signal, fs, &config).unwrap());
        });
    }
    group.finish();
}

fn bench_eda_pipeline(c: &mut Criterion) {
    let mut group = c.benchmark_group("eda_processing_pipeline");
    let fs = 100.0;
    let decomp_cfg = EdaDecompositionConfig::default();
    let peak_cfg = EdaPeakDetectionConfig::default();

    for size in [1000, 5000, 10000, 100000].iter() {
        let mut signal = Array1::<f64>::zeros(*size);
        for i in (100..*size).step_by(200) {
            signal[i] = 2.0;
        }

        group.bench_with_input(BenchmarkId::new("eda_decompose", size), size, |b, _| {
            b.iter(|| eda_decompose(&signal, fs, &decomp_cfg).unwrap());
        });

        group.bench_with_input(BenchmarkId::new("eda_full_pipeline", size), size, |b, _| {
            b.iter(|| {
                let cleaned = eda_clean(&signal, fs).unwrap();
                let comp = eda_decompose(&cleaned, fs, &decomp_cfg).unwrap();
                eda_findpeaks_events(&comp.phasic, fs, &peak_cfg).unwrap();
            });
        });
    }
    group.finish();
}

fn bench_rsp_pipeline(c: &mut Criterion) {
    let mut group = c.benchmark_group("rsp_processing_pipeline");
    let fs = 100.0;
    let clean_cfg = RspCleaningConfig::default();
    let proc_cfg = RspProcessingConfig::default();

    for size in [1000, 5000, 10000, 100000].iter() {
        let mut signal = Array1::<f64>::zeros(*size);
        for i in 0..*size {
            let t = i as f64 / fs;
            signal[i] = (2.0 * std::f64::consts::PI * 0.25 * t).sin();
        }

        group.bench_with_input(BenchmarkId::new("rsp_clean", size), size, |b, _| {
            b.iter(|| rsp_clean_config(&signal, fs, &clean_cfg).unwrap());
        });

        group.bench_with_input(BenchmarkId::new("rsp_cycles", size), size, |b, _| {
            b.iter(|| rsp_cycles_config(&signal, fs, &proc_cfg).unwrap());
        });

        group.bench_with_input(BenchmarkId::new("rsp_full_pipeline", size), size, |b, _| {
            b.iter(|| {
                let cleaned = rsp_clean_config(&signal, fs, &clean_cfg).unwrap();
                let _cycles = rsp_cycles_config(&cleaned, fs, &proc_cfg).unwrap();
                rsp_rate_config(&cleaned, fs, &proc_cfg).unwrap();
            });
        });
    }
    group.finish();
}

fn bench_multimodal_pipeline(c: &mut Criterion) {
    let mut group = c.benchmark_group("multimodal_processing_pipeline");
    let fs = 100.0;

    for count in [1000, 10000, 100000].iter() {
        let ecg_peaks: Vec<usize> = (0..*count).map(|i| i * 100).collect();
        let ppg_peaks: Vec<usize> = (0..*count).map(|i| i * 100 + 20).collect();
        let phases: Vec<f64> = (0..*count)
            .map(|i| (i as f64 * 0.1) % (2.0 * std::f64::consts::PI))
            .collect();

        group.bench_with_input(BenchmarkId::new("ecg_ppg_timing", count), count, |b, _| {
            b.iter(|| ecg_ppg_timing(&ecg_peaks, fs, 0.0, &ppg_peaks, fs, 0.0).unwrap());
        });

        group.bench_with_input(BenchmarkId::new("phase_coupling", count), count, |b, _| {
            b.iter(|| cardiorespiratory_phase_coupling(&phases).unwrap());
        });
    }

    group.finish();
}

fn bench_feature_extraction(c: &mut Criterion) {
    let mut group = c.benchmark_group("feature_extraction_layer");
    let fs = 100.0;

    // 1. Event Count Matrix (10k, 100k, 1M events)
    for &n_events in &[10_000, 100_000, 1_000_000] {
        let r_peaks: Vec<usize> = (0..n_events)
            .map(|i| (i as f64 * 0.1 * fs).round() as usize) // 10 Hz event density
            .collect();

        let input = MultimodalInput::new().with_ecg(r_peaks, fs, 0.0).unwrap();

        let cfg = FeatureConfig {
            window: WindowConfig {
                window_duration_sec: 60.0,
                step_sec: 30.0,
                min_coverage: 0.8,
            },
            ..FeatureConfig::default()
        };

        group.bench_with_input(
            BenchmarkId::new("events_count_60s_win_30s_hop", n_events),
            &n_events,
            |b, _| {
                b.iter(|| extract_features(&input, &cfg).unwrap());
            },
        );
    }

    // 2. Window Duration & Hop Size Matrix (30s, 60s, 300s durations; 1.0x, 0.5x, 0.1x hops)
    let dur_sec = 600.0; // 10 min recording
    let n_events = (dur_sec * fs) as usize / 100; // 1 beat / sec
    let r_peaks: Vec<usize> = (0..n_events)
        .map(|i| (i as f64 * fs).round() as usize)
        .collect();
    let input = MultimodalInput::new().with_ecg(r_peaks, fs, 0.0).unwrap();

    for &win_dur in &[30.0, 60.0, 300.0] {
        for &hop_mult in &[1.0, 0.5, 0.1] {
            let step_sec = win_dur * hop_mult;
            let cfg = FeatureConfig {
                window: WindowConfig {
                    window_duration_sec: win_dur,
                    step_sec,
                    min_coverage: 0.8,
                },
                ..FeatureConfig::default()
            };

            let label = format!("win_{}s_hop_{:.1}x", win_dur as usize, hop_mult);
            group.bench_function(BenchmarkId::new("window_hop_matrix", label), |b| {
                b.iter(|| extract_features(&input, &cfg).unwrap());
            });
        }
    }

    // 3. Naive vs Optimized comparison on moderate workload (10,000 events)
    let n_events_comp = 10_000;
    let r_peaks_comp: Vec<usize> = (0..n_events_comp)
        .map(|i| (i as f64 * 0.1 * fs).round() as usize)
        .collect();
    let input_comp = MultimodalInput::new()
        .with_ecg(r_peaks_comp, fs, 0.0)
        .unwrap();

    let cfg_comp = FeatureConfig {
        window: WindowConfig {
            window_duration_sec: 60.0,
            step_sec: 10.0, // Overlapping hops
            min_coverage: 0.8,
        },
        ..FeatureConfig::default()
    };

    group.bench_function("comparison_optimized_10k_events", |b| {
        b.iter(|| extract_features(&input_comp, &cfg_comp).unwrap());
    });

    group.bench_function("comparison_naive_10k_events", |b| {
        b.iter(|| extract_features_naive(&input_comp, &cfg_comp).unwrap());
    });

    group.finish();
}

fn bench_sample_entropy(c: &mut Criterion) {
    let mut group = c.benchmark_group("sample_entropy");
    let size = 500;
    let mut signal = Array1::<f64>::zeros(size);
    for i in 0..size {
        signal[i] = (i as f64 * 0.1).sin();
    }
    group.bench_function("500_samples_m2", |b| {
        b.iter(|| sample_entropy(&signal, 2, 0.2).unwrap());
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_filter_design,
    bench_filtfilt,
    bench_moving_average,
    bench_findpeaks,
    bench_ecg_pipeline,
    bench_ppg_pipeline,
    bench_eda_pipeline,
    bench_rsp_pipeline,
    bench_multimodal_pipeline,
    bench_feature_extraction,
    bench_sample_entropy
);
criterion_main!(benches);
