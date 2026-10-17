//! Envelopes: exponential decays, linear attacks, ADSR and breakpoint
//! curves (R-0008 AC3).

/// `e^{−i / (rate · τ)}` for `i < n`. A non-positive `τ` gives a one-sample
/// impulse (the limit of an instant decay), never a NaN.
pub fn exp_decay(n: usize, rate: f64, tau: f64) -> Vec<f64> {
    if tau <= 0.0 || !tau.is_finite() {
        return (0..n).map(|i| if i == 0 { 1.0 } else { 0.0 }).collect();
    }
    let k = -1.0 / (rate * tau);
    (0..n).map(|i| (k * i as f64).exp()).collect()
}

/// `min(1, t / attack)`: a linear rise to 1 in `attack` seconds.
pub fn linear_attack(n: usize, rate: f64, attack: f64) -> Vec<f64> {
    (0..n)
        .map(|i| {
            if attack <= 0.0 {
                1.0
            } else {
                (i as f64 / rate / attack).min(1.0)
            }
        })
        .collect()
}

/// `(i / (n − 1))^p` for `i < n`: a rise from 0 to 1 over the whole signal
/// (`p = 3` is the house swell).
pub fn rise(n: usize, p: f64) -> Vec<f64> {
    if n < 2 {
        return vec![1.0; n];
    }
    let d = (n - 1) as f64;
    (0..n).map(|i| (i as f64 / d).powf(p)).collect()
}

/// Attack, decay, sustain, release: linear segments, in seconds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Adsr {
    pub attack: f64,
    pub decay: f64,
    /// Level held after the decay, 0–1.
    pub sustain: f64,
    pub release: f64,
}

impl Adsr {
    /// The envelope of a note held for `gate` seconds, `n` samples long
    /// (the release starts at the gate from the level reached there).
    pub fn render(&self, n: usize, rate: f64, gate: f64) -> Vec<f64> {
        let level = |t: f64| -> f64 {
            if t < self.attack {
                t / self.attack
            } else if t < self.attack + self.decay {
                1.0 - (1.0 - self.sustain) * (t - self.attack) / self.decay
            } else {
                self.sustain
            }
        };
        let at_gate = level(gate);
        (0..n)
            .map(|i| {
                let t = i as f64 / rate;
                if t < gate {
                    level(t)
                } else if self.release > 0.0 && t < gate + self.release {
                    at_gate * (1.0 - (t - gate) / self.release)
                } else {
                    0.0
                }
            })
            .collect()
    }
}

/// A piecewise-linear curve through `(t, value)` points, sampled at
/// `i / rate` for `i < n`; flat before the first point and after the last.
/// Points must be in time order (two at the same time make a step).
pub fn breakpoints(n: usize, rate: f64, points: &[(f64, f64)]) -> Vec<f64> {
    if points.is_empty() {
        return vec![0.0; n];
    }
    let mut out = Vec::with_capacity(n);
    let mut j = 0;
    for i in 0..n {
        let t = i as f64 / rate;
        while j + 1 < points.len() && points[j + 1].0 <= t {
            j += 1;
        }
        let (t0, v0) = points[j];
        let v = if t <= t0 || j + 1 == points.len() {
            if t < points[0].0 {
                points[0].1
            } else {
                v0
            }
        } else {
            let (t1, v1) = points[j + 1];
            v0 + (v1 - v0) * (t - t0) / (t1 - t0)
        };
        out.push(v);
    }
    out
}
