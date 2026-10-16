//! A [`Score`] → the mix and its stems (R-0011 AC7–AC9).

use std::collections::BTreeMap;
use std::path::Path;

use crate::synth::fft::convolve;
use crate::synth::{
    breakpoints, duck_follower, duck_windows, fade, filter, normalize_peak, sample_at, seed_for,
    Bus, DuckShape, Noise, Pan, Room, Stereo,
};

use super::instrument::Ctx;
use super::parse::{BedLevel, DuckBy, Piece, Score, Source, STREAM_ROOM, STREAM_ROOM_VOICE};
use super::timeline::Timeline;
use super::RATE;

/// The voice coming in: samples (48 kHz, mono, in [−1, 1)) and the (t0, t1)
/// windows of its lines.
pub struct Voice<'a> {
    pub samples: &'a [f64],
    pub lines: &'a [(f64, f64)],
}

/// What comes out: the mix and its stems, at the mix's scale (voice + bed =
/// mix before the fades), to measure them.
pub struct Output {
    pub mix: Stereo,
    pub voice: Stereo,
    pub bed: Stereo,
    /// Each bus with no room and no ducking, at the bed's scale.
    pub dry: BTreeMap<String, Stereo>,
    /// Each bus with its room, no ducking, at the bed's scale.
    pub buses: BTreeMap<String, Stereo>,
    /// How far the bed sits under the voice, in words.
    pub report: String,
}

fn signal(p: &Piece, n: usize, times: &Timeline) -> (Vec<f64>, i64, Option<Vec<f64>>) {
    let mut g = Noise::new(p.seed);
    match &p.source {
        Source::Drone {
            sound,
            envelope,
            lfo,
        } => {
            let mut cx = Ctx {
                len: n,
                noise: &mut g,
                seed: p.seed,
                series: &[],
            };
            let mut x = sound.render(&mut cx).samples;
            let e = breakpoints(n, RATE, envelope);
            for (i, v) in x.iter_mut().enumerate() {
                let l = lfo.map_or(1.0, |(hz, depth)| {
                    1.0 + depth * (std::f64::consts::TAU * hz * i as f64 / RATE).sin()
                });
                *v *= e[i] * l;
            }
            (x, 0, None)
        }
        Source::Follower {
            series,
            sound,
            from,
            to,
            envelope,
            lfo,
        } => {
            let s = times
                .series(series)
                .expect("the series was checked at parse");
            let v = s.samples(n);
            let mut cx = Ctx {
                len: n,
                noise: &mut g,
                seed: p.seed,
                series: &v,
            };
            let mut x = sound.render(&mut cx).samples;
            let e = envelope.as_ref().map(|p| breakpoints(n, RATE, p));
            for (i, v) in x.iter_mut().enumerate() {
                let t = i as f64 / RATE;
                if from.is_some_and(|d| t < d) || to.is_some_and(|h| t > h) {
                    *v = 0.0;
                }
                if let Some(e) = &e {
                    *v *= e[i];
                }
                if let Some((hz, depth)) = lfo {
                    *v *= 1.0 + depth * (std::f64::consts::TAU * hz * t).sin();
                }
            }
            (x, 0, None)
        }
        Source::Hit { at, sound } => {
            let mut cx = Ctx {
                len: n,
                noise: &mut g,
                seed: p.seed,
                series: &[],
            };
            let r = sound.render(&mut cx);
            let x = r.samples;
            // the moving pan: along the instrument's curve, or in a straight line
            let pan = p.pan_to.map(|b| {
                let m = x.len();
                let k = r
                    .pan_curve
                    .unwrap_or_else(|| (0..m).map(|i| i as f64 / (m.max(2) - 1) as f64).collect());
                k.iter().map(|k| p.pan + (b - p.pan) * k).collect()
            });
            (x, sample_at(*at, RATE), pan)
        }
    }
}

/// The voice windows, with gaps shorter than `gap` s closed.
pub fn merge_lines(lines: &[(f64, f64)], gap: f64) -> Vec<(f64, f64)> {
    let mut out: Vec<(f64, f64)> = Vec::new();
    for &(a, b) in lines {
        match out.last_mut() {
            Some(u) if a - u.1 < gap => u.1 = u.1.max(b),
            _ => out.push((a, b)),
        }
    }
    out
}

