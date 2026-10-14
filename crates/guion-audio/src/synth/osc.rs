//! Oscillators: sines, sums of partials, and phase from a frequency track
//! (R-0008 AC2).
//!
//! A long sine (a 90 s drone has 4.3 M samples per partial) is computed by a
//! rotating phasor, resynchronised to the exact `sin`/`cos` every
//! [`RESYNC`] samples so the error never accumulates past a few ulps.

use std::f64::consts::TAU;

/// Samples between exact resynchronisations of the phasor.
pub const RESYNC: usize = 1024;

/// Adds `amp · sin(2π f i / rate + phase)` to `out[i]`.
pub fn add_sine(out: &mut [f64], rate: f64, freq: f64, phase: f64, amp: f64) {
    let w = TAU * freq / rate;
    let (sw, cw) = w.sin_cos();
    let mut i0 = 0;
    while i0 < out.len() {
        // the exact phase at the start of the block, reduced to whole cycles first
        let cycles = (freq * i0 as f64 / rate).fract();
        let (mut s, mut c) = (TAU * cycles + phase).sin_cos();
        let end = (i0 + RESYNC).min(out.len());
        for o in &mut out[i0..end] {
            *o += amp * s;
            let s2 = s * cw + c * sw;
            c = c * cw - s * sw;
            s = s2;
        }
        i0 = end;
    }
}

/// `n` samples of `amp · sin(2π f t + phase)`.
pub fn sine(n: usize, rate: f64, freq: f64, phase: f64, amp: f64) -> Vec<f64> {
    let mut out = vec![0.0; n];
    add_sine(&mut out, rate, freq, phase, amp);
    out
}

/// One partial of a sum of partials: frequency `ratio · f0`, amplitude
/// `amp`, starting phase `phase`, and an exponential decay with time
/// constant `decay` seconds (`None` sustains).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Partial {
    pub ratio: f64,
    pub amp: f64,
    pub phase: f64,
    pub decay: Option<f64>,
}

impl Partial {
    /// A sustained partial with phase 0.
    pub const fn new(ratio: f64, amp: f64) -> Self {
        Partial {
            ratio,
            amp,
            phase: 0.0,
            decay: None,
        }
    }
    /// The same partial, decaying with time constant `tau` seconds.
    pub const fn decaying(self, tau: f64) -> Self {
        Partial {
            decay: Some(tau),
            ..self
        }
    }
    /// The same partial with starting phase `phase` radians.
    pub const fn with_phase(self, phase: f64) -> Self {
        Partial { phase, ..self }
    }
}

/// `n` samples of `Σ amp_k · sin(2π ratio_k f0 t + phase_k) · e^{−t/τ_k}`.
pub fn partials(n: usize, rate: f64, f0: f64, parts: &[Partial]) -> Vec<f64> {
    let mut out = vec![0.0; n];
    let mut one = vec![0.0; n];
    for p in parts {
        match p.decay {
            None => add_sine(&mut out, rate, f0 * p.ratio, p.phase, p.amp),
            Some(tau) => {
                one.iter_mut().for_each(|x| *x = 0.0);
                add_sine(&mut one, rate, f0 * p.ratio, p.phase, p.amp);
                let e = crate::synth::env::exp_decay(n, rate, tau);
                for ((o, s), g) in out.iter_mut().zip(&one).zip(&e) {
                    *o += s * g;
                }
            }
        }
    }
    out
}

/// The running phase of a frequency track: `φ[i] = 2π Σ_{j≤i} f[j] / rate`
/// (radians). A chirp or a pitch that follows the picture is `sin(φ)`.
pub fn phase_of(freqs: &[f64], rate: f64) -> Vec<f64> {
    let mut acc = 0.0;
    freqs
        .iter()
        .map(|f| {
            acc += f;
            TAU * acc / rate
        })
        .collect()
}
