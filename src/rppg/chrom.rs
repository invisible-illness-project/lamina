use crate::error::{Result, SignalError};
use crate::rppg::algorithms::RppgAlgorithm;
use crate::rppg::config::{RppgAlgorithmId, RppgConfig};
use crate::rppg::signal::OpticalSignal;

/// Chrominance-based rPPG extraction algorithm (de Haan & Jeanne, 2013).
///
/// # Citation
/// de Haan G, Jeanne V. Robust pulse rate from chrominance-based rPPG.
/// *IEEE Transactions on Biomedical Engineering*. 2013;60(10):2878-2886.
/// DOI: [10.1109/TBME.2013.2266196](https://doi.org/10.1109/TBME.2013.2266196) | PMID: [23744659](https://pubmed.ncbi.nlm.nih.gov/23744659/)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ChromAlgorithm;

impl RppgAlgorithm for ChromAlgorithm {
    fn id(&self) -> RppgAlgorithmId {
        RppgAlgorithmId::Chrom
    }

    fn extract_window(&self, optical: &OpticalSignal, _config: &RppgConfig) -> Result<Vec<f64>> {
        optical.validate()?;
        let n = optical.red.len();
        if n < 4 {
            return Err(SignalError::InsufficientSamples {
                required: 4,
                provided: n,
            });
        }

        let mean_r = optical.red.iter().sum::<f64>() / n as f64;
        let mean_g = optical.green.iter().sum::<f64>() / n as f64;
        let mean_b = optical.blue.iter().sum::<f64>() / n as f64;

        if mean_r <= 1e-6 || mean_g <= 1e-6 || mean_b <= 1e-6 {
            return Err(SignalError::NonFiniteInput);
        }
        if !mean_r.is_finite() || !mean_g.is_finite() || !mean_b.is_finite() {
            return Err(SignalError::NonFiniteInput);
        }

        // 1. Channel normalization: C_n = C / mean(C) - 1.0
        let r_n: Vec<f64> = optical.red.iter().map(|&r| r / mean_r - 1.0).collect();
        let g_n: Vec<f64> = optical.green.iter().map(|&g| g / mean_g - 1.0).collect();
        let b_n: Vec<f64> = optical.blue.iter().map(|&b| b / mean_b - 1.0).collect();

        // 2. Chrominance signals:
        // X = 3*R_n - 2*G_n
        // Y = 1.5*R_n + 1.5*G_n - 3*B_n
        let mut x_vec = Vec::with_capacity(n);
        let mut y_vec = Vec::with_capacity(n);

        for i in 0..n {
            let x = 3.0 * r_n[i] - 2.0 * g_n[i];
            let y = 1.5 * r_n[i] + 1.5 * g_n[i] - 3.0 * b_n[i];
            x_vec.push(x);
            y_vec.push(y);
        }

        // 3. Standard deviations:
        let mean_x = x_vec.iter().sum::<f64>() / n as f64;
        let mean_y = y_vec.iter().sum::<f64>() / n as f64;

        let var_x = x_vec.iter().map(|v| (v - mean_x).powi(2)).sum::<f64>() / n as f64;
        let var_y = y_vec.iter().map(|v| (v - mean_y).powi(2)).sum::<f64>() / n as f64;

        let std_x = var_x.sqrt();
        let std_y = var_y.sqrt();

        // 4. Alpha ratio: alpha = std_X / std_Y
        let alpha = if std_y > 1e-12 {
            std_x / std_y
        } else if std_x <= 1e-12 {
            0.0
        } else {
            return Err(SignalError::NonFiniteInput);
        };

        if !alpha.is_finite() {
            return Err(SignalError::NonFiniteInput);
        }

        // 5. CHROM pulse projection: S = X - alpha * Y
        let pulse: Vec<f64> = x_vec
            .iter()
            .zip(y_vec.iter())
            .map(|(&x, &y)| x - alpha * y)
            .collect();

        Ok(pulse)
    }
}
