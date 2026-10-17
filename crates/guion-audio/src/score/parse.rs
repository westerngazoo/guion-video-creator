//! `sonido.toml` → a [`Score`], with every time already resolved against the
//! [`Timeline`] (R-0011 AC1–AC6).

use crate::synth::{duck::Follower, seed_for};

use super::instrument::{Instruments, Kind, Sound};
use super::params::Params;
use super::timeline::Timeline;

/// The room of the buses (one response per channel, with its seed).
#[derive(Clone, Copy, Debug)]
pub struct RoomCfg {
    pub t60: f64,
    pub length: f64,
    pub damping: f64,
    /// How much of the room returns to the mix.
    pub send: f64,
}

/// The voice up front.
#[derive(Clone, Copy, Debug)]
pub struct VoiceCfg {
    pub highpass: f64,
    pub level: f64,
    pub room_t60: f64,
    pub room_level: f64,
}

/// The level of the bed (before it ducks).
#[derive(Clone, Copy, Debug)]
pub enum BedLevel {
    /// The peak of all buses together, against the voice's peak (1). It hangs
    /// on one peak, so another room or another noise moves it 2 or 3 dB.
    Peak(f64),
    /// The bed's RMS (both channels, the whole reel), in dB against the voice
    /// as it enters the mix (or full scale, with no voice). It keeps the
    /// balance.
    Rms(f64),
}

/// The final mix.
#[derive(Clone, Copy, Debug)]
pub struct MixCfg {
    pub bed: BedLevel,
    pub peak: f64,
    pub fade_in: f64,
    pub fade_out: f64,
}

/// How the engine knows when the voice speaks.
#[derive(Clone, Copy, Debug)]
pub enum DuckBy {
    /// The lines of `voz.json`.
    Lines,
    /// The level of `voz.wav`.
    Follower(Follower),
}

/// The bed ducks under the voice.
#[derive(Clone, Debug)]
pub struct DuckCfg {
    pub by: DuckBy,
    pub pre: f64,
    pub post: f64,
    pub ramp: f64,
    /// With [`DuckBy::Lines`]: gaps between lines shorter than `merge` s do
    /// not open the bed (0: each line is its own window).
    pub merge: f64,
    /// dB per bus.
    pub buses: Vec<(String, f64)>,
}

/// Where a sound comes from.
#[derive(Debug)]
pub enum Source {
    Drone {
        sound: Box<dyn Sound>,
        envelope: Vec<(f64, f64)>,
        lfo: Option<(f64, f64)>,
    },
    /// `envelope` and `lfo` as in the drone (without `envelope`, the level is 1).
    Follower {
        series: String,
        sound: Box<dyn Sound>,
        from: Option<f64>,
        to: Option<f64>,
        envelope: Option<Vec<(f64, f64)>>,
        lfo: Option<(f64, f64)>,
    },
    Hit {
        at: f64,
        sound: Box<dyn Sound>,
    },
}

/// A placed sound: its source, level, place and noise stream.
#[derive(Debug)]
pub struct Piece {
    pub source: Source,
    /// The instrument's name, as the score wrote it.
    pub instrument: String,
    pub gain: f64,
    pub pan: f64,
    /// For a moving pan: from `pan` to this.
    pub pan_to: Option<f64>,
    pub send: f64,
    pub bus: String,
    pub seed: u64,
    /// Where it is in the score, for errors.
    pub location: String,
}

/// All of `sonido.toml`, with its times resolved.
#[derive(Debug)]
pub struct Score {
    pub seed: u64,
    pub room: RoomCfg,
    pub voice: Option<VoiceCfg>,
    pub mix: MixCfg,
    pub duck: Option<DuckCfg>,
    pub pieces: Vec<Piece>,
}

/// The noise streams: each class of sound in its range.
pub(crate) const STREAM_HIT: u64 = 0;
pub(crate) const STREAM_DRONE: u64 = 1_000_000;
pub(crate) const STREAM_FOLLOWER: u64 = 2_000_000;
pub(crate) const STREAM_ROOM: u64 = 3_000_000;
pub(crate) const STREAM_ROOM_VOICE: u64 = 3_000_010;

