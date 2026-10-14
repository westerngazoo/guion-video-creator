//! Seeded noise: one independent stream per event (R-0008 AC4, AC1).
//!
//! The generator is xoshiro256** seeded through SplitMix64; Gaussian samples
//! come from Box–Muller. A stream is named by `(seed, index)` with
//! [`seed_for`], so the noise of one event does not depend on how many other
//! events drew noise before it, or in what order they were rendered.

use std::f64::consts::TAU;

/// One step of SplitMix64: a well-mixed 64-bit value from any 64-bit state.
fn splitmix(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// The seed of stream `index` under `seed`: a post's seed and an event's
/// number give that event its own noise.
pub fn seed_for(seed: u64, index: u64) -> u64 {
    let mut s = seed ^ index.wrapping_mul(0xD1B5_4A32_D192_ED03);
    splitmix(&mut s)
}

/// A deterministic noise source.
#[derive(Debug, Clone)]
pub struct Noise {
    s: [u64; 4],
    spare: Option<f64>,
}

impl Noise {
    /// A stream from `seed`; the same seed always gives the same samples.
    pub fn new(seed: u64) -> Self {
        let mut st = seed;
        let s = [
            splitmix(&mut st),
            splitmix(&mut st),
            splitmix(&mut st),
            splitmix(&mut st),
        ];
        Noise { s, spare: None }
    }

    fn next_u64(&mut self) -> u64 {
        let r = self.s[1].wrapping_mul(5).rotate_left(7).wrapping_mul(9);
        let t = self.s[1] << 17;
        self.s[2] ^= self.s[0];
        self.s[3] ^= self.s[1];
        self.s[1] ^= self.s[2];
        self.s[0] ^= self.s[3];
        self.s[2] ^= t;
        self.s[3] = self.s[3].rotate_left(45);
        r
    }

    /// Uniform in `[0, 1)`, 53 bits.
    pub fn uniform(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0 / (1u64 << 53) as f64)
    }

    /// A random phase in `[0, 2π)`.
    pub fn phase(&mut self) -> f64 {
        TAU * self.uniform()
    }

    /// Standard normal (mean 0, variance 1), Box–Muller.
    pub fn gaussian(&mut self) -> f64 {
        if let Some(z) = self.spare.take() {
            return z;
        }
        // 1 − u is in (0, 1]: the logarithm is finite
        let u = 1.0 - self.uniform();
        let v = self.uniform();
        let r = (-2.0 * u.ln()).sqrt();
        self.spare = Some(r * (TAU * v).sin());
        r * (TAU * v).cos()
    }

    /// `n` standard-normal samples: white noise.
    pub fn white(&mut self, n: usize) -> Vec<f64> {
        (0..n).map(|_| self.gaussian()).collect()
    }
}
