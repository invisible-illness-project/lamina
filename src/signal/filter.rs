use crate::error::{Result, SignalError};
use biquad::{Coefficients, ToHertz, Type};
use ndarray::Array1;
use std::f64::consts::{FRAC_1_SQRT_2, PI};

/// High-level filter type specification for Lamina DSP routines.
#[derive(Debug, Clone, PartialEq)]
pub enum FilterKind {
    LowPass,
    HighPass,
    BandPass,
    Notch,
}

/// Filter specification containing frequency cutoff, sampling rate, order, and filter type.
#[derive(Debug, Clone, PartialEq)]
pub struct FilterSpec {
    pub kind: FilterKind,
    pub sampling_rate: f64,
    pub cutoffs: Vec<f64>,
    pub order: usize,
}

impl FilterSpec {
    /// Create a low-pass filter specification.
    pub fn lowpass(sampling_rate: f64, cutoff: f64, order: usize) -> Self {
        Self {
            kind: FilterKind::LowPass,
            sampling_rate,
            cutoffs: vec![cutoff],
            order,
        }
    }

    /// Create a high-pass filter specification.
    pub fn highpass(sampling_rate: f64, cutoff: f64, order: usize) -> Self {
        Self {
            kind: FilterKind::HighPass,
            sampling_rate,
            cutoffs: vec![cutoff],
            order,
        }
    }

    /// Create a band-pass filter specification.
    pub fn bandpass(sampling_rate: f64, lowcut: f64, highcut: f64, order: usize) -> Self {
        Self {
            kind: FilterKind::BandPass,
            sampling_rate,
            cutoffs: vec![lowcut, highcut],
            order,
        }
    }

    /// Create a notch (band-stop) filter specification.
    pub fn notch(sampling_rate: f64, lowcut: f64, highcut: f64, order: usize) -> Self {
        Self {
            kind: FilterKind::Notch,
            sampling_rate,
            cutoffs: vec![lowcut, highcut],
            order,
        }
    }

    /// Validate the filter specification parameters.
    pub fn validate(&self) -> Result<()> {
        if !self.sampling_rate.is_finite() || self.sampling_rate <= 0.0 {
            return Err(SignalError::InvalidSamplingRate(self.sampling_rate));
        }
        if self.order == 0 {
            return Err(SignalError::InvalidFilterOrder(self.order));
        }
        if self.cutoffs.is_empty() {
            return Err(SignalError::InvalidCutoffFrequency(
                "No cutoff frequencies provided".to_string(),
            ));
        }

        let nyquist = self.sampling_rate / 2.0;

        for &c in &self.cutoffs {
            if !c.is_finite() || c <= 0.0 || c >= nyquist {
                return Err(SignalError::InvalidCutoffFrequency(format!(
                    "Cutoff frequency ({}) must be > 0.0 and strictly below Nyquist frequency ({})",
                    c, nyquist
                )));
            }
        }

        if (self.kind == FilterKind::BandPass || self.kind == FilterKind::Notch)
            && self.cutoffs.len() >= 2
            && self.cutoffs[0] >= self.cutoffs[1]
        {
            return Err(SignalError::InvalidCutoffFrequency(format!(
                "Lowcut frequency ({}) must be strictly less than highcut frequency ({})",
                self.cutoffs[0], self.cutoffs[1]
            )));
        }

        Ok(())
    }
}

/// Second-Order Section (SOS / Biquad) representation: $H(z) = \frac{b_0 + b_1 z^{-1} + b_2 z^{-2}}{1 + a_1 z^{-1} + a_2 z^{-2}}$.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SosSection {
    pub b0: f64,
    pub b1: f64,
    pub b2: f64,
    pub a1: f64,
    pub a2: f64,
}

impl SosSection {
    /// Create a new SOS section from coefficients $[b_0, b_1, b_2, a_1, a_2]$.
    pub fn new(b0: f64, b1: f64, b2: f64, a1: f64, a2: f64) -> Self {
        Self { b0, b1, b2, a1, a2 }
    }

