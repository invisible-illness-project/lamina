use ndarray::Array1;

/// Converts a boolean peak array into R-R intervals (or generic peak-to-peak intervals) in milliseconds.
pub fn peaks_to_intervals(
    peaks: &Array1<bool>,
    sampling_rate: f64,
) -> Array1<f64> {
    let peak_indices: Vec<usize> = peaks.iter()
        .enumerate()
        .filter_map(|(i, &p)| if p { Some(i) } else { None })
        .collect();

    if peak_indices.len() < 2 {
        return Array1::<f64>::zeros(0);
    }

    let mut intervals = Array1::<f64>::zeros(peak_indices.len() - 1);
    for i in 0..(peak_indices.len() - 1) {
        let diff_samples = peak_indices[i + 1] - peak_indices[i];
        intervals[i] = (diff_samples as f64 / sampling_rate) * 1000.0;
    }

    intervals
}
