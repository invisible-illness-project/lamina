use crate::error::{Result, SignalError};
use crate::signal::filter::{FilterSpec, signal_filtfilt};
use ndarray::Array1;

/// Clean an Electrodermal Activity (EDA/GSR) signal using a zero-phase 5Hz 4th-order low-pass Butterworth filter.
///
/// # Scientific Contract
/// - **Inputs**:
///   - `signal`: Raw EDA skin conductance signal array in microsiemens ($\mu\text{S}$).
///   - `sampling_rate`: Sampling frequency $F_s$ in Hertz (Hz). Must be $> 0.0$.
/// - **Output**: Cleaned EDA signal array of identical length $N$.
/// - **Methodology**: Applies zero-phase 4th-order low-pass Butterworth filtering at 5.0 Hz (`signal_filtfilt`) to suppress high-frequency noise spikes while preserving skin conductance variations.
///
/// # Errors
/// Returns [`SignalError`] if `signal` is empty, contains non-finite samples, if `sampling_rate` is invalid, or if cutoff exceeds Nyquist.
pub fn eda_clean(signal: &Array1<f64>, sampling_rate: f64) -> Result<Array1<f64>> {
    let n = signal.len();
    if n == 0 {
        return Err(SignalError::EmptySignal);
    }
    if !sampling_rate.is_finite() || sampling_rate <= 0.0 {
        return Err(SignalError::InvalidSamplingRate(sampling_rate));
    }
    for &val in signal.iter() {
        if !val.is_finite() {
            return Err(SignalError::NonFiniteInput);
        }
    }

    let cutoff = 5.0;
    let nyquist = sampling_rate / 2.0;
    if cutoff >= nyquist {
        return Err(SignalError::InvalidCutoffFrequency(format!(
            "Cutoff frequency ({}) must be strictly less than Nyquist frequency ({})",
            cutoff, nyquist
        )));
    }

    let filter_order = 4;
    if n < 3 * filter_order {
        return Err(SignalError::InsufficientSamples {
            required: 3 * filter_order,
            provided: n,
        });
    }

    let filter_spec = FilterSpec::lowpass(sampling_rate, cutoff, filter_order);
    signal_filtfilt(signal, &filter_spec)
}
