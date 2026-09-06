use crate::error::{Result, SignalError};
use crate::rppg::algorithms::RppgAlgorithm;
use crate::rppg::config::{RppgAlgorithmId, RppgConfig};
use crate::rppg::signal::OpticalSignal;

/// Plane-Orthogonal-to-Skin (POS) rPPG extraction algorithm (Wang et al., 2017).
///
/// # Citation
/// Wang W, den Brinker AC, Stuijk S, de Haan G. Algorithmic Principles of Remote PPG.
/// *IEEE Transactions on Biomedical Engineering*. 2017;64(7):1479-1491.
/// DOI: [10.1109/TBME.2016.2609282](https://doi.org/10.1109/TBME.2016.2609282) | PMID: [28113245](https://pubmed.ncbi.nlm.nih.gov/28113245/)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PosAlgorithm;

impl RppgAlgorithm for PosAlgorithm {
    fn id(&self) -> RppgAlgorithmId {
        RppgAlgorithmId::Pos
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

        // 1. Channel normalization: C_n = C / mean(C)
        let r_n: Vec<f64> = optical.red.iter().map(|&r| r / mean_r).collect();
        let g_n: Vec<f64> = optical.green.iter().map(|&g| g / mean_g).collect();
        let b_n: Vec<f64> = optical.blue.iter().map(|&b| b / mean_b).collect();

        // 2. Projection vectors:
        // S1 = G_n - B_n
        // S2 = G_n + B_n - 2 * R_n
        let mut s1_vec = Vec::with_capacity(n);
        let mut s2_vec = Vec::with_capacity(n);

        for i in 0..n {
            let s1 = g_n[i] - b_n[i];
            let s2 = g_n[i] + b_n[i] - 2.0 * r_n[i];
            s1_vec.push(s1);
            s2_vec.push(s2);
        }

        // 3. Standard deviations:
        let mean_s1 = s1_vec.iter().sum::<f64>() / n as f64;
        let mean_s2 = s2_vec.iter().sum::<f64>() / n as f64;

        let var_s1 = s1_vec.iter().map(|v| (v - mean_s1).powi(2)).sum::<f64>() / n as f64;
        let var_s2 = s2_vec.iter().map(|v| (v - mean_s2).powi(2)).sum::<f64>() / n as f64;

        let std_s1 = var_s1.sqrt();
        let std_s2 = var_s2.sqrt();

        // 4. Alpha ratio: alpha = std_S1 / std_S2
        let alpha = if std_s2 > 1e-12 {
            std_s1 / std_s2
        } else if std_s1 <= 1e-12 {
            0.0
        } else {
            return Err(SignalError::NonFiniteInput);
        };

        if !alpha.is_finite() {
            return Err(SignalError::NonFiniteInput);
        }

        // 5. POS pulse projection: P = S1 + alpha * S2
        let pulse: Vec<f64> = s1_vec
            .iter()
            .zip(s2_vec.iter())
            .map(|(&s1, &s2)| s1 + alpha * s2)
            .collect();

        Ok(pulse)
    }
}
