//! One clock for picture and sound: the named instants of a reel and the
//! curves its sound follows, written as `events.json` (AC1).
//!
//! The scene names its instants once ([`Clock::event`]) and samples whatever
//! the sound should follow ([`Clock::series`], e.g. the angle of an arrow).
//! The sound design then reads them from the file and never writes a time by
//! hand: the hit lands on the frame where the stamp lands because both read
//! the same number.
//!
//! ## The file (schema v1)
//!
//! One JSON object, keys in this order, no whitespace:
//!
//! ```text
//! {"duracion":D,"fps":F,"eventos":{"<name>":T,...},"series":{"<name>":{"t0":T0,"dt":DT,"v":[V,...]},...}}
//! ```
//!
//! Every number is written with exactly six decimals (`{:.6}`); events and
//! series keep the order the scene declared them in. The key names are
//! Spanish because the first studio that wrote the file (rotorf-sico) and every
//! sound design that reads it use them; changing them would change every
//! file's bytes.

use crate::error::ReelError;
use crate::json;

/// A sampled curve: `values[k]` is the value at `t0 + k·dt` seconds.
#[derive(Debug, Clone, PartialEq)]
pub struct Series {
    pub name: String,
    pub t0: f64,
    pub dt: f64,
    pub values: Vec<f64>,
}

/// The instants of a reel and the curves its sound follows.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Clock {
    events: Vec<(String, f64)>,
    series: Vec<Series>,
}

fn invalid(name: &str, reason: String) -> ReelError {
    ReelError::InvalidClock {
        name: name.to_string(),
        reason,
    }
}

impl Clock {
    pub fn new() -> Clock {
        Clock::default()
    }

    /// A named instant at `t` seconds. `t` must be finite and not negative.
    pub fn event(mut self, name: &str, t: f64) -> Result<Clock, ReelError> {
        if !(t.is_finite() && t >= 0.0) {
            return Err(invalid(
                name,
                format!("an event needs a finite t ≥ 0, got {t}"),
            ));
        }
        self.events.push((name.to_string(), t));
        Ok(self)
    }

    /// A sampled curve: `values` every `dt` seconds from `t0`. Every number
    /// must be finite (JSON has no NaN) and `dt` positive.
    pub fn series(
        mut self,
        name: &str,
        t0: f64,
        dt: f64,
        values: Vec<f64>,
    ) -> Result<Clock, ReelError> {
        if !(t0.is_finite() && dt.is_finite() && dt > 0.0) {
            return Err(invalid(
                name,
                format!("a series needs a finite t0 and dt > 0, got t0 = {t0}, dt = {dt}"),
            ));
        }
        if let Some(k) = values.iter().position(|v| !v.is_finite()) {
            return Err(invalid(name, format!("sample {k} is {}", values[k])));
        }
        self.series.push(Series {
            name: name.to_string(),
            t0,
            dt,
            values,
        });
        Ok(self)
    }

    /// The events, in the order they were declared.
    pub fn events(&self) -> &[(String, f64)] {
        &self.events
    }

    /// The series, in the order they were declared.
    pub fn series_list(&self) -> &[Series] {
        &self.series
    }

    /// The `events.json` text for a reel of `duration` seconds at `fps`.
    pub fn to_json(&self, duration: f64, fps: f64) -> String {
        let mut s = String::new();
        s.push_str("{\"duracion\":");
        json::number(&mut s, duration);
        s.push_str(",\"fps\":");
        json::number(&mut s, fps);
        s.push_str(",\"eventos\":{");
        for (k, (name, t)) in self.events.iter().enumerate() {
            if k > 0 {
                s.push(',');
            }
            json::string(&mut s, name);
            s.push(':');
            json::number(&mut s, *t);
        }
        s.push_str("},\"series\":{");
        for (k, se) in self.series.iter().enumerate() {
            if k > 0 {
                s.push(',');
            }
            json::string(&mut s, &se.name);
            s.push_str(":{\"t0\":");
            json::number(&mut s, se.t0);
            s.push_str(",\"dt\":");
            json::number(&mut s, se.dt);
            s.push_str(",\"v\":[");
            for (i, v) in se.values.iter().enumerate() {
                if i > 0 {
                    s.push(',');
                }
                json::number(&mut s, *v);
            }
            s.push_str("]}");
        }
        s.push_str("}}");
        s
    }
}
