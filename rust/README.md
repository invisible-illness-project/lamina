# Lamina (Rust) 🦀

Lamina is a high-performance, safe Rust library for neurophysiological signal processing (ECG, PPG, EDA, EMG, RSP). It is heavily inspired by Python's [NeuroKit2](https://github.com/neuropsychology/NeuroKit) and designed with zero-cost abstractions, making it suitable for integration in high-throughput or embedded sensing systems.

## Modules

Currently, Lamina features the scaffolding and basic logic for:

- `signal`: Core processing utilities (filters, smoothing, and local peak detection).
- `ecg`: Electrocardiogram routines mimicking Pan-Tompkins QRS logic and baseline wander removal.
- `ppg`: Photoplethysmogram techniques featuring adaptive thresholds and dicrotic notch elimination.
- `hrv`: Time-domain Heart Rate Variability summarizations (RMSSD, Mean NN).

## Architecture

To prioritize maximum speed and simple dependency graphs, Lamina centers its data processing exclusively around `ndarray::Array1<f64>`. 

Dependencies rely on standard implementations, with mathematical operations offloaded to standard fast ecosystems like `rustfft` and `statrs` without necessitating heavy dataframe libraries.

## Goal

Eventually, Lamina's Rust backend architecture is expected to serve as the main signal processor for the broader Lamina ecosystem, outperforming the Python-dependent counterparts safely and asynchronously.

## Testing

Golden datasets reflecting NeuroKit2 behaviors can be derived using the python utility inside `tests`. Assuming standard Rust toolchains are loaded:

```bash
cargo test
```
