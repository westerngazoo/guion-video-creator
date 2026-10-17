//! One table of the score, with the keys it accepts (R-0011 AC3, AC6).
//!
//! A [`Params`] keeps count of the keys that were read: at the end, one nobody
//! read is an error naming the table and the valid keys (a typo is never
//! ignored, CLAUDE.md §6). Instruments read their own keys through it.
//!
//! In a hit that repeats (`en` with several events, or `repetir`), a number
//! may be a list with one value per hit (`pan = [0.3, -0.3]`), and `paso`
//! adds `i ·` step to hit `i` (`paso = { octavas = 0.0833 }`).

use std::cell::{Cell, RefCell};

/// A table of the score as an instrument sees it. Every error names the table
/// (`[[golpe]] 3 («radio-2»)`).
pub struct Params<'a> {
    pub(crate) t: &'a toml::Table,
    pub(crate) location: String,
    read: RefCell<Vec<String>>,
    /// Hit `i` of `n`, while one of a repetition is being read.
    pub(crate) index: Cell<Option<(usize, usize)>>,
    pub(crate) step: Cell<Option<&'a toml::Table>>,
    /// A hit's length from `en` to `hasta`, s, when the table has `hasta`.
    pub(crate) span: Cell<Option<f64>>,
}

impl<'a> Params<'a> {
    pub(crate) fn new(t: &'a toml::Table, location: impl Into<String>) -> Self {
        Params {
            t,
            location: location.into(),
            read: Default::default(),
            index: Default::default(),
            step: Default::default(),
            span: Default::default(),
        }
    }

    /// Where this table is in the score, for error messages.
    pub fn location(&self) -> &str {
        &self.location
    }

    /// This hit's element of a per-hit list.
    fn of_index(&self, k: &str, v: &'a [toml::Value]) -> Result<&'a toml::Value, String> {
        match self.index.get() {
            Some((i, n)) if v.len() == n => Ok(&v[i]),
            Some((_, n)) => Err(format!(
                "{}: {k} tiene {} valores para {n} golpes",
                self.location,
                v.len()
            )),
            None => Err(format!(
                "{}: {k} es una lista, pero aquí va un solo valor",
                self.location
            )),
        }
    }

    /// The raw value of `k`, marked as read.
    pub fn get(&self, k: &str) -> Option<&'a toml::Value> {
        self.read.borrow_mut().push(k.to_string());
        self.t.get(k)
    }

    /// A number (an integer is a number), with this hit's list element and
    /// step applied.
    pub fn num(&self, k: &str) -> Result<Option<f64>, String> {
        let one = |v: &toml::Value| match v {
            toml::Value::Float(x) if x.is_finite() => Ok(*x),
            toml::Value::Integer(n) => Ok(*n as f64),
            _ => Err(format!("{}: {k} debe ser un número", self.location)),
        };
        let v = match self.get(k) {
            None => return Ok(None),
            Some(toml::Value::Array(v)) => one(self.of_index(k, v)?)?,
            Some(v) => one(v)?,
        };
        let step = match (self.step.get().and_then(|p| p.get(k)), self.index.get()) {
            (None, _) => 0.0,
            (Some(p), Some((i, _))) => i as f64 * one(p)?,
            (Some(_), None) => 0.0,
        };
        Ok(Some(v + step))
    }

    /// A number, or `def` when absent.
    pub fn num_or(&self, k: &str, def: f64) -> Result<f64, String> {
        Ok(self.num(k)?.unwrap_or(def))
    }

    /// A number that must be > 0, or `def` when absent.
    pub fn positive(&self, k: &str, def: f64) -> Result<f64, String> {
        let v = self.num_or(k, def)?;
        if v > 0.0 {
            Ok(v)
        } else {
            Err(format!("{}: {k} debe ser > 0 (es {v})", self.location))
        }
    }

