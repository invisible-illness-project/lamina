use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use lamina::complexity::entropy::sample_entropy;
use lamina::signal::filter::signal_filter;
use lamina::signal::peaks::signal_findpeaks;
use lamina::signal::smooth::signal_smooth_moving_average;
use ndarray::Array1;

fn bench_moving_average(c: &mut Criterion) {
    let mut group = c.benchmark_group("signal_smooth_moving_average");
    for size in [1000, 10000].iter() {
        let signal = Array1::from_elem(*size, 1.0);
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| signal_smooth_moving_average(&signal, 51).unwrap());
        });
    }
    group.finish();
}

fn bench_findpeaks(c: &mut Criterion) {
    let mut group = c.benchmark_group("signal_findpeaks");
    let size = 10000;
    let mut signal = Array1::<f64>::zeros(size);
    for i in (0..size).step_by(50) {
        signal[i] = 10.0;
    }
    group.bench_function("10k_samples", |b| {
        b.iter(|| signal_findpeaks(&signal).unwrap());
    });
    group.finish();
}

fn bench_filter(c: &mut Criterion) {
    let mut group = c.benchmark_group("signal_filter");
    let size = 10000;
    let signal = Array1::from_elem(size, 1.0);
    group.bench_function("10k_samples_lowpass", |b| {
        b.iter(|| signal_filter(&signal, 100.0, None, Some(5.0), 3).unwrap());
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
    bench_moving_average,
    bench_findpeaks,
    bench_filter,
    bench_sample_entropy
);
criterion_main!(benches);
