//! Ducking: a bed that steps aside while the voice speaks (R-0008 AC9).
//!
//! Two ways to know when the voice speaks:
//!
//! - **By the line windows** ([`duck_windows`]): the measured `(t0, t1)` of
//!   each line, as a [`Narration`] reports them ([`narration_windows`]) or a
//!   studio's timing file stores them. Exact and independent of the level.
//! - **By an envelope follower** ([`duck_follower`]): the voice's own level,
//!   for a voice track with no timing file.
//!
//! Either way the gain is `10^(depth/20)` inside a window (widened by `pre`
//! and `post`) and 1 outside, smoothed by a centred moving average `ramp`
//! seconds long, so it moves without clicks.

use crate::error::AudioError;
use crate::lines::Narration;
use crate::synth::filter::OnePole;

/// How deep and how fast the bed ducks.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DuckShape {
    /// Gain inside a window, dB (negative ducks).
    pub depth_db: f64,
    /// The window opens this many seconds before the voice.
    pub pre: f64,
    /// And closes this many seconds after it.
    pub post: f64,
    /// Length of the smoothing (the ramps), seconds.
    pub ramp: f64,
}

impl DuckShape {
    fn check(&self) -> Result<(), AudioError> {
        let bad = |what: &str, message: String| {
            Err(AudioError::InvalidSynth {
                what: format!("duck.{what}"),
                message,
            })
        };
        if !self.depth_db.is_finite() {
            return bad("depth_db", format!("must be finite, got {}", self.depth_db));
        }
        for (name, v) in [("pre", self.pre), ("post", self.post), ("ramp", self.ramp)] {
            if !(v.is_finite() && v >= 0.0) {
                return bad(name, format!("must be finite and ≥ 0, got {v}"));
            }
        }
        Ok(())
    }
}

/// The `(t0, t1)` of every line of a narration.
pub fn narration_windows(n: &Narration) -> Vec<(f64, f64)> {
    n.lines.iter().map(|l| (l.t0, l.t1)).collect()
}

/// A gain curve of `n` samples: ducked by `shape` inside each window.
pub fn duck_windows(
    n: usize,
    rate: f64,
    windows: &[(f64, f64)],
    shape: &DuckShape,
) -> Result<Vec<f64>, AudioError> {
    shape.check()?;
    let low = 10f64.powf(shape.depth_db / 20.0);
    let mut g = vec![1.0; n];
    for &(t0, t1) in windows {
        if !(t0.is_finite() && t1.is_finite()) {
            return Err(AudioError::InvalidSynth {
                what: "duck.window".into(),
                message: format!("({t0}, {t1}) is not finite"),
            });
        }
        let a = ((t0 - shape.pre) * rate).round().max(0.0) as usize;
        let b = (((t1 + shape.post) * rate).round().max(0.0) as usize).min(n);
        for v in g.iter_mut().take(b).skip(a) {
            *v = low;
        }
    }
    Ok(smooth(&g, (shape.ramp * rate).round() as usize))
}

/// A centred moving average of length `m`, holding the edge values outside
/// the signal (so the gain does not sag at the start and the end).
fn smooth(g: &[f64], m: usize) -> Vec<f64> {
    if m <= 1 || g.is_empty() {
        return g.to_vec();
    }
    let n = g.len() as i64;
    let h = (m as i64 - 1) / 2;
    let at = |i: i64| g[i.clamp(0, n - 1) as usize];
    // prefix sums over the padded signal, in a fixed order
    let lo = -(m as i64);
    let mut pre = Vec::with_capacity((n + 2 * m as i64) as usize + 1);
    pre.push(0.0);
    let mut acc = 0.0;
    for i in lo..n + m as i64 {
        acc += at(i);
        pre.push(acc);
    }
    let idx = |i: i64| (i - lo) as usize;
    (0..n)
        .map(|i| {
            // the window i − (m − 1 − h) ..= i + h, like numpy's `same`
            let a = i - (m as i64 - 1 - h);
            let b = i + h;
            (pre[idx(b) + 1] - pre[idx(a)]) / m as f64
        })
        .collect()
}

/// The level of `x`: the rectified signal through a one-pole with separate
/// attack and release cutoffs (Hz), so it rises fast and falls slowly.
pub fn follow(x: &[f64], rate: f64, attack: f64, release: f64) -> Vec<f64> {
    let up = OnePole::new(rate, attack).a;
    let down = OnePole::new(rate, release).a;
    let mut y = 0.0;
    x.iter()
        .map(|v| {
            let a = v.abs();
            let k = if a > y { up } else { down };
            y += k * (a - y);
            y
        })
        .collect()
}

/// How an envelope follower decides that the voice speaks.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Follower {
    /// Attack cutoff, Hz.
    pub attack: f64,
    /// Release cutoff, Hz (low: holds over the gaps between words).
    pub release: f64,
    /// The voice speaks while its level is above this, dBFS.
    pub threshold_db: f64,
}

/// A gain curve of `voice.len()` samples: ducked by `shape` while the
/// voice's level (by `follower`) is above the threshold.
pub fn duck_follower(
    voice: &[f64],
    rate: f64,
    follower: &Follower,
    shape: &DuckShape,
) -> Result<Vec<f64>, AudioError> {
    for (name, v) in [("attack", follower.attack), ("release", follower.release)] {
        if !(v.is_finite() && v > 0.0) {
            return Err(AudioError::InvalidSynth {
                what: format!("follower.{name}"),
                message: format!("must be finite and > 0 Hz, got {v}"),
            });
        }
    }
    let thr = 10f64.powf(follower.threshold_db / 20.0);
    let env = follow(voice, rate, follower.attack, follower.release);
    let mut windows = Vec::new();
    let mut open: Option<usize> = None;
    for (i, e) in env.iter().enumerate() {
        match (open, *e > thr) {
            (None, true) => open = Some(i),
            (Some(a), false) => {
                windows.push((a as f64 / rate, i as f64 / rate));
                open = None;
            }
            _ => {}
        }
    }
    if let Some(a) = open {
        windows.push((a as f64 / rate, env.len() as f64 / rate));
    }
    duck_windows(voice.len(), rate, &windows, shape)
}