    /// Create an SOS section from a `biquad::Coefficients<f64>`.
    pub fn from_biquad(coeffs: &Coefficients<f64>) -> Self {
        Self {
            b0: coeffs.b0,
            b1: coeffs.b1,
            b2: coeffs.b2,
            a1: coeffs.a1,
            a2: coeffs.a2,
        }
    }

    /// Calculate steady-state initial conditions ($z_{\text{init}}$) for step response in DF2T.
    ///
    /// Matches SciPy `scipy.signal.sosfilt_zi` for 2nd order section.
    pub fn initial_state(&self) -> (f64, f64) {
        let denom = 1.0 + self.a1 + self.a2;
        if denom.abs() < 1e-12 {
            return (0.0, 0.0);
        }
        let z0 = (self.b1 + self.b2 - self.a1 * self.b0 - self.a2 * self.b0) / denom;
        let z1 = (self.b2 + self.a1 * self.b2 - self.a2 * self.b0 - self.a2 * self.b1) / denom;
        (z0, z1)
    }

    /// DC steady-state gain of this section ($H(1) = \frac{b_0 + b_1 + b_2}{1 + a_1 + a_2}$).
    pub fn dc_gain(&self) -> f64 {
        let denom = 1.0 + self.a1 + self.a2;
        if denom.abs() < 1e-12 {
            1.0
        } else {
            (self.b0 + self.b1 + self.b2) / denom
        }
    }
}

/// Cascaded Second-Order Sections (SOS) filter executor.
#[derive(Debug, Clone, PartialEq)]
pub struct SosFilter {
    pub sections: Vec<SosSection>,
}

impl SosFilter {
    /// Create an SOS filter directly from custom SOS sections.
    pub fn from_sections(sections: Vec<SosSection>) -> Self {
        Self { sections }
    }

    /// Design an SOS filter from a high-level [`FilterSpec`].
    pub fn from_spec(spec: &FilterSpec) -> Result<Self> {
        spec.validate()?;

        let fs = spec.sampling_rate;
        let mut sections = Vec::new();

        match spec.kind {
            FilterKind::LowPass => {
                let fc = spec.cutoffs[0];
                let num_biquads = spec.order / 2;
                for k in 0..num_biquads {
                    let q =
                        1.0 / (2.0 * ((2 * k + 1) as f64 * PI / (2.0 * spec.order as f64)).cos());
                    let coeffs =
                        Coefficients::<f64>::from_params(Type::LowPass, fs.hz(), fc.hz(), q)
                            .map_err(|e| SignalError::InvalidCutoffFrequency(format!("{:?}", e)))?;
                    sections.push(SosSection::from_biquad(&coeffs));
                }
                if !spec.order.is_multiple_of(2) {
                    let coeffs = Coefficients::<f64>::from_params(
                        Type::SinglePoleLowPass,
                        fs.hz(),
                        fc.hz(),
                        FRAC_1_SQRT_2,
                    )
                    .map_err(|e| SignalError::InvalidCutoffFrequency(format!("{:?}", e)))?;
                    sections.push(SosSection::from_biquad(&coeffs));
                }
            }
            FilterKind::HighPass => {
                let fc = spec.cutoffs[0];
                let num_biquads = spec.order / 2;
                for k in 0..num_biquads {
                    let q =
                        1.0 / (2.0 * ((2 * k + 1) as f64 * PI / (2.0 * spec.order as f64)).cos());
                    let coeffs =
                        Coefficients::<f64>::from_params(Type::HighPass, fs.hz(), fc.hz(), q)
                            .map_err(|e| SignalError::InvalidCutoffFrequency(format!("{:?}", e)))?;
                    sections.push(SosSection::from_biquad(&coeffs));
                }
                if !spec.order.is_multiple_of(2) {
                    let coeffs = Coefficients::<f64>::from_params(
                        Type::HighPass,
                        fs.hz(),
                        fc.hz(),
                        FRAC_1_SQRT_2,
                    )
                    .map_err(|e| SignalError::InvalidCutoffFrequency(format!("{:?}", e)))?;
                    sections.push(SosSection::from_biquad(&coeffs));
                }
            }
            FilterKind::BandPass => {
                let f_low = spec.cutoffs[0];
                let f_high = spec.cutoffs[1];
                let f0 = (f_low * f_high).sqrt();
                let bw = f_high - f_low;
                let q_base = f0 / bw;

                let num_biquads = spec.order;
                for k in 0..num_biquads {
                    let q_factor = q_base
                        / (2.0
                            * ((2 * k + 1) as f64 * PI / (2.0 * (2 * num_biquads) as f64)).cos());
                    let coeffs = Coefficients::<f64>::from_params(
                        Type::BandPass,
                        fs.hz(),
                        f0.hz(),
                        q_factor,
                    )
                    .map_err(|e| SignalError::InvalidCutoffFrequency(format!("{:?}", e)))?;
                    sections.push(SosSection::from_biquad(&coeffs));
                }
            }
            FilterKind::Notch => {
                let f_low = spec.cutoffs[0];
                let f_high = spec.cutoffs[1];
                let f0 = (f_low * f_high).sqrt();
                let bw = f_high - f_low;
                let q_base = f0 / bw;

                let coeffs =
                    Coefficients::<f64>::from_params(Type::Notch, fs.hz(), f0.hz(), q_base)
                        .map_err(|e| SignalError::InvalidCutoffFrequency(format!("{:?}", e)))?;
                sections.push(SosSection::from_biquad(&coeffs));
            }
        }

        Ok(Self { sections })
    }

