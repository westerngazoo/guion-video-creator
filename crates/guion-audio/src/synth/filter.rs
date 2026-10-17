//! Filters (R-0008 AC5).
//!
//! - **Zero-phase spectral filters** ([`lowpass`], [`highpass`], [`bandpass`]):
//!   the spectrum is multiplied by a closed-form magnitude, so a sine at `f`
//!   comes out scaled by exactly `|H(f)|` and not shifted in time. They are
//!   the studio's sound (a Butterworth magnitude of order `n`, and the
//!   resonance `1/√(1 + Q²(f/f₀ − f₀/f)²)`), and they see the whole signal
//!   at once, so they suit rendered events, not live streams.
//! - **One-pole filters** ([`OnePole`]): causal and cheap, for control
//!   signals and envelope followers.

use std::f64::consts::TAU;

use crate::synth::fft::fft_filter;

/// `|H(f)|` of a Butterworth low-pass of `order` at cutoff `fc`.
pub fn lowpass_gain(f: f64, fc: f64, order: u32) -> f64 {
    1.0 / (1.0 + (f / fc).powi(2 * order as i32)).sqrt()
}

/// `|H(f)|` of a Butterworth high-pass of `order` at cutoff `fc` (0 at DC).
pub fn highpass_gain(f: f64, fc: f64, order: u32) -> f64 {
    1.0 / (1.0 + (fc / f.max(1e-9)).powi(2 * order as i32)).sqrt()
}

/// `|H(f)|` of a resonant band-pass centred on `f0` with quality `q`
/// (1 at `f0`, 0 at DC).
pub fn bandpass_gain(f: f64, f0: f64, q: f64) -> f64 {
    if f <= 0.0 {
        return 0.0;
    }
    let d = f / f0 - f0 / f;
    1.0 / (1.0 + q * q * d * d).sqrt()
}

/// Zero-phase Butterworth low-pass magnitude.
pub fn lowpass(x: &[f64], rate: f64, fc: f64, order: u32) -> Vec<f64> {
    fft_filter(x, rate, |f| lowpass_gain(f, fc, order))
}

/// Zero-phase Butterworth high-pass magnitude.
pub fn highpass(x: &[f64], rate: f64, fc: f64, order: u32) -> Vec<f64> {
    fft_filter(x, rate, |f| highpass_gain(f, fc, order))
}

/// Zero-phase resonant band-pass.
pub fn bandpass(x: &[f64], rate: f64, f0: f64, q: f64) -> Vec<f64> {
    fft_filter(x, rate, |f| bandpass_gain(f, f0, q))
}

/// A causal one-pole filter, `y[i] = a·x[i] + (1 − a)·y[i−1]`, with
/// `a = 1 − e^{−2π fc / rate}`. Its magnitude in closed form is
/// `a / |1 − (1 − a) e^{−iω}|`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OnePole {
    pub a: f64,
}

impl OnePole {
    /// The one-pole with cutoff `fc` Hz.
    pub fn new(rate: f64, fc: f64) -> Self {
        OnePole {
            a: 1.0 - (-TAU * fc / rate).exp(),
        }
    }

    /// The closed-form magnitude at `f` Hz.
    pub fn gain(&self, rate: f64, f: f64) -> f64 {
        let w = TAU * f / rate;
        let b = 1.0 - self.a;
        let re = 1.0 - b * w.cos();
        let im = b * w.sin();
        self.a / (re * re + im * im).sqrt()
    }

    /// The low-pass output (starts from rest).
    pub fn lowpass(&self, x: &[f64]) -> Vec<f64> {
        let mut y = 0.0;
        x.iter()
            .map(|v| {
                y += self.a * (v - y);
                y
            })
            .collect()
    }

    /// The high-pass output: the input minus its low-pass.
    pub fn highpass(&self, x: &[f64]) -> Vec<f64> {
        let lp = self.lowpass(x);
        x.iter().zip(lp).map(|(v, l)| v - l).collect()
    }
}