fn rms_db(x: &[f64], m: &[bool], inside: bool) -> f64 {
    let (mut s, mut c) = (0.0, 0usize);
    for (v, h) in x.iter().zip(m) {
        if *h == inside {
            s += v * v;
            c += 1;
        }
    }
    20.0 * ((s / c.max(1) as f64).sqrt() + 1e-12).log10()
}

/// The whole sound, in score order. Fails (writing nothing) on a missing
/// voice or a bad room or duck.
pub fn render(score: &Score, times: &Timeline, voice: Option<Voice>) -> Result<Output, String> {
    let d = score;
    let n = (RATE * times.duration) as usize;
    if d.voice.is_some() && voice.is_none() {
        return Err("[voz]: falta voz.wav".into());
    }
    // the buses, in order of appearance
    let mut names: Vec<String> = Vec::new();
    for p in &d.pieces {
        if !names.contains(&p.bus) {
            names.push(p.bus.clone());
        }
    }
    let mut buses: Vec<Bus> = names.iter().map(|_| Bus::new(n)).collect();
    for p in &d.pieces {
        let (x, at, pan) = signal(p, n, times);
        let b = names
            .iter()
            .position(|b| *b == p.bus)
            .expect("every bus is in the list");
        let pan = pan.as_deref().map_or(Pan::Fixed(p.pan), Pan::Track);
        buses[b].place(&x, at, p.gain, pan, p.send);
    }
    // one response per channel, the same for every bus (the room is one)
    let room = |channel: u64, t60: f64| {
        Room::new(
            t60,
            d.room.length,
            d.room.damping,
            seed_for(d.seed, channel),
        )
        .map(|r| r.impulse_response(RATE))
        .map_err(|e| e.to_string())
    };
    let (rl, rr) = (
        room(STREAM_ROOM, d.room.t60)?,
        room(STREAM_ROOM + 1, d.room.t60)?,
    );
    let heard: Vec<Stereo> = buses
        .iter()
        .map(|b| b.render((&rl, &rr), d.room.send))
        .collect();

    // the voice: no extra lows, at peak 1, and a little room
    let (voice_st, mono, lines) = match (&d.voice, &voice) {
        (Some(c), Some(v)) => {
            let mut m = v.samples.to_vec();
            m.resize(n, 0.0);
            let mut m = filter::highpass(&m, RATE, c.highpass, 2);
            let p = m.iter().fold(0.0_f64, |a, x| a.max(x.abs()));
            if p > 0.0 {
                m.iter_mut().for_each(|x| *x /= p);
            }
            let wl = convolve(&m, &room(STREAM_ROOM_VOICE, c.room_t60)?);
            let wr = convolve(&m, &room(STREAM_ROOM_VOICE + 1, c.room_t60)?);
            let st = Stereo {
                l: m.iter()
                    .zip(&wl)
                    .map(|(a, b)| c.level * a + c.room_level * b)
                    .collect(),
                r: m.iter()
                    .zip(&wr)
                    .map(|(a, b)| c.level * a + c.room_level * b)
                    .collect(),
            };
            (st, m, v.lines.to_vec())
        }
        _ => (Stereo::silence(n), Vec::new(), Vec::new()),
    };

    // the bed: every bus together at its level, each bus ducked under the voice
    let mut together = Stereo::silence(n);
    for o in &heard {
        together.mix(o, 1.0);
    }
    let peak = together.peak();
    let rms = |x: &Stereo| {
        (x.l.iter().chain(&x.r).map(|v| v * v).sum::<f64>() / (2 * x.len().max(1)) as f64).sqrt()
    };
    let k_bed = match d.mix.bed {
        BedLevel::Peak(c) if peak > 0.0 => c / peak,
        BedLevel::Rms(db) if peak > 0.0 => {
            let reference = if d.voice.is_some() {
                rms(&voice_st)
            } else {
                1.0
            };
            10f64.powf(db / 20.0) * reference / rms(&together)
        }
        _ => 1.0,
    };
    let mut bed = Stereo::silence(n);
    for (name, o) in names.iter().zip(&heard) {
        let mut o = o.clone();
        if let Some(a) = &d.duck {
            if let Some((_, db)) = a.buses.iter().find(|(b, _)| b == name) {
                let shape = DuckShape {
                    depth_db: *db,
                    pre: a.pre,
                    post: a.post,
                    ramp: a.ramp,
                };
                let g = match a.by {
                    DuckBy::Lines => duck_windows(n, RATE, &merge_lines(&lines, a.merge), &shape),
                    DuckBy::Follower(f) => duck_follower(&mono, RATE, &f, &shape),
                }
                .map_err(|e| format!("[agachar] «{name}»: {e}"))?;
                o.apply_gain(&g);
            }
        }
        bed.mix(&o, k_bed);
    }

    let mut mix = voice_st.clone();
    mix.mix(&bed, 1.0);
    fade(&mut mix, RATE, d.mix.fade_in, d.mix.fade_out);
    let k = normalize_peak(&mut mix, d.mix.peak);

    let mut voice_st = voice_st;
    voice_st.scale(k);
    bed.scale(k);
    let scaled = |x: &Stereo| {
        let mut s = x.clone();
        s.scale(k_bed * k);
        s
    };
    let dry = names
        .iter()
        .zip(&buses)
        .map(|(name, b)| (name.clone(), scaled(&b.dry)))
        .collect();
    let wet = names
        .iter()
        .zip(&heard)
        .map(|(name, o)| (name.clone(), scaled(o)))
        .collect();

    // how far the bed sits under the voice (RMS in the lines and between them)
    let report = if lines.is_empty() {
        format!(
            "cama sola, {:.1} dB RMS (canal izq.)",
            rms_db(&bed.l, &vec![true; n], true)
        )
    } else {
        let (pre, post) = d.duck.as_ref().map_or((0.08, 0.12), |a| (a.pre, a.post));
        let mut speaks = vec![false; n];
        for (t0, t1) in &lines {
            let a = ((t0 - pre) * RATE).round().max(0.0) as usize;
            let b = (((t1 + post) * RATE).round().max(0.0) as usize).min(n);
            speaks.iter_mut().take(b).skip(a).for_each(|h| *h = true);
        }
        format!(
            "voz {:.1} dB; cama bajo la voz {:.1} dB, entre líneas {:.1} dB (RMS, canal izq.)",
            rms_db(&voice_st.l, &speaks, true),
            rms_db(&bed.l, &speaks, true),
            rms_db(&bed.l, &speaks, false)
        )
    };
    let report = format!(
        "{report}\nlos buses suman un pico de {peak:.3} y un RMS de {:.1} dB; la cama va ×{k_bed:.3} (pico {:.3}, RMS {:.1} dB frente a la voz); la mezcla, ×{k:.3}",
        20.0 * rms(&together).log10(),
        peak * k_bed,
        20.0 * (rms(&together) * k_bed * k / rms(&voice_st).max(1e-12)).log10(),
    );
    Ok(Output {
        mix,
        voice: voice_st,
        bed,
        dry,
        buses: wet,
        report,
    })
}

/// The resolved hits, one per row: instrument, start and end in s (the end
/// only for those with an extent, like a swell), to measure.
pub fn hits_tsv(score: &Score) -> String {
    let mut out = String::from("instrumento\ten\tfin\n");
    for p in &score.pieces {
        if let Source::Hit { at, sound } = &p.source {
            let end = sound
                .extent()
                .map_or(String::new(), |x| format!("{:.6}", at + x));
            out.push_str(&format!("{}\t{at:.6}\t{end}\n", p.instrument));
        }
    }
    out
}

/// Writes `x` as interleaved `f32` (L, R), to measure a stem.
pub fn write_f32(path: &Path, x: &Stereo) -> std::io::Result<()> {
    let mut b = Vec::with_capacity(8 * x.len());
    for (l, r) in x.l.iter().zip(&x.r) {
        b.extend_from_slice(&(*l as f32).to_le_bytes());
        b.extend_from_slice(&(*r as f32).to_le_bytes());
    }
    std::fs::write(path, b)
}