    /// Single-pass forward filtering over a 1D signal using Direct Form II Transposed (DF2T).
    pub fn forward(&self, signal: &Array1<f64>) -> Result<Array1<f64>> {
        let n = signal.len();
        if n == 0 {
            return Err(SignalError::EmptySignal);
        }
        for &v in signal.iter() {
            if !v.is_finite() {
                return Err(SignalError::NonFiniteInput);
            }
        }

        let mut out = signal.to_vec();
        let mut states: Vec<(f64, f64)> = self.sections.iter().map(|s| s.initial_state()).collect();

        // Scale initial states by initial sample x[0] and cascaded DC gains (matching SciPy sosfilt_zi)
        let mut cur_scale = out[0];
        for (st, sec) in states.iter_mut().zip(self.sections.iter()) {
            st.0 *= cur_scale;
            st.1 *= cur_scale;
            cur_scale *= sec.dc_gain();
        }

        for sample in out.iter_mut() {
            let mut x = *sample;
            for (sec, st) in self.sections.iter().zip(states.iter_mut()) {
                let y = sec.b0 * x + st.0;
                st.0 = sec.b1 * x - sec.a1 * y + st.1;
                st.1 = sec.b2 * x - sec.a2 * y;
                x = y;
            }
            *sample = x;
        }

        Ok(Array1::from_vec(out))
    }

