//! From a mix to a file: fades, peak normalisation and 16-bit stereo PCM
//! (R-0008 AC10).

use crate::pcm::Pcm16;
use crate::synth::bus::Stereo;

/// Linear fades: in over `fade_in` seconds, out over the last `fade_out`.
pub fn fade(x: &mut Stereo, rate: f64, fade_in: f64, fade_out: f64) {
    let n = x.len();
    let dur = n as f64 / rate;
    let g: Vec<f64> = (0..n)
        .map(|i| {
            let t = i as f64 / rate;
            let a = if fade_in > 0.0 {
                (t / fade_in).clamp(0.0, 1.0)
            } else {
                1.0
            };
            let b = if fade_out > 0.0 {
                ((dur - t) / fade_out).clamp(0.0, 1.0)
            } else {
                1.0
            };
            a * b
        })
        .collect();
    x.apply_gain(&g);
}

/// Scales `x` so its peak is `target` (a silent mix stays silent).
/// Returns the factor applied.
pub fn normalize_peak(x: &mut Stereo, target: f64) -> f64 {
    let p = x.peak();
    let k = if p > 0.0 { target / p } else { 1.0 };
    x.scale(k);
    k
}

/// One sample to 16 bits: `x · 32767`, rounded to the nearest integer (ties
/// to even) and clamped.
pub fn to_i16(x: f64) -> i16 {
    (x * 32767.0).round_ties_even().clamp(-32768.0, 32767.0) as i16
}

/// Interleaved 16-bit stereo at `rate`.
pub fn to_pcm16(x: &Stereo, rate: u32) -> Pcm16 {
    let mut samples = Vec::with_capacity(2 * x.len());
    for (l, r) in x.l.iter().zip(&x.r) {
        samples.push(to_i16(*l));
        samples.push(to_i16(*r));
    }
    Pcm16 {
        sample_rate: rate,
        channels: 2,
        samples,
    }
}

/// A mono 16-bit track as `f64` in `[−1, 1)` (`/ 32768`).
pub fn from_pcm16(samples: &[i16]) -> Vec<f64> {
    samples.iter().map(|s| *s as f64 / 32768.0).collect()
}
