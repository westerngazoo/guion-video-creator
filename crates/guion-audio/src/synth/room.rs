//! An algorithmic room: reverb by convolution with a synthesized impulse
//! response (R-0008 AC6).
//!
//! The response starts as seeded Gaussian noise under an exponential decay of
//! `−60 dB per t60`, and is then made **level-stable**: a plain noise response
//! has a random gain at every single frequency (its power is exponentially
//! distributed, so a sustained tone can come out 20 dB down or 8 dB up
//! depending on the seed; measured, a 55 Hz drone moved 4.5 dB between two
//! seeds). Alternating projections fix it: the spectrum is set to the target
//! magnitude (a low-pass at `damping` Hz, order 2) keeping its phases, then the
//! local level (10 ms) is set back to the decay, [`FLATTEN`] times. The result
//! decays exactly as asked and passes any steady tone with a gain within about
//! a decibel of the target, whatever the seed. Normalised to unit energy, the
//! send level alone sets how much room is heard. Nothing is recorded: the same
//! `(t60, length, damping, seed)` gives the same room.

use crate::error::AudioError;
use crate::synth::fft::{convolve, irfft, next_pow2, rfft};
use crate::synth::filter::lowpass_gain;
use crate::synth::noise::Noise;

/// Rounds of the alternating projections (magnitude, then decay).
pub const FLATTEN: usize = 12;

/// A room's parameters.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Room {
    /// Seconds for the tail to fall 60 dB.
    pub t60: f64,
    /// Length of the impulse response, seconds.
    pub length: f64,
    /// Low-pass on the tail (order 2), Hz.
    pub damping: f64,
    /// The noise stream of the response.
    pub seed: u64,
}

impl Room {
    /// A room, checked: every time and frequency must be finite and positive
    /// (AC12: the error names the parameter).
    pub fn new(t60: f64, length: f64, damping: f64, seed: u64) -> Result<Room, AudioError> {
        for (name, v) in [("t60", t60), ("length", length), ("damping", damping)] {
            if !(v.is_finite() && v > 0.0) {
                return Err(AudioError::InvalidSynth {
                    what: format!("room.{name}"),
                    message: format!("must be finite and > 0, got {v}"),
                });
            }
        }
        Ok(Room {
            t60,
            length,
            damping,
            seed,
        })
    }

    /// The impulse response at `rate`, with unit energy (`Σ h² = 1`).
    pub fn impulse_response(&self, rate: f64) -> Vec<f64> {
        let m = (self.length * rate).round() as usize;
        if m == 0 {
            return Vec::new();
        }
        let mut g = Noise::new(self.seed);
        let k = -3.0 * std::f64::consts::LN_10 / (self.t60 * rate);
        let env: Vec<f64> = (0..m).map(|i| (k * i as f64).exp()).collect();
        let mut h: Vec<f64> = env.iter().map(|e| g.gaussian() * e).collect();
        // twice the length: the spectrum is sampled finely enough that a tone
        // between two bins sees the same gain
        let n = 2 * next_pow2(m);
        let target: Vec<f64> = (0..=n / 2)
            .map(|b| {
                if b == 0 {
                    0.0
                } else {
                    lowpass_gain(b as f64 * rate / n as f64, self.damping, 2)
                }
            })
            .collect();
        let frame = ((0.01 * rate).round() as usize).max(1);
        for _ in 0..FLATTEN {
            let mut spec = rfft(&h, n);
            for (c, t) in spec.iter_mut().zip(&target) {
                let a = (c.re * c.re + c.im * c.im).sqrt();
                *c = if a > 0.0 {
                    crate::synth::fft::Complex::new(c.re * t / a, c.im * t / a)
                } else {
                    crate::synth::fft::Complex::default()
                };
            }
            let mut y = irfft(&spec, n);
            y.truncate(m);
            let level = local_rms(&y, frame);
            h = y
                .iter()
                .zip(&level)
                .zip(&env)
                .map(|((v, l), e)| if *l > 0.0 { v * e / l } else { 0.0 })
                .collect();
        }
        let e: f64 = h.iter().map(|v| v * v).sum::<f64>().sqrt();
        if e > 0.0 {
            h.iter_mut().for_each(|v| *v /= e);
        }
        h
    }

    /// `x` heard in the room (wet only), `x.len()` samples. To hear several
    /// signals in one room, build the response once and use [`convolve`].
    pub fn apply(&self, x: &[f64], rate: f64) -> Vec<f64> {
        convolve(x, &self.impulse_response(rate))
    }
}

/// The RMS over a centred window of `frame` samples (zeros outside), by
/// prefix sums in a fixed order.
fn local_rms(x: &[f64], frame: usize) -> Vec<f64> {
    let n = x.len();
    let mut pre = Vec::with_capacity(n + 1);
    pre.push(0.0);
    let mut acc = 0.0;
    for v in x {
        acc += v * v;
        pre.push(acc);
    }
    let h = (frame - 1) / 2;
    (0..n)
        .map(|i| {
            let a = i.saturating_sub(frame - 1 - h);
            let b = (i + h + 1).min(n);
            ((pre[b] - pre[a]).max(0.0) / frame as f64).sqrt()
        })
        .collect()
}