    /// Zero-phase forward-backward filtering (`filtfilt`) with SciPy-style odd reflection padding (`padtype='odd'`).
    ///
    /// # Scientific Contract
    /// - **Inputs**: 1D signal array `Array1<f64>`.
    /// - **Output**: Filtered array of identical length $N$.
    /// - **Padding**: Odd-reflection padding length $L = 3 \times (2 \cdot n_{\text{sections}} + 1)$.
    ///   - Left: $\text{padded}[L - i] = 2 \cdot x[0] - x[i]$ for $i=1 \dots L$.
    ///   - Right: $\text{padded}[L + N - 1 + i] = 2 \cdot x[N - 1] - x[N - 1 - i]$ for $i=1 \dots L$.
    /// - **Initial Conditions**: Filter states are initialized using steady-state $z_{\text{init}}$ scaled by the first sample of each pass.
    ///
    /// # Errors
    /// Returns [`SignalError::InsufficientSamples`] if $N \le L$.
    pub fn filtfilt(&self, signal: &Array1<f64>) -> Result<Array1<f64>> {
        let n = signal.len();
        if n == 0 {
            return Err(SignalError::EmptySignal);
        }
        for &v in signal.iter() {
            if !v.is_finite() {
                return Err(SignalError::NonFiniteInput);
            }
        }

        let padlen = 3 * (2 * self.sections.len() + 1);
        if n <= padlen {
            return Err(SignalError::InsufficientSamples {
                required: padlen + 1,
                provided: n,
            });
        }

        // Construct odd-reflected padded array
        let total_len = n + 2 * padlen;
        let mut padded = vec![0.0; total_len];

        // Copy original signal into center
        for i in 0..n {
            padded[padlen + i] = signal[i];
        }

        // Left odd reflection
        let x0 = signal[0];
        for i in 1..=padlen {
            padded[padlen - i] = 2.0 * x0 - signal[i];
        }

        // Right odd reflection
        let x_end = signal[n - 1];
        for i in 1..=padlen {
            padded[padlen + n - 1 + i] = 2.0 * x_end - signal[n - 1 - i];
        }

        // Forward Pass
        let mut fwd_states: Vec<(f64, f64)> =
            self.sections.iter().map(|s| s.initial_state()).collect();
        let mut cur_scale = padded[0];
        for (st, sec) in fwd_states.iter_mut().zip(self.sections.iter()) {
            st.0 *= cur_scale;
            st.1 *= cur_scale;
            cur_scale *= sec.dc_gain();
        }

        let mut fwd_out = padded.clone();
        for sample in fwd_out.iter_mut() {
            let mut x = *sample;
            for (sec, st) in self.sections.iter().zip(fwd_states.iter_mut()) {
                let y = sec.b0 * x + st.0;
                st.0 = sec.b1 * x - sec.a1 * y + st.1;
                st.1 = sec.b2 * x - sec.a2 * y;
                x = y;
            }
            *sample = x;
        }

        // Backward Pass (Reverse fwd_out, filter, reverse back)
        fwd_out.reverse();

        let mut bwd_states: Vec<(f64, f64)> =
            self.sections.iter().map(|s| s.initial_state()).collect();
        let mut cur_bwd_scale = fwd_out[0];
        for (st, sec) in bwd_states.iter_mut().zip(self.sections.iter()) {
            st.0 *= cur_bwd_scale;
            st.1 *= cur_bwd_scale;
            cur_bwd_scale *= sec.dc_gain();
        }

        let mut bwd_out = fwd_out;
        for sample in bwd_out.iter_mut() {
            let mut x = *sample;
            for (sec, st) in self.sections.iter().zip(bwd_states.iter_mut()) {
                let y = sec.b0 * x + st.0;
                st.0 = sec.b1 * x - sec.a1 * y + st.1;
                st.1 = sec.b2 * x - sec.a2 * y;
                x = y;
            }
            *sample = x;
        }

        bwd_out.reverse();

        // Extract signal unpadded portion
        let output = bwd_out[padlen..(padlen + n)].to_vec();
        Ok(Array1::from_vec(output))
    }
}

/// Zero-phase digital filtering (`signal_filtfilt`) using Second-Order Sections (SOS) and odd reflection padding.
///
/// This is the primary zero-phase filtering entry point for Lamina.
///
/// # Errors
/// Returns [`SignalError`] if input parameters are invalid or signal is too short for padding.
pub fn signal_filtfilt(signal: &Array1<f64>, spec: &FilterSpec) -> Result<Array1<f64>> {
    let filter = SosFilter::from_spec(spec)?;
    filter.filtfilt(signal)
}

/// Convenience entry point for digital filtering, constructing a [`FilterSpec`] and running [`signal_filtfilt`].
///
/// Backwards-compatible signature for Lamina signal functions.
pub fn signal_filter(
    signal: &Array1<f64>,
    sampling_rate: f64,
    lowcut: Option<f64>,
    highcut: Option<f64>,
    order: usize,
) -> Result<Array1<f64>> {
    let spec = match (lowcut, highcut) {
        (Some(lc), Some(hc)) => FilterSpec::bandpass(sampling_rate, lc, hc, order),
        (Some(lc), None) => FilterSpec::highpass(sampling_rate, lc, order),
        (None, Some(hc)) => FilterSpec::lowpass(sampling_rate, hc, order),
        (None, None) => {
            return Err(SignalError::InvalidCutoffFrequency(
                "Must specify at least lowcut or highcut frequency".to_string(),
            ));
        }
    };

    signal_filtfilt(signal, &spec)
}
