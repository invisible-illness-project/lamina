use ndarray::Array1;

/// Find local maxima in a 1D array.
/// Returns a boolean mask of the same size as the input, where `true` indicates a peak.
pub fn signal_findpeaks(signal: &Array1<f64>) -> Array1<bool> {
    let n = signal.len();
    let mut peaks = Array1::<bool>::from_elem(n, false);
    
    if n < 3 {
        return peaks;
    }

    for i in 1..(n - 1) {
        if signal[i] > signal[i - 1] && signal[i] > signal[i + 1] {
            peaks[i] = true;
        }
    }

    peaks
}
