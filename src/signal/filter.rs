use ndarray::Array1;
use realfft::RealFftPlanner;

/// Apply a zero-phase digital filter utilizing frequency-domain magnitude multiplication.
/// This acts as a highly efficient stand-in for `scipy.signal.filtfilt` using a Butterworth magnitude.
pub fn signal_filter(
    signal: &Array1<f64>,
    sampling_rate: f64,
    lowcut: Option<f64>,
    highcut: Option<f64>,
    order: usize,
) -> Array1<f64> {
    let n = signal.len();
    if n == 0 {
        return signal.clone();
    }

    let mut planner = RealFftPlanner::<f64>::new();
    let r2c = planner.plan_fft_forward(n);
    let c2r = planner.plan_fft_inverse(n);

    let mut indata = signal.to_vec();
    // In realfft, output complex array is length N/2 + 1
    let mut spectrum = r2c.make_output_vec();

    // Forward FFT
    r2c.process(&mut indata, &mut spectrum).expect("FFT failed");

    // Multiply by zero-phase Butterworth magnitude response
    for k in 0..spectrum.len() {
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

        spectrum[k].re *= h2;
        spectrum[k].im *= h2;
    }

    // Inverse FFT
    let mut outdata = c2r.make_output_vec();
    c2r.process(&mut spectrum, &mut outdata).expect("IFFT failed");

    // Normalize
    let norm = 1.0 / (n as f64);
    for x in outdata.iter_mut() {
        *x *= norm;
    }

    Array1::from_vec(outdata)
}
