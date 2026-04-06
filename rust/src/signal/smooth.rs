use ndarray::{Array1, ArrayView1};

/// Smooth a signal using a rolling moving average
pub fn signal_smooth_moving_average(signal: &Array1<f64>, window_size: usize) -> Array1<f64> {
    if window_size <= 1 || signal.is_empty() {
        return signal.clone();
    }

    let n = signal.len();
    let mut smoothed = Array1::<f64>::zeros(n);
    
    // Half-window for centered moving average
    let half_win = window_size / 2;

    for i in 0..n {
        let start = i.saturating_sub(half_win);
        let end = (i + half_win + 1).min(n);
        let slice: ArrayView1<f64> = signal.slice(ndarray::s![start..end]);
        smoothed[i] = slice.mean().unwrap_or(0.0);
    }

    smoothed
}
