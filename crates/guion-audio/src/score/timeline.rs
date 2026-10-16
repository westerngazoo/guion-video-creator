//! The clock of a score: what the scene wrote in `eventos.json` (R-0011 AC2).
//!
//! A score never writes a time by hand. It names an event, with an offset if
//! it needs one: `"qed"`, `"qed - 1.2"`, `"q2-fin + 0.5"`, `"fin - 0.5"`,
//! `"0"`, or a fraction of the span between two: `"m1..m1-fin @ 0.69"`.

use std::fmt;

use serde::de::{Deserializer, MapAccess, Visitor};
use serde::Deserialize;
use serde_json::Value;

use crate::synth::breakpoints;

use super::RATE;

/// A series of `eventos.json`: one value every `dt` s from `t0`.
#[derive(Clone, Debug)]
pub struct Series {
    pub t0: f64,
    pub dt: f64,
    pub v: Vec<f64>,
}

impl Series {
    /// The series at each of `n` samples, linear between its points and flat
    /// outside them (`np.interp`).
    pub fn samples(&self, n: usize) -> Vec<f64> {
        let pts: Vec<(f64, f64)> = self
            .v
            .iter()
            .enumerate()
            .map(|(i, v)| (self.t0 + self.dt * i as f64, *v))
            .collect();
        breakpoints(n, RATE, &pts)
    }
}

/// What the scene wrote in `eventos.json`: its length, its frame rate, its
/// named instants (in file order; the first of a repeated name wins) and its
/// series.
#[derive(Clone, Debug)]
pub struct Timeline {
    pub duration: f64,
    /// Frames per second of the scene (for `cada_cuadros`); 30 if absent.
    pub fps: f64,
    pub events: Vec<(String, f64)>,
    pub series: Vec<(String, Series)>,
}

/// A JSON object read in file order, repeated keys kept.
struct Ordered(Vec<(String, Value)>);

impl<'de> Deserialize<'de> for Ordered {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = Ordered;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("un objeto")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut m: A) -> Result<Ordered, A::Error> {
                let mut out = Vec::new();
                while let Some((k, v)) = m.next_entry::<String, Value>()? {
                    out.push((k, v));
                }
                Ok(Ordered(out))
            }
        }
        d.deserialize_map(V)
    }
}

#[derive(Deserialize)]
struct Raw {
    duracion: Option<Value>,
    fps: Option<Value>,
    eventos: Option<Ordered>,
    series: Option<Ordered>,
}

impl Timeline {
    /// Reads `eventos.json`. Errors name the event or series.
    pub fn from_json(text: &str) -> Result<Timeline, String> {
        let j: Raw = serde_json::from_str(text).map_err(|e| format!("eventos.json: {e}"))?;
        let duration = j
            .duracion
            .as_ref()
            .and_then(Value::as_f64)
            .ok_or("eventos.json: falta duracion")?;
        let mut events = Vec::new();
        for (k, x) in j.eventos.map(|o| o.0).unwrap_or_default() {
            let t = x
                .as_f64()
                .ok_or(format!("eventos.json: «{k}» no es un número"))?;
            events.push((k, t));
        }
        let mut series = Vec::new();
        for (k, s) in j.series.map(|o| o.0).unwrap_or_default() {
            let num = |c: &str| {
                s.get(c)
                    .and_then(Value::as_f64)
                    .ok_or(format!("eventos.json: la serie «{k}» no tiene {c}"))
            };
            let vals = s
                .get("v")
                .and_then(Value::as_array)
                .ok_or(format!("eventos.json: la serie «{k}» no tiene v"))?
                .iter()
                .map(|x| {
                    x.as_f64().ok_or(format!(
                        "eventos.json: la serie «{k}» tiene un valor que no es número"
                    ))
                })
                .collect::<Result<Vec<_>, _>>()?;
            if vals.is_empty() {
                return Err(format!("eventos.json: la serie «{k}» está vacía"));
            }
            let (t0, dt) = (num("t0")?, num("dt")?);
            series.push((k, Series { t0, dt, v: vals }));
        }
        let fps = j.fps.as_ref().and_then(Value::as_f64).unwrap_or(30.0);
        Ok(Timeline {
            duration,
            fps,
            events,
            series,
        })
    }

    /// The instant of event `name`, if the scene wrote it.
    pub fn event(&self, name: &str) -> Option<f64> {
        self.events.iter().find(|(k, _)| k == name).map(|(_, t)| *t)
    }

    /// The series `name`, if the scene wrote it.
    pub fn series(&self, name: &str) -> Option<&Series> {
        self.series.iter().find(|(k, _)| k == name).map(|(_, s)| s)
    }

    /// A base: a number, `fin` or an event.
    fn base(&self, s: &str) -> Option<f64> {
        if s == "fin" {
            return Some(self.duration);
        }
        self.event(s)
            .or_else(|| s.parse::<f64>().ok().filter(|x| x.is_finite()))
    }

    /// An instant of the score: `"qed"`, `"qed - 1.2"`, `"fin"`, `"0"`, or a
    /// fraction of the span between two: `"m1..m1-fin @ 0.69"`. An exact
    /// event name wins (`"menos-1"` is the event «menos-1»); to offset, the
    /// sign goes between spaces (or attached, when the whole is no event).
    pub fn time(&self, expr: &str) -> Result<f64, String> {
        let e = expr.trim();
        if let Some(t) = self.base(e) {
            return Ok(t);
        }
        if let Some((span, p)) = e.split_once('@') {
            let (a, b) = span
                .split_once("..")
                .ok_or(format!("«{expr}»: la fracción va como «a..b @ p»"))?;
            let p: f64 = p
                .trim()
                .parse()
                .map_err(|_| format!("«{expr}»: la fracción tras @ es un número"))?;
            let (ta, tb) = (self.time(a)?, self.time(b)?);
            return Ok(ta + p * (tb - ta));
        }
        for (i, c) in e.char_indices().rev() {
            if (c == '+' || c == '-') && i > 0 {
                let (a, b) = (e[..i].trim(), e[i + 1..].trim());
                if let (Some(t), Ok(d)) = (self.base(a), b.parse::<f64>()) {
                    if d.is_finite() {
                        return Ok(if c == '+' { t + d } else { t - d });
                    }
                }
            }
        }
        Err(format!(
            "«{expr}»: no es un evento de eventos.json, ni «evento ± s», ni fin, ni un número"
        ))
    }
}
