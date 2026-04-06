# Lamina (Rust) 🦀

Lamina is a high-performance, safe Rust library for neurophysiological signal processing (ECG, PPG, EDA, HRV, RSP). It is heavily inspired by Python's [NeuroKit2](https://github.com/neuropsychology/NeuroKit) and designed with zero-cost abstractions, making it suitable for integration in high-throughput or embedded sensing systems.

## Modules

Lamina currently features the structural scaffolding and algorithmic processing for:

- `signal`: Core processing utilities. Features advanced zero-phase digital filtering utilizing `realfft` frequency-domain multiplication and basic extrema trackers.
- `ecg`: Electrocardiogram routines mimicking Pan-Tompkins QRS logic and baseline wander removal.
- `ppg`: Photoplethysmogram techniques featuring adaptive thresholds and dicrotic notch elimination.
- `eda`: Electrodermal Activity pipelines isolating Phasic elements (Skin Conductance Response) from the Tonic baseline.
- `rsp`: Respiratory signal structures bounded by physiological respiration rates to detect individual breaths.
- `hrv`: Time-domain Heart Rate Variability summarizations (RMSSD, Mean NN).
- `complexity`: Non-linear state metrics featuring the Sample Entropy (SampEn) algorithm.

## Architecture

To prioritize maximum speed and simple dependency graphs, Lamina centers its data processing exclusively around `ndarray::Array1<f64>`. 

Dependencies rely on standard mathematical implementations, with operations offloaded to standard fast ecosystems like `rustfft` and `statrs` without necessitating heavy, volatile dataframe libraries.

## Contributing and Getting Started

Welcome to Lamina! As we continue porting features from NeuroKit2 into safe systems Rust, developer contributions are highly appreciated.

### Setting up the Environment
1. Ensure you have standard Rust toolchains installed (`cargo`).
2. Run `cargo build` in this directory to pull dependencies such as `ndarray` and `rustfft`.

### Validating Changes
- **Unit Tests:** Always ensure you add module-specific logic tests when adding new methodologies (e.g. `tests/eda_tests.rs`).
- run `cargo test` to execute native synthetic tests and ensure your additions didn't break continuous structures.

### Integration Testing with NeuroKit2 (Golden Datasets)
When porting a complex feature from Python, we ensure precision parity against Python outputs:
1. Ensure you have Python (`>=3.9`), `numpy`, `scipy` and `neurokit2` (`<=0.2.7`) installed via pip in your local env or conda.
2. We have provided `tests/generate_test_data.py`. Run this Python script to extract known sample sets through NeuroKit2 into absolute `golden` JSON files.
3. Your final validation test (e.g., inside `tests/integration_tests.rs`) should deserialize `golden_ecg.json` and forcefully align your Rust implementations against it.

*Golden Data Integration ensures Lamina always tracks alongside academic standards set by NeuroKit2!*
