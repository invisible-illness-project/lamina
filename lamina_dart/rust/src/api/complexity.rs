use crate::api::error::SignalError;
use lamina::complexity::entropy::sample_entropy as core_sample_entropy;
use ndarray::Array1;

pub fn process_sample_entropy(signal: Vec<f64>, m: usize, r: f64) -> Result<f64, SignalError> {
    let nd = Array1::from_vec(signal);
    let val = core_sample_entropy(&nd, m, r)?;
    Ok(val)
}