/// The most hits one table may resolve to.
pub const MAX_HITS: usize = 100;

impl Score {
    /// Reads `sonido.toml` against the scene's times, with `instruments`.
    pub fn parse(
        source: &str,
        times: &Timeline,
        instruments: &Instruments,
    ) -> Result<Score, String> {
        let root: toml::Table = source.parse().map_err(|e: toml::de::Error| e.to_string())?;
        let r = Params::new(&root, "sonido.toml");

        let s = r.table("sonido")?.ok_or("falta la tabla [sonido]")?;
        let seed = match s.get("semilla") {
            Some(toml::Value::Integer(n)) if *n >= 0 => *n as u64,
            None => 0,
            Some(_) => return Err("[sonido]: semilla debe ser un entero ≥ 0".into()),
        };
        let room = match s.table("sala")? {
            None => RoomCfg {
                t60: 2.8,
                length: 3.4,
                damping: 4200.0,
                send: 0.8,
            },
            Some(t) => {
                let v = RoomCfg {
                    t60: t.positive("t60", 2.8)?,
                    length: t.positive("largo", 3.4)?,
                    damping: t.positive("oscuro", 4200.0)?,
                    send: t.num_or("envio", 0.8)?,
                };
                t.finish()?;
                v
            }
        };
        s.finish()?;

        let voice = match r.table("voz")? {
            None => None,
            Some(v) => {
                let (room_t60, room_level) = match v.table("sala")? {
                    None => (1.2, 0.06),
                    Some(t) => {
                        let x = (t.positive("t60", 1.2)?, t.num_or("nivel", 0.06)?);
                        t.finish()?;
                        x
                    }
                };
                let c = VoiceCfg {
                    highpass: v.positive("paso_alto", 70.0)?,
                    level: v.num_or("nivel", 0.707)?,
                    room_t60,
                    room_level,
                };
                v.finish()?;
                Some(c)
            }
        };

        let mix = match r.table("mezcla")? {
            None => MixCfg {
                bed: BedLevel::Peak(0.30),
                peak: 0.89,
                fade_in: 0.02,
                fade_out: 0.5,
            },
            Some(m) => {
                let bed = match (m.num("cama")?, m.num("cama_rms")?) {
                    (Some(_), Some(_)) => {
                        return Err("[mezcla]: cama o cama_rms, no los dos".into())
                    }
                    (Some(c), None) if c > 0.0 => BedLevel::Peak(c),
                    (Some(c), None) => return Err(format!("[mezcla]: cama debe ser > 0 (es {c})")),
                    (None, Some(db)) => BedLevel::Rms(db),
                    (None, None) => BedLevel::Peak(0.30),
                };
                let x = MixCfg {
                    bed,
                    peak: m.positive("pico", 0.89)?,
                    fade_in: m.num_or("entrada", 0.02)?.max(0.0),
                    fade_out: m.num_or("salida", 0.5)?.max(0.0),
                };
                if x.peak > 1.0 {
                    return Err(format!("[mezcla]: pico = {} recortaría (máximo 1)", x.peak));
                }
                m.finish()?;
                x
            }
        };

        let duck = match r.table("agachar")? {
            None => None,
            Some(a) => {
                let by = match a.text("por")?.unwrap_or("lineas") {
                    "lineas" => DuckBy::Lines,
                    "seguidor" => DuckBy::Follower(Follower {
                        attack: a.positive("ataque", 50.0)?,
                        release: a.positive("suelta", 4.0)?,
                        threshold_db: a.num_or("umbral", -40.0)?,
                    }),
                    other => {
                        return Err(format!(
                            "[agachar]: por = «{other}» (valen: lineas, seguidor)"
                        ))
                    }
                };
                let mut buses = Vec::new();
                if let Some(b) = a.table("buses")? {
                    for k in b.t.keys() {
                        buses.push((k.clone(), b.num(k)?.ok_or("")?));
                    }
                    b.finish()?;
                }
                let x = DuckCfg {
                    by,
                    pre: a.num_or("antes", 0.08)?,
                    post: a.num_or("despues", 0.12)?,
                    ramp: a.num_or("rampa", 0.08)?,
                    merge: a.num_or("unir", 0.0)?,
                    buses,
                };
                a.finish()?;
                Some(x)
            }
        };

        let instrument_of = |d: &Params, kind: Kind| -> Result<String, String> {
            match (d.text("instrumento")?, instruments.default_for(kind)) {
                (Some(n), _) | (None, Some(n)) => Ok(n.to_string()),
                (None, None) => Err(format!("{}: falta instrumento", d.location)),
            }
        };

        let mut pieces = Vec::new();
        for (i, t) in tables(&r, "dron")?.iter().enumerate() {
            let d = Params::new(t, format!("[[dron]] {}", i + 1));
            let name = instrument_of(&d, Kind::Drone)?;
            let sound = instruments.build(Kind::Drone, &name, &d)?;
            let envelope = match d.list("envolvente")? {
                None => vec![(0.0, 1.0)],
                Some(v) => points(v, times, &d.location)?,
            };
            let lfo = match d.table("lfo")? {
                None => None,
                Some(l) => {
                    let x = (l.positive("hz", 0.06)?, l.num_or("hondo", 0.12)?);
                    l.finish()?;
                    Some(x)
                }
            };
            let source = Source::Drone {
                sound,
                envelope,
                lfo,
            };
            pieces.push(piece(
                &d,
                source,
                name,
                seed,
                STREAM_DRONE + i as u64,
                "dron",
            )?);
            d.finish()?;
        }

        for (i, t) in tables(&r, "seguidor")?.iter().enumerate() {
            let d = Params::new(t, format!("[[seguidor]] {}", i + 1));
            let series = d
                .text("serie")?
                .ok_or(format!("{}: falta serie", d.location))?
                .to_string();
            if times.series(&series).is_none() {
                return Err(format!(
                    "{}: eventos.json no tiene la serie «{series}»",
                    d.location
                ));
            }
            let name = instrument_of(&d, Kind::Follower)?;
            let sound = instruments.build(Kind::Follower, &name, &d)?;
            let at = |k: &str| -> Result<Option<f64>, String> {
                d.text(k)?
                    .map(|e| {
                        times
                            .time(e)
                            .map_err(|m| format!("{}: {k} {m}", d.location))
                    })
                    .transpose()
            };
            let (from, to) = (at("desde")?, at("hasta")?);
            let envelope = match d.list("envolvente")? {
                None => None,
                Some(v) => Some(points(v, times, &d.location)?),
            };
            let lfo = match d.table("lfo")? {
                None => None,
                Some(l) => {
                    let x = (l.positive("hz", 4.2)?, l.num_or("hondo", 0.08)?);
                    l.finish()?;
                    Some(x)
                }
            };
            let source = Source::Follower {
                series,
                sound,
                from,
                to,
                envelope,
                lfo,
            };
            pieces.push(piece(
                &d,
                source,
                name,
                seed,
                STREAM_FOLLOWER + i as u64,
                "dron",
            )?);
            d.finish()?;
        }

        for (i, t) in tables(&r, "golpe")?.iter().enumerate() {
            let d = Params::new(t, format!("[[golpe]] {}", i + 1));
            let instants = instants(&d, times)?;
            if instants.len() > MAX_HITS {
                return Err(format!(
                    "{}: {} golpes en una tabla (el máximo es {MAX_HITS}: pártela)",
                    d.location,
                    instants.len()
                ));
            }
            let name = instrument_of(&d, Kind::Hit)?;
            let curve = match d.list("envolvente")? {
                None => None,
                Some(v) => Some(points(v, times, &d.location)?),
            };
            if let Some(p) = d.table("paso")? {
                d.step.set(Some(p.t));
            }
            let n = instants.len();
            for (j, (label, at)) in instants.iter().enumerate() {
                let here = if n > 1 {
                    format!("{} («{label}»)", d.location)
                } else {
                    d.location.clone()
                };
                d.index.set((n > 1).then_some((j, n)));
                let at = *at;
                let span = match d.text("hasta")? {
                    None => None,
                    Some(h) => {
                        let end = times.time(h).map_err(|m| format!("{here}: hasta {m}"))?;
                        if end <= at {
                            return Err(format!(
                                "{here}: hasta ({end} s) no es después de en ({at} s)"
                            ));
                        }
                        Some(end - at)
                    }
                };
                d.span.set(span);
                let relocate = |m: String| m.replacen(&d.location, &here, 1);
                let sound = instruments.build(Kind::Hit, &name, &d).map_err(relocate)?;
                let stream = STREAM_HIT + (i as u64 + 1) * 100 + j as u64;
                let mut p = piece(
                    &d,
                    Source::Hit { at, sound },
                    name.clone(),
                    seed,
                    stream,
                    "golpe",
                )
                .map_err(relocate)?;
                if let Some(c) = &curve {
                    p.gain *= interp(c, at);
                }
                p.location = here;
                pieces.push(p);
            }
            d.index.set(None);
            d.span.set(None);
            d.finish()?;
        }
        r.finish()?;

        if let Some(a) = &duck {
            for (b, _) in &a.buses {
                if !pieces.iter().any(|p| p.bus == *b) {
                    return Err(format!("[agachar.buses]: ningún sonido va al bus «{b}»"));
                }
            }
            if voice.is_none() {
                return Err("[agachar]: sin [voz] no hay bajo qué agacharse".into());
            }
        }
        Ok(Score {
            seed,
            room,
            voice,
            mix,
            duck,
            pieces,
        })
    }
}

