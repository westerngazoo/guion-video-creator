//! Stereo buffers, constant-power pan and mixing buses with a room send
//! (R-0008 AC7, AC8).

use std::f64::consts::FRAC_PI_4;

use crate::synth::fft::convolve;

/// Left and right gains for `pan` in `[−1, 1]` (−1 left, 0 centre, +1
/// right): `cos θ` and `sin θ` with `θ = (pan + 1)·π/4`, so `L² + R² = 1`.
/// Values outside the range are clamped.
pub fn pan_gains(pan: f64) -> (f64, f64) {
    let a = (pan.clamp(-1.0, 1.0) + 1.0) * FRAC_PI_4;
    (a.cos(), a.sin())
}

/// Where a sound sits: a fixed pan, or one per sample of the sound.
#[derive(Debug, Clone, Copy)]
pub enum Pan<'a> {
    Fixed(f64),
    /// One pan per sample of the placed signal (the last one holds).
    Track(&'a [f64]),
}

/// Two channels of equal length.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Stereo {
    pub l: Vec<f64>,
    pub r: Vec<f64>,
}

impl Stereo {
    /// `n` samples of silence.
    pub fn silence(n: usize) -> Self {
        Stereo {
            l: vec![0.0; n],
            r: vec![0.0; n],
        }
    }

    /// The same mono signal on both channels.
    pub fn dual(x: &[f64]) -> Self {
        Stereo {
            l: x.to_vec(),
            r: x.to_vec(),
        }
    }

    /// Samples per channel.
    pub fn len(&self) -> usize {
        self.l.len()
    }

    /// No samples.
    pub fn is_empty(&self) -> bool {
        self.l.is_empty()
    }

    /// The largest absolute sample of either channel.
    pub fn peak(&self) -> f64 {
        self.l
            .iter()
            .chain(&self.r)
            .fold(0.0_f64, |m, v| m.max(v.abs()))
    }

    /// Multiplies both channels by `k`.
    pub fn scale(&mut self, k: f64) {
        self.l
            .iter_mut()
            .chain(self.r.iter_mut())
            .for_each(|v| *v *= k);
    }

    /// Multiplies both channels by a gain curve (one value per sample).
    pub fn apply_gain(&mut self, g: &[f64]) {
        for (i, k) in g.iter().enumerate().take(self.len()) {
            self.l[i] *= k;
            self.r[i] *= k;
        }
    }

    /// Adds `k · other` sample by sample (up to the shorter length).
    pub fn mix(&mut self, other: &Stereo, k: f64) {
        for (a, b) in self.l.iter_mut().zip(&other.l) {
            *a += k * b;
        }
        for (a, b) in self.r.iter_mut().zip(&other.r) {
            *a += k * b;
        }
    }

    /// Adds `gain · sig` from sample `at`, panned with constant power.
    /// A negative start cuts the beginning of `sig`; whatever runs past the
    /// end is cut too.
    pub fn place(&mut self, sig: &[f64], at: i64, gain: f64, pan: Pan) {
        let n = self.len();
        let (left, right) = (&mut self.l, &mut self.r);
        place(
            sig,
            at,
            gain,
            pan,
            |i, l, r| {
                left[i] += l;
                right[i] += r;
            },
            n,
        );
    }
}

/// Walks the placed, panned samples of `sig`: `put(index, left, right)`.
fn place(
    sig: &[f64],
    at: i64,
    gain: f64,
    pan: Pan,
    mut put: impl FnMut(usize, f64, f64),
    n: usize,
) {
    let skip = (-at).max(0) as usize;
    let start = at.max(0) as usize;
    if start >= n || skip >= sig.len() {
        return;
    }
    let count = (sig.len() - skip).min(n - start);
    for j in 0..count {
        let s = sig[skip + j] * gain;
        let (gl, gr) = match pan {
            Pan::Fixed(p) => pan_gains(p),
            Pan::Track(t) => pan_gains(t.get(skip + j).or(t.last()).copied().unwrap_or(0.0)),
        };
        put(start + j, s * gl, s * gr);
    }
}

/// A mixing bus: a dry stereo sum and a stereo send to a room.
#[derive(Debug, Clone, PartialEq)]
pub struct Bus {
    pub dry: Stereo,
    pub send: Stereo,
}

impl Bus {
    /// A silent bus of `n` samples.
    pub fn new(n: usize) -> Self {
        Bus {
            dry: Stereo::silence(n),
            send: Stereo::silence(n),
        }
    }

    /// Adds `gain · sig` from sample `at`, panned with `pan`, and sends
    /// `send` of the panned signal to the room.
    pub fn place(&mut self, sig: &[f64], at: i64, gain: f64, pan: Pan, send: f64) {
        let n = self.dry.len();
        let (dry, wet) = (&mut self.dry, &mut self.send);
        place(
            sig,
            at,
            gain,
            pan,
            |i, l, r| {
                dry.l[i] += l;
                dry.r[i] += r;
                wet.l[i] += l * send;
                wet.r[i] += r * send;
            },
            n,
        );
    }

    /// The bus heard: `dry + wet · (send ∗ response)`, with one room
    /// response per channel ([`crate::synth::Room::impulse_response`]; two
    /// seeds decorrelate left and right). The responses are built once and
    /// shared by every bus in the same room.
    pub fn render(&self, responses: (&[f64], &[f64]), wet: f64) -> Stereo {
        let mut out = self.dry.clone();
        let l = convolve(&self.send.l, responses.0);
        let r = convolve(&self.send.r, responses.1);
        for (a, b) in out.l.iter_mut().zip(&l) {
            *a += wet * b;
        }
        for (a, b) in out.r.iter_mut().zip(&r) {
            *a += wet * b;
        }
        out
    }
}
