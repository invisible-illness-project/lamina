use crate::error::{Result, SignalError};
use ndarray::Array1;
use realfft::RealFftPlanner;

/// Apply digital frequency-domain filtering using Butterworth magnitude multiplication.
///
/// # Scientific Contract
/// - **Inputs**:
///   - `signal`: 1D array of real-valued floating-point samples (`f64`).
///   - `sampling_rate`: Sampling frequency $F_s$ in Hertz (Hz). Must be finite and $> 0.0$.
///   - `lowcut`: Optional highpass cutoff frequency in Hz ($f_{\text{low}} > 0.0$).
///   - `highcut`: Optional lowpass cutoff frequency in Hz ($f_{\text{high}} < F_s / 2$).
///   - `order`: Filter order $N \ge 1$.
/// - **Output**: Filtered 1D signal of identical length.
/// - **Boundary Behavior**: Operates via RealFFT magnitude multiplication without boundary reflection.
/// - **Note**: This frequency-domain magnitude multiplication is a fast zero-phase approximation.
///   Full time-domain `scipy.signal.filtfilt` zero-phase IIR filtering with boundary extension
///   is planned for a future release via `biquad` Second-Order Sections (SOS).
///
/// # Errors
/// Returns [`SignalError`] if:
/// - `signal` is empty ([`SignalError::EmptySignal`]).
/// - `signal` contains non-finite samples ([`SignalError::NonFiniteInput`]).
/// - `sampling_rate` is non-positive or non-finite ([`SignalError::InvalidSamplingRate`]).
/// - `order` is 0 ([`SignalError::InvalidFilterOrder`]).
/// - `lowcut` or `highcut` frequencies violate Nyquist limits ($0 < f_{\text{low}} < f_{\text{high}} < F_s / 2$) ([`SignalError::InvalidCutoffFrequency`]).
pub fn signal_filter(
    signal: &Array1<f64>,
    sampling_rate: f64,
    lowcut: Option<f64>,
    highcut: Option<f64>,
    order: usize,
) -> Result<Array1<f64>> {
    let n = signal.len();
    if n == 0 {
        return Err(SignalError::EmptySignal);
    }
    if !sampling_rate.is_finite() || sampling_rate <= 0.0 {
        return Err(SignalError::InvalidSamplingRate(sampling_rate));
    }
    if order == 0 {
        return Err(SignalError::InvalidFilterOrder(order));
    }
    for &val in signal.iter() {
        if !val.is_finite() {
            return Err(SignalError::NonFiniteInput);
        }
    }

    let nyquist = sampling_rate / 2.0;

    if let Some(lc) = lowcut {
        let invalid = !lc.is_finite() || lc <= 0.0 || lc >= nyquist;
        if invalid {
            return Err(SignalError::InvalidCutoffFrequency(format!(
                "Lowcut ({}) must be > 0.0 and < Nyquist frequency ({})",
                lc, nyquist
            )));
        }
    }

    if let Some(hc) = highcut {
        let invalid = !hc.is_finite() || hc <= 0.0 || hc >= nyquist;
        if invalid {
            return Err(SignalError::InvalidCutoffFrequency(format!(
                "Highcut ({}) must be > 0.0 and < Nyquist frequency ({})",
                hc, nyquist
            )));
        }
    }

    if let (Some(lc), Some(hc)) = (lowcut, highcut) {
        let invalid = lc >= hc;
        if invalid {
            return Err(SignalError::InvalidCutoffFrequency(format!(
                "Lowcut ({}) must be strictly less than highcut ({})",
                lc, hc
            )));
        }
    }

    let mut planner = RealFftPlanner::<f64>::new();
    let r2c = planner.plan_fft_forward(n);
    let c2r = planner.plan_fft_inverse(n);

    let mut indata = signal.to_vec();
    let mut spectrum = r2c.make_output_vec();

    // Forward FFT
    r2c.process(&mut indata, &mut spectrum)
        .map_err(|e| SignalError::InvalidCutoffFrequency(e.to_string()))?;

    // Multiply by zero-phase Butterworth magnitude response
    for (k, spec_val) in spectrum.iter_mut().enumerate() {
        let f_k = (k as f64) * sampling_rate / (n as f64);
        let mut h2 = 1.0;

        // Highpass (filters out frequencies below lowcut)
        if let Some(lc) = lowcut {
            if f_k == 0.0 {
                h2 = 0.0;
            } else {
                let ratio = lc / f_k;
                h2 *= 1.0 / (1.0 + ratio.powi(2 * order as i32));
            }
        }

        // Lowpass (filters out frequencies above highcut)
        if let Some(hc) = highcut {
            let ratio = f_k / hc;
            h2 *= 1.0 / (1.0 + ratio.powi(2 * order as i32));
        }

        spec_val.re *= h2;
        spec_val.im *= h2;
    }

    // Inverse FFT
    let mut outdata = c2r.make_output_vec();
    c2r.process(&mut spectrum, &mut outdata)
        .map_err(|e| SignalError::InvalidCutoffFrequency(e.to_string()))?;

    // Normalize
    let norm = 1.0 / (n as f64);
    for x in outdata.iter_mut() {
        *x *= norm;
    }

    Ok(Array1::from_vec(outdata))
}