/// The instants of a hit: `en` (an event or a list), or `repetir`: `n` hits
/// spread from `desde` to before `hasta`, or one every `cada` s (or
/// `cada_cuadros`) from `desde` to before `hasta` (or `n` of them).
fn instants(d: &Params, times: &Timeline) -> Result<Vec<(String, f64)>, String> {
    let t = |e: &str, k: &str| {
        times
            .time(e)
            .map_err(|m| format!("{}: {k} {m}", d.location))
    };
    match (d.get("en"), d.table("repetir")?) {
        (Some(_), Some(_)) => Err(format!("{}: en o repetir, no los dos", d.location)),
        (None, None) => Err(format!(
            "{}: falta en (un evento o una lista) o repetir",
            d.location
        )),
        (Some(toml::Value::String(e)), None) => Ok(vec![(e.clone(), t(e, "en")?)]),
        (Some(toml::Value::Array(v)), None) if !v.is_empty() => v
            .iter()
            .map(|x| {
                let e = x.as_str().ok_or(format!(
                    "{}: en debe ser texto o una lista de textos",
                    d.location
                ))?;
                Ok((e.to_string(), t(e, "en")?))
            })
            .collect(),
        (Some(_), None) => Err(format!(
            "{}: en debe ser texto o una lista de textos",
            d.location
        )),
        (None, Some(r)) => {
            let from = t(
                r.text("desde")?
                    .ok_or(format!("{}: falta desde", r.location))?,
                "repetir.desde",
            )?;
            let to = r
                .text("hasta")?
                .map(|h| t(h, "repetir.hasta"))
                .transpose()?;
            let n = r.num("n")?;
            let every = match (r.num("cada")?, r.num("cada_cuadros")?) {
                (Some(_), Some(_)) => {
                    return Err(format!("{}: cada o cada_cuadros, no los dos", r.location))
                }
                (Some(c), None) => Some(c),
                (None, Some(c)) => Some(c / times.fps),
                (None, None) => None,
            };
            r.finish()?;
            let ts: Vec<f64> = match (n, every, to) {
                (Some(n), None, Some(h)) if n >= 1.0 => (0..n as usize)
                    .map(|i| from + (h - from) * i as f64 / n)
                    .collect(),
                (Some(n), Some(c), None) if n >= 1.0 && c > 0.0 => {
                    (0..n as usize).map(|i| from + c * i as f64).collect()
                }
                (None, Some(c), Some(h)) if c > 0.0 => (0..)
                    .map(|i| from + c * i as f64)
                    .take_while(|x| *x < h)
                    .take(MAX_HITS + 1)
                    .collect(),
                _ => {
                    return Err(format!(
                        "{}: va n con hasta, cada con hasta, o n con cada (n ≥ 1, cada > 0)",
                        r.location
                    ))
                }
            };
            if ts.is_empty() {
                return Err(format!("{}: no cae ningún golpe", r.location));
            }
            Ok(ts
                .iter()
                .enumerate()
                .map(|(i, x)| (format!("repetición {}", i + 1), *x))
                .collect())
        }
    }
}

