//! The two easing helpers the narration's fades use. They are kept to the
//! exact floating-point operations of the studio they came from, so a fade
//! gives the same opacity, bit for bit (AC8).

/// Normalized progress of `t` through `[a, b]`, clamped to `[0, 1]`.
pub fn progress(t: f64, a: f64, b: f64) -> f64 {
    ((t - a) / (b - a)).clamp(0.0, 1.0)
}

/// Cubic ease-out: 1 − (1 − p)³.
pub fn ease_out_cubic(p: f64) -> f64 {
    1.0 - (1.0 - p).powi(3)
}
