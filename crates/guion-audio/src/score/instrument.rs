//! The instruments a score plays, supplied by the caller (R-0011 AC5).
//!
//! The engine knows where a sound goes (its instant, gain, pan, room send, bus
//! and noise stream); what it sounds like is the brand's. A brand registers
//! each instrument by name, for one [`Kind`] of table, with a builder that
//! reads the instrument's own keys from [`Params`] and returns a [`Sound`].

use crate::synth::Noise;

use super::params::Params;

/// The table an instrument plays in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// `[[dron]]`: from sample 0 to the end, under an envelope and an LFO.
    Drone,
    /// `[[seguidor]]`: a tone that follows a series of `eventos.json`.
    Follower,
    /// `[[golpe]]`: a short sound at an instant.
    Hit,
}

impl Kind {
    /// The table name in the score.
    pub fn table(self) -> &'static str {
        match self {
            Kind::Drone => "dron",
            Kind::Follower => "seguidor",
            Kind::Hit => "golpe",
        }
    }
}

/// What a sound gets to render with.
pub struct Ctx<'a> {
    /// The track length in samples (a drone or follower fills it).
    pub len: usize,
    /// This sound's own noise stream (`seed_for(score seed, n)`).
    pub noise: &'a mut Noise,
    /// The seed of that stream, to split it further (`seed_for(seed, k)`).
    pub seed: u64,
    /// A follower's series, one value per sample (empty otherwise).
    pub series: &'a [f64],
}

/// What a sound renders: mono samples at [`super::RATE`] and, for a hit that
/// pans as it moves, the 0 → 1 curve the pan follows (one per sample; without
/// it the pan moves in a straight line).
pub struct Rendered {
    pub samples: Vec<f64>,
    pub pan_curve: Option<Vec<f64>>,
}

impl From<Vec<f64>> for Rendered {
    fn from(samples: Vec<f64>) -> Self {
        Rendered {
            samples,
            pan_curve: None,
        }
    }
}

/// A configured instrument: a pure function of its parameters and its noise.
pub trait Sound {
    /// The samples (and the pan curve, if any).
    fn render(&self, cx: &mut Ctx) -> Rendered;
    /// For a hit that lasts from `en` to a known end (a swell, a riser), its
    /// length in s: `golpes.tsv` reports the end. `None` for the rest.
    fn extent(&self) -> Option<f64> {
        None
    }
}

impl std::fmt::Debug for dyn Sound {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.extent() {
            Some(x) => write!(f, "Sound {{ extent: {x} }}"),
            None => f.write_str("Sound"),
        }
    }
}

/// A [`Sound`] from a closure, for instruments with no state but their
/// parameters.
pub struct FnSound<F> {
    f: F,
    extent: Option<f64>,
}

impl<F: Fn(&mut Ctx) -> Rendered> Sound for FnSound<F> {
    fn render(&self, cx: &mut Ctx) -> Rendered {
        (self.f)(cx)
    }
    fn extent(&self) -> Option<f64> {
        self.extent
    }
}

/// A boxed [`Sound`] from a closure.
pub fn sound<F: Fn(&mut Ctx) -> Rendered + 'static>(f: F) -> Box<dyn Sound> {
    Box::new(FnSound { f, extent: None })
}

/// A boxed [`Sound`] from a closure, with its extent for `golpes.tsv`.
pub fn sound_lasting<F: Fn(&mut Ctx) -> Rendered + 'static>(extent: f64, f: F) -> Box<dyn Sound> {
    Box::new(FnSound {
        f,
        extent: Some(extent),
    })
}

/// Reads an instrument's keys and returns the configured sound.
pub type Builder = Box<dyn Fn(&Params) -> Result<Box<dyn Sound>, String>>;

/// The instruments a brand supplies, by kind and name, in registration order
/// (an unknown name lists them in that order).
#[derive(Default)]
pub struct Instruments {
    entries: Vec<(Kind, String, Builder)>,
    defaults: Vec<(Kind, String)>,
}

impl Instruments {
    /// No instruments.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers `name` for tables of `kind`. A later registration of the
    /// same kind and name replaces the earlier one.
    pub fn add(
        &mut self,
        kind: Kind,
        name: &str,
        build: impl Fn(&Params) -> Result<Box<dyn Sound>, String> + 'static,
    ) -> &mut Self {
        self.entries.retain(|(k, n, _)| !(*k == kind && n == name));
        self.entries.push((kind, name.to_string(), Box::new(build)));
        self
    }

    /// The instrument a table of `kind` plays when it names none. Without a
    /// default, `instrumento` is required.
    pub fn set_default(&mut self, kind: Kind, name: &str) -> &mut Self {
        self.defaults.retain(|(k, _)| *k != kind);
        self.defaults.push((kind, name.to_string()));
        self
    }

    /// The default instrument of `kind`, if any.
    pub fn default_for(&self, kind: Kind) -> Option<&str> {
        self.defaults
            .iter()
            .find(|(k, _)| *k == kind)
            .map(|(_, n)| n.as_str())
    }

    /// The names registered for `kind`, in order.
    pub fn names(&self, kind: Kind) -> Vec<&str> {
        self.entries
            .iter()
            .filter(|(k, _, _)| *k == kind)
            .map(|(_, n, _)| n.as_str())
            .collect()
    }

    /// Builds `name` from `p`; an unknown name lists the valid ones.
    pub(crate) fn build(
        &self,
        kind: Kind,
        name: &str,
        p: &Params,
    ) -> Result<Box<dyn Sound>, String> {
        match self
            .entries
            .iter()
            .find(|(k, n, _)| *k == kind && n == name)
        {
            Some((_, _, b)) => b(p),
            None => Err(format!(
                "{}: instrumento «{name}» (valen: {})",
                p.location(),
                self.names(kind).join(", ")
            )),
        }
    }
}
