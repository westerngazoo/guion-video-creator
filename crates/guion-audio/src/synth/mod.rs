//! Generic, deterministic sound synthesis for reels (R-0008).
//!
//! The building blocks a studio's sound design is made of, with no brand in
//! them: oscillators ([`osc`]), envelopes ([`env`]), seeded noise
//! ([`noise`]), filters ([`filter`]), an algorithmic room ([`room`]),
//! constant-power pan and mixing buses ([`bus`]), ducking under a voice
//! ([`duck`]) and the way out to a 16-bit stereo WAV ([`out`], through
//! [`crate::write_wav`]).
//!
//! **Deterministic.** Everything runs on one thread in a fixed order, the
//! FFT ([`fft`]) included, and every random number comes from a stream named
//! by `(seed, index)` ([`noise::seed_for`]). The same input gives the same
//! bytes on the same platform (`sin`, `exp` and `ln` are the platform's
//! `libm`). Instruments (a drum, a bell, a house drone) belong to the
//! studio that designs them, not here.

pub mod bus;
pub mod duck;
pub mod env;
pub mod fft;
pub mod filter;
pub mod noise;
pub mod osc;
pub mod out;
pub mod room;

pub use bus::{pan_gains, Bus, Pan, Stereo};
pub use duck::{duck_follower, duck_windows, follow, narration_windows, DuckShape, Follower};
pub use env::{breakpoints, exp_decay, linear_attack, rise, Adsr};
pub use filter::{bandpass, highpass, lowpass, OnePole};
pub use noise::{seed_for, Noise};
pub use osc::{add_sine, partials, phase_of, sine, Partial};
pub use out::{fade, from_pcm16, normalize_peak, to_i16, to_pcm16};
pub use room::Room;

/// The sample at time `t` seconds: `round(t · rate)`, ties to even (like
/// Python's `round`), so an event lands on the same sample in every tool.
pub fn sample_at(t: f64, rate: f64) -> i64 {
    (t * rate).round_ties_even() as i64
}
