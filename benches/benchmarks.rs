use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use lamina::complexity::entropy::sample_entropy;
use lamina::ecg::{EcgPeakDetectionConfig, ecg_findpeaks_config};
use lamina::eda::{
    EdaDecompositionConfig, EdaPeakDetectionConfig, eda_clean, eda_decompose, eda_findpeaks_events,
};
use lamina::ppg::{PpgPeakDetectionConfig, ppg_findpeaks_config};
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
    bench_sample_entropy
);
criterion_main!(benches);
