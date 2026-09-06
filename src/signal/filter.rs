use crate::error::{Result, SignalError};
use biquad::Coefficients;
use ndarray::Array1;
use std::f64::consts::PI;

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

#[derive(Debug, Clone, Copy, PartialEq)]
struct Complex64 {
    re: f64,
    im: f64,
}

impl Complex64 {
    fn new(re: f64, im: f64) -> Self {
        Self { re, im }
    }

    fn from_polar(r: f64, theta: f64) -> Self {
        Self {
            re: r * theta.cos(),
            im: r * theta.sin(),
        }
    }

    fn abs(&self) -> f64 {
        self.re.hypot(self.im)
    }

    fn conj(&self) -> Self {
        Self {
            re: self.re,
            im: -self.im,
        }
    }

    fn sqrt(&self) -> Self {
        let r = self.abs();
        let theta = self.im.atan2(self.re);
        Self::from_polar(r.sqrt(), theta / 2.0)
    }
}

impl std::ops::Add for Complex64 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self::new(self.re + rhs.re, self.im + rhs.im)
    }
}

impl std::ops::Sub for Complex64 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self::new(self.re - rhs.re, self.im - rhs.im)
    }
}

impl std::ops::Mul for Complex64 {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self {
        Self::new(
            self.re * rhs.re - self.im * rhs.im,
            self.re * rhs.im + self.im * rhs.re,
        )
    }
}

impl std::ops::Mul<f64> for Complex64 {
    type Output = Self;
    fn mul(self, rhs: f64) -> Self {
        Self::new(self.re * rhs, self.im * rhs)
    }
}

impl std::ops::Div for Complex64 {
    type Output = Self;
    fn div(self, rhs: Self) -> Self {
        let denom = rhs.re * rhs.re + rhs.im * rhs.im;
        Self::new(
            (self.re * rhs.re + self.im * rhs.im) / denom,
            (self.im * rhs.re - self.re * rhs.im) / denom,
        )
    }
}

impl std::ops::Div<f64> for Complex64 {
    type Output = Self;
    fn div(self, rhs: f64) -> Self {
        Self::new(self.re / rhs, self.im / rhs)
    }
}

impl std::ops::Neg for Complex64 {
    type Output = Self;
    fn neg(self) -> Self {
        Self::new(-self.re, -self.im)
    }
}

