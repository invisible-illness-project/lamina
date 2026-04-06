use ndarray::Array1;

/// Computes RMSSD (Root Mean Square of Successive Differences) from NN/RR intervals (in ms).
pub fn hrv_rmssd(intervals: &Array1<f64>) -> Option<f64> {
    let n = intervals.len();
    if n < 2 {
        return None;
    }

    let mut sq_diff_sum = 0.0;
    for i in 0..(n - 1) {
        let diff = intervals[i + 1] - intervals[i];
        sq_diff_sum += diff * diff;
    }

    Some((sq_diff_sum / (n - 1) as f64).sqrt())
}

/// Computes the Mean of NN intervals (in ms).
pub fn hrv_mean_nn(intervals: &Array1<f64>) -> Option<f64> {
    let n = intervals.len();
    if n == 0 {
        return None;
    }
    intervals.mean()
}