/// `np.interp`: linear between points, flat outside.
pub(crate) fn interp(pts: &[(f64, f64)], t: f64) -> f64 {
    match pts.iter().position(|(x, _)| *x > t) {
        None => pts.last().map_or(1.0, |p| p.1),
        Some(0) => pts[0].1,
        Some(k) => {
            let ((t0, v0), (t1, v1)) = (pts[k - 1], pts[k]);
            if t1 > t0 {
                v0 + (v1 - v0) * (t - t0) / (t1 - t0)
            } else {
                v1
            }
        }
    }
}

fn tables<'a>(r: &Params<'a>, k: &str) -> Result<Vec<&'a toml::Table>, String> {
    match r.get(k) {
        None => Ok(Vec::new()),
        Some(toml::Value::Array(v)) => v
            .iter()
            .map(|x| x.as_table().ok_or(format!("[[{k}]]: no es una tabla")))
            .collect(),
        Some(_) => Err(format!("{k}: se esperaba [[{k}]]")),
    }
}

/// `[["evento", valor], …]`, in order.
fn points(v: &[toml::Value], times: &Timeline, location: &str) -> Result<Vec<(f64, f64)>, String> {
    let mut out: Vec<(f64, f64)> = Vec::new();
    for p in v {
        let pair = p.as_array().filter(|a| a.len() == 2).ok_or(format!(
            "{location}: envolvente lleva pares [\"evento\", valor]"
        ))?;
        let t = times
            .time(
                pair[0]
                    .as_str()
                    .ok_or(format!("{location}: el tiempo de la envolvente es texto"))?,
            )
            .map_err(|m| format!("{location}: {m}"))?;
        let x = pair[1]
            .as_float()
            .or(pair[1].as_integer().map(|n| n as f64))
            .ok_or(format!(
                "{location}: el valor de la envolvente es un número"
            ))?;
        if let Some(&(t0, _)) = out.last() {
            if t < t0 {
                return Err(format!(
                    "{location}: la envolvente va hacia atrás en «{}»",
                    pair[0]
                ));
            }
        }
        out.push((t, x));
    }
    Ok(out)
}

/// What every piece has: gain, pan, room send, bus and seed.
fn piece(
    d: &Params,
    source: Source,
    instrument: String,
    score_seed: u64,
    auto: u64,
    bus: &str,
) -> Result<Piece, String> {
    let n = match d.get("semilla") {
        None => auto,
        Some(toml::Value::Integer(n)) if *n >= 0 => *n as u64,
        Some(_) => return Err(format!("{}: semilla debe ser un entero ≥ 0", d.location)),
    };
    let pan = d.num_or("pan", 0.0)?;
    let pan_to = d.num("pan_hasta")?;
    for p in std::iter::once(pan).chain(pan_to) {
        if !(-1.0..=1.0).contains(&p) {
            return Err(format!("{}: pan = {p} (va de −1 a 1)", d.location));
        }
    }
    Ok(Piece {
        source,
        instrument,
        gain: d.num_or("ganancia", 1.0)?,
        pan,
        pan_to,
        send: d.num_or("sala", 0.3)?,
        bus: d.text("bus")?.unwrap_or(bus).to_string(),
        seed: seed_for(score_seed, n),
        location: d.location.clone(),
    })
}