/// Design a digital Butterworth filter SOS matrix matching SciPy `scipy.signal.butter(..., output='sos')`.
pub fn design_butterworth_sos(spec: &FilterSpec) -> Result<Vec<SosSection>> {
    spec.validate()?;

    let order = spec.order;
    let fs = spec.sampling_rate;

    // 1. Analog lowpass prototype poles: p_k = exp(j * pi * (2*k + 1 + order) / (2 * order))
    let mut p_proto = Vec::with_capacity(order);
    for k in 0..order {
        let angle = PI * ((2 * k + 1 + order) as f64) / (2.0 * order as f64);
        p_proto.push(Complex64::from_polar(1.0, angle));
    }

    // 2. Frequency pre-warping & analog s-plane transformation
    let (p_s, z_s, k_s) = match spec.kind {
        FilterKind::LowPass => {
            let fc = spec.cutoffs[0];
            let wp = 2.0 * fs * (PI * fc / fs).tan();
            let p_s: Vec<Complex64> = p_proto.iter().map(|&p| p * wp).collect();
            let z_s: Vec<Complex64> = Vec::new();
            let k_s = wp.powi(order as i32);
            (p_s, z_s, k_s)
        }
        FilterKind::HighPass => {
            let fc = spec.cutoffs[0];
            let wp = 2.0 * fs * (PI * fc / fs).tan();
            let p_s: Vec<Complex64> = p_proto
                .iter()
                .map(|&p| Complex64::new(wp, 0.0) / p)
                .collect();
            let z_s: Vec<Complex64> = vec![Complex64::new(0.0, 0.0); order];
            let mut prod_neg_p = Complex64::new(1.0, 0.0);
            for &p in &p_proto {
                prod_neg_p = prod_neg_p * (-p);
            }
            let k_s = prod_neg_p.re;
            (p_s, z_s, k_s)
        }
        FilterKind::BandPass => {
            let f1 = spec.cutoffs[0];
            let f2 = spec.cutoffs[1];
            let w1 = 2.0 * fs * (PI * f1 / fs).tan();
            let w2 = 2.0 * fs * (PI * f2 / fs).tan();
            let w0 = (w1 * w2).sqrt();
            let bw = w2 - w1;

            let mut p_s = Vec::with_capacity(2 * order);
            for &pk in &p_proto {
                let term1 = pk * pk * (bw * bw);
                let disc = (term1 - Complex64::new(4.0 * w0 * w0, 0.0)).sqrt();
                p_s.push((pk * bw + disc) / 2.0);
                p_s.push((pk * bw - disc) / 2.0);
            }
            let z_s = vec![Complex64::new(0.0, 0.0); order];
            let k_s = bw.powi(order as i32);
            (p_s, z_s, k_s)
        }
        FilterKind::Notch => {
            let f1 = spec.cutoffs[0];
            let f2 = spec.cutoffs[1];
            let w1 = 2.0 * fs * (PI * f1 / fs).tan();
            let w2 = 2.0 * fs * (PI * f2 / fs).tan();
            let w0 = (w1 * w2).sqrt();
            let bw = w2 - w1;

            let mut p_s = Vec::with_capacity(2 * order);
            for &pk in &p_proto {
                let term1 = (Complex64::new(bw, 0.0) / pk) * (Complex64::new(bw, 0.0) / pk);
                let disc = (term1 - Complex64::new(4.0 * w0 * w0, 0.0)).sqrt();
                p_s.push((Complex64::new(bw, 0.0) / pk + disc) / 2.0);
                p_s.push((Complex64::new(bw, 0.0) / pk - disc) / 2.0);
            }
            let mut z_s = Vec::with_capacity(2 * order);
            for _ in 0..order {
                z_s.push(Complex64::new(0.0, w0));
                z_s.push(Complex64::new(0.0, -w0));
            }
            let mut prod_neg_p = Complex64::new(1.0, 0.0);
            for &p in &p_proto {
                prod_neg_p = prod_neg_p * (-p);
            }
            let k_s = prod_neg_p.re;
            (p_s, z_s, k_s)
        }
    };

    // 3. Bilinear Transformation (fs_b = 2 * fs)
    let fs_b = 2.0 * fs;
    let fs_c = Complex64::new(fs_b, 0.0);

    let mut p_z: Vec<Complex64> = p_s.iter().map(|&ps| (fs_c + ps) / (fs_c - ps)).collect();
    let mut z_z: Vec<Complex64> = z_s.iter().map(|&zs| (fs_c + zs) / (fs_c - zs)).collect();

    let num_neg1 = p_s.len().saturating_sub(z_s.len());
    for _ in 0..num_neg1 {
        z_z.push(Complex64::new(-1.0, 0.0));
    }

    let mut prod_z = Complex64::new(1.0, 0.0);
    for &zs in &z_s {
        prod_z = prod_z * (fs_c - zs);
    }
    let mut prod_p = Complex64::new(1.0, 0.0);
    for &ps in &p_s {
        prod_p = prod_p * (fs_c - ps);
    }
    let k_z = (Complex64::new(k_s, 0.0) * prod_z / prod_p).re;

    // 4. Convert z, p, k to SOS matrix matching SciPy zpk2sos (pairing='nearest')
    let n_sections = p_z.len().max(z_z.len()).div_ceil(2);
    if p_z.len() < n_sections * 2 {
        p_z.resize(n_sections * 2, Complex64::new(0.0, 0.0));
    }
    if z_z.len() < n_sections * 2 {
        z_z.resize(n_sections * 2, Complex64::new(0.0, 0.0));
    }

    let mut sections = vec![SosSection::new(0.0, 0.0, 0.0, 0.0, 0.0); n_sections];

    let idx_worst = |p_list: &[Complex64]| -> usize {
        let mut best_i = 0;
        let mut best_d = (1.0 - p_list[0].abs()).abs();
        for (i, p) in p_list.iter().enumerate().skip(1) {
            let d = (1.0 - p.abs()).abs();
            if d < best_d {
                best_d = d;
                best_i = i;
            }
        }
        best_i
    };

    let idx_nearest_zero =
        |z_list: &[Complex64], target_p: Complex64, kind: &str| -> Option<usize> {
            let mut best_i = None;
            let mut best_d = f64::MAX;
            for (i, &zv) in z_list.iter().enumerate() {
                let is_cplx = zv.im.abs() > 1e-12;
                if kind == "complex" && !is_cplx {
                    continue;
                }
                if kind == "real" && is_cplx {
                    continue;
                }
                let d = (zv - target_p).abs();
                if d < best_d {
                    best_d = d;
                    best_i = Some(i);
                }
            }
            best_i
        };

    for si in (0..n_sections).rev() {
        let p1_idx = idx_worst(&p_z);
        let p1 = p_z.remove(p1_idx);

        let is_p1_real = p1.im.abs() <= 1e-12;
        let num_real_p = p_z.iter().filter(|pv| pv.im.abs() <= 1e-12).count();

        if is_p1_real && num_real_p == 0 {
            let z1_idx = idx_nearest_zero(&z_z, p1, "real")
                .or_else(|| idx_nearest_zero(&z_z, p1, "any"))
                .unwrap_or(0);
            let z1 = z_z.remove(z1_idx);

            sections[si] = SosSection::new(1.0, -z1.re, 0.0, -p1.re, 0.0);
        } else {
            let p2 = if is_p1_real {
                let real_p_indices: Vec<usize> = p_z
                    .iter()
                    .enumerate()
                    .filter(|(_, pv)| pv.im.abs() <= 1e-12)
                    .map(|(i, _)| i)
                    .collect();
                let worst_sub =
                    idx_worst(&real_p_indices.iter().map(|&i| p_z[i]).collect::<Vec<_>>());
                let p2_idx = real_p_indices[worst_sub];
                p_z.remove(p2_idx)
            } else {
                let mut best_conj_i = 0;
                let mut best_conj_d = f64::MAX;
                for (i, &pv) in p_z.iter().enumerate() {
                    let d = (pv - p1.conj()).abs();
                    if d < best_conj_d {
                        best_conj_d = d;
                        best_conj_i = i;
                    }
                }
                p_z.remove(best_conj_i)
            };

            let a1 = -(p1 + p2).re;
            let a2 = (p1 * p2).re;

            let z1_idx = idx_nearest_zero(&z_z, p1, "any").unwrap_or(0);
            let z1 = z_z.remove(z1_idx);

            let z2 = if z1.im.abs() > 1e-12 {
                let mut best_z_conj_i = 0;
                let mut best_z_conj_d = f64::MAX;
                for (i, &zv) in z_z.iter().enumerate() {
                    let d = (zv - z1.conj()).abs();
                    if d < best_z_conj_d {
                        best_z_conj_d = d;
                        best_z_conj_i = i;
                    }
                }
                z_z.remove(best_z_conj_i)
            } else {
                if let Some(z2_idx) = idx_nearest_zero(&z_z, p1, "real") {
                    z_z.remove(z2_idx)
                } else {
                    Complex64::new(0.0, 0.0)
                }
            };

            let b0 = 1.0;
            let b1 = -(z1 + z2).re;
            let b2 = (z1 * z2).re;

            sections[si] = SosSection::new(b0, b1, b2, a1, a2);
        }
    }

    sections[0].b0 *= k_z;
    sections[0].b1 *= k_z;
    sections[0].b2 *= k_z;

    Ok(sections)
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
        let sections = design_butterworth_sos(spec)?;
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

        let zero_b2 = self.sections.iter().filter(|s| s.b2 == 0.0).count();
        let zero_a2 = self.sections.iter().filter(|s| s.a2 == 0.0).count();
        let ntaps = (2 * self.sections.len() + 1).saturating_sub(zero_b2.min(zero_a2));
        let padlen = 3 * ntaps;
        if n <= padlen {
            return Err(SignalError::InsufficientSamples {
                required: padlen + 1,
                provided: n,
            });
        }

        let total_len = n + 2 * padlen;
        let mut padded = vec![0.0; total_len];

        for i in 0..n {
            padded[padlen + i] = signal[i];
        }

        let x0 = signal[0];
        for i in 1..=padlen {
            padded[padlen - i] = 2.0 * x0 - signal[i];
        }

        let x_end = signal[n - 1];
        for i in 1..=padlen {
            padded[padlen + n - 1 + i] = 2.0 * x_end - signal[n - 1 - i];
        }

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

        let output = bwd_out[padlen..(padlen + n)].to_vec();
        Ok(Array1::from_vec(output))
    }
}

/// Zero-phase digital filtering (`signal_filtfilt`) using Second-Order Sections (SOS) and odd reflection padding.
pub fn signal_filtfilt(signal: &Array1<f64>, spec: &FilterSpec) -> Result<Array1<f64>> {
    let filter = SosFilter::from_spec(spec)?;
    filter.filtfilt(signal)
}

/// Convenience entry point for digital filtering, constructing a [`FilterSpec`] and running [`signal_filtfilt`].
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
