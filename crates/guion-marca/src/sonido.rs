//! Neutral instruments for the score (R-0011), so a new account has a bed
//! before it has a sound of its own (R-0014 AC7). `guion reel sonido <post>`
//! renders `sonido.toml` with them.
//!
//! - `[[dron]] instrumento = "tono"`: a soft tone, `f0` (Hz, default 110),
//!   with its octave and twelfth quietly above;
//! - `[[golpe]] instrumento = "pulso"`: a struck tone, `f0` (default 220),
//!   decaying over `dur` s (default 0.6, or `en` … `hasta`);
//! - `[[golpe]] instrumento = "roce"`: a short burst of the sound's own noise,
//!   decaying over `dur` s (default 0.25).
//!
//! An account that wants its own instruments registers them in its motor
//! (as Rotor's do); phase 2 describes them in the contract (R-0014 §5).

use std::path::Path;

use guion_audio::score::{render_post, sound, Instruments, Kind, PostOutput, RATE};
use guion_audio::synth::{exp_decay, sine};

/// The neutral kit.
pub fn instrumentos() -> Instruments {
    let mut r = Instruments::new();
    r.add(Kind::Drone, "tono", |d| {
        let f0 = d.frequency(110.0)?;
        Ok(sound(move |cx| {
            let mut s = sine(cx.len, RATE, f0, 0.0, 1.0);
            for (k, a) in [(2.0, 0.3), (3.0, 0.1)] {
                for (x, y) in s.iter_mut().zip(sine(cx.len, RATE, f0 * k, 0.0, a)) {
                    *x += y;
                }
            }
            s.into()
        }))
    });
    r.add(Kind::Hit, "pulso", |d| {
        let (f0, dur) = (d.frequency(220.0)?, d.duration(0.6)?);
        Ok(sound(move |_| {
            let n = (dur * RATE) as usize;
            let e = exp_decay(n, RATE, dur / 4.0);
            let a = sine(n, RATE, f0, 0.0, 1.0);
            let b = sine(n, RATE, f0 * 2.0, 0.0, 0.25);
            a.iter()
                .zip(&b)
                .zip(&e)
                .map(|((a, b), e)| (a + b) * e)
                .collect::<Vec<_>>()
                .into()
        }))
    });
    r.add(Kind::Hit, "roce", |d| {
        let dur = d.duration(0.25)?;
        Ok(sound(move |cx| {
            let n = (dur * RATE) as usize;
            let e = exp_decay(n, RATE, dur / 5.0);
            cx.noise
                .white(n)
                .iter()
                .zip(e)
                .map(|(a, b)| a * b * 0.5)
                .collect::<Vec<_>>()
                .into()
        }))
    });
    r.set_default(Kind::Drone, "tono");
    r.set_default(Kind::Hit, "pulso");
    r
}

/// `<post>/sonido.toml` (+ `eventos.json`, `voz.wav`, `voz.json`) →
/// `<post>/audio.wav`, with the neutral kit.
pub fn render(post: &Path) -> Result<PostOutput, String> {
    render_post(post, &instrumentos(), None, None).map_err(|e| e.to_string())
}