    /// A text (or this hit's element of a list of texts).
    pub fn text(&self, k: &str) -> Result<Option<&'a str>, String> {
        let v = match self.get(k) {
            None => return Ok(None),
            Some(toml::Value::Array(v)) => self.of_index(k, v)?,
            Some(v) => v,
        };
        v.as_str()
            .map(Some)
            .ok_or(format!("{}: {k} debe ser texto", self.location))
    }

    /// A sub-table (`sala = { … }`).
    pub fn table(&self, k: &str) -> Result<Option<Params<'a>>, String> {
        match self.get(k) {
            None => Ok(None),
            Some(toml::Value::Table(t)) => {
                Ok(Some(Params::new(t, format!("{}.{k}", self.location))))
            }
            Some(_) => Err(format!("{}: {k} debe ser una tabla", self.location)),
        }
    }

    /// A list, as written.
    pub fn list(&self, k: &str) -> Result<Option<&'a Vec<toml::Value>>, String> {
        match self.get(k) {
            None => Ok(None),
            Some(toml::Value::Array(v)) => Ok(Some(v)),
            Some(_) => Err(format!("{}: {k} debe ser una lista", self.location)),
        }
    }

    /// A list of numbers (`voces = [55, 82.5]`).
    pub fn nums(&self, k: &str) -> Result<Option<Vec<f64>>, String> {
        let Some(v) = self.list(k)? else {
            return Ok(None);
        };
        v.iter()
            .map(|x| match x {
                toml::Value::Float(f) if f.is_finite() => Ok(*f),
                toml::Value::Integer(n) => Ok(*n as f64),
                _ => Err(format!(
                    "{}: {k} debe ser una lista de números",
                    self.location
                )),
            })
            .collect::<Result<Vec<_>, _>>()
            .map(Some)
    }

    /// A list of number pairs (`notas = [[220, 1.0], [330, 0.5]]`).
    pub fn pairs(&self, k: &str) -> Result<Option<Vec<(f64, f64)>>, String> {
        let Some(v) = self.list(k)? else {
            return Ok(None);
        };
        let bad = || format!("{}: cada elemento de {k} es [a, b]", self.location);
        v.iter()
            .map(|n| {
                let p = n.as_array().filter(|p| p.len() == 2).ok_or_else(bad)?;
                let x = |v: &toml::Value| {
                    v.as_float()
                        .or(v.as_integer().map(|n| n as f64))
                        .ok_or_else(bad)
                };
                Ok((x(&p[0])?, x(&p[1])?))
            })
            .collect::<Result<Vec<_>, String>>()
            .map(Some)
    }

    /// A hit's length: `dur`, or from `en` to `hasta` (not both), or `def`.
    pub fn duration(&self, def: f64) -> Result<f64, String> {
        match (self.span.get(), self.num("dur")?) {
            (Some(_), Some(_)) => Err(format!("{}: dur o hasta, no los dos", self.location)),
            (Some(h), None) => Ok(h),
            (None, Some(x)) if x > 0.0 => Ok(x),
            (None, Some(x)) => Err(format!("{}: dur debe ser > 0 (es {x})", self.location)),
            (None, None) => Ok(def),
        }
    }

    /// A frequency: `f0` (Hz, `def` when absent) times 2^`octavas`.
    pub fn frequency(&self, def: f64) -> Result<f64, String> {
        Ok(self.positive("f0", def)? * 2f64.powf(self.num_or("octavas", 0.0)?))
    }

    /// Fails if a key was left unread (or a `paso` of a key nobody read).
    pub(crate) fn finish(&self) -> Result<(), String> {
        let read = self.read.borrow();
        if let Some(p) = self.step.get() {
            for k in p.keys() {
                if !read.iter().any(|l| l == k) {
                    return Err(format!(
                        "{}: paso.{k} no es un número de este golpe",
                        self.location
                    ));
                }
            }
        }
        for k in self.t.keys() {
            if !read.iter().any(|l| l == k) {
                let mut valid: Vec<&str> = read.iter().map(String::as_str).collect();
                valid.sort_unstable();
                valid.dedup();
                return Err(format!(
                    "{}: clave desconocida «{k}» (valen: {})",
                    self.location,
                    valid.join(", ")
                ));
            }
        }
        Ok(())
    }
}
