use ndarray::Array1;

/// Computes Sample Entropy (SampEn) for a 1D signal.
/// 
/// Embeds the signal into dimensions `m` and `m+1` to evaluate repeating template frequencies
/// given a radius tolerance `r`.
/// Returns `f64::NAN` if the signal is too short or `f64::INFINITY` if no matches are found.
pub fn sample_entropy(signal: &Array1<f64>, m: usize, r: f64) -> f64 {
    let n = signal.len();
    if n <= m + 1 {
        return f64::NAN;
    }

    let mut count_m = 0_usize;
    let mut count_m1 = 0_usize;

    // Compare all pairs of templates of length m
    for i in 0..(n - m) {
        for j in (i + 1)..(n - m) {
            let mut match_m = true;
            for k in 0..m {
                if (signal[i + k] - signal[j + k]).abs() > r {
                    match_m = false;
                    break;
                }
            }

            if match_m {
                count_m += 1;
                // Additionally check if the subsequent elements also match (dimension m + 1)
                if i + m < n && j + m < n {
                    if (signal[i + m] - signal[j + m]).abs() <= r {
                        count_m1 += 1;
                    }
                }
            }
        }
    }

    if count_m == 0 || count_m1 == 0 {
        return f64::INFINITY;
    }

    -((count_m1 as f64) / (count_m as f64)).ln()
}
