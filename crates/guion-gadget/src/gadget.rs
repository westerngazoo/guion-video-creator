//! The [`Gadget`] contract (R-0015 AC1): typed params in, a pure
//! `eval(t, params) → (prims, eventos, valores)` out, plus asserts and
//! oracles.
//!
//! A gadget knows nothing about the reel around it: not the clock (the
//! escena hands it the time since its clip started and the params already
//! animated to that instant), not the account (the [`Contexto`] hands it the
//! canvas, the palette by role and the faces), not the file it was saved in.
//! That is what makes one gadget serve every account and keeps `eval` pure.

use std::collections::BTreeMap;

use guion_reel::Canvas;
use motoreel::{Prim2, Rgb};
use motoreel_typeset::{measure, FaceId, Fonts};

use crate::valor::{ParamSpec, Params, SalidaSpec, Valor};

/// The library's shelves (the prototype's categories).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Categoria {
    Fisica,
    Geometria,
    TresD,
    Datos,
    TextoVoz,
}

impl Categoria {
    /// The shelf's label in the studio.
    pub fn nombre(self) -> &'static str {
        match self {
            Categoria::Fisica => "Física",
            Categoria::Geometria => "Geometría",
            Categoria::TresD => "3D",
            Categoria::Datos => "Datos",
            Categoria::TextoVoz => "Texto y voz",
        }
    }
}

/// An account's colours: tokens by name, and the roles that name them
/// (`fondo`, `texto`, `secundario`, `acento`, `foco`). A gadget asks for a
/// role, so the same gadget takes each account's palette.
#[derive(Debug, Clone, PartialEq)]
pub struct Paleta {
    pub tokens: BTreeMap<String, Rgb>,
    pub roles: BTreeMap<String, String>,
}

impl Paleta {
    /// The neutral palette of a post with no account: near-black ground,
    /// warm white text, grey secondary, one amber accent.
    pub fn neutra() -> Paleta {
        let t = |n: &str, r, g, b| (n.to_string(), Rgb { r, g, b });
        let tokens = BTreeMap::from([
            t("negro", 10, 10, 11),
            t("hueso", 241, 239, 232),
            t("acero", 154, 156, 165),
            t("ambar", 233, 178, 63),
        ]);
        let roles = [
            ("fondo", "negro"),
            ("texto", "hueso"),
            ("secundario", "acero"),
            ("acento", "ambar"),
        ]
        .into_iter()
        .map(|(a, b)| (a.to_string(), b.to_string()))
        .collect();
        Paleta { tokens, roles }
    }

    /// The colour a role or a token names.
    pub fn color(&self, nombre: &str) -> Option<Rgb> {
        let token = self.roles.get(nombre).map_or(nombre, String::as_str);
        self.tokens.get(token).copied()
    }
}

/// What a gadget may know about where it is drawn.
pub struct Contexto<'a> {
    pub canvas: Canvas,
    pub paleta: &'a Paleta,
    /// The faces text is measured and set in; `None` draws no text (a post
    /// whose account has no faces yet), and the gadget says so in its avisos.
    pub caras: Option<&'a Fonts>,
    /// The face labels use.
    pub cara: usize,
    /// The safe area, `[x0, y0, x1, y1]` px: nothing a gadget places by
    /// itself (a label) leaves it.
    pub seguro: [f64; 4],
}

impl Contexto<'_> {
    /// A role's or token's colour; an unknown name (the escena rejects it at
    /// load, so only a gadget's own default can get here) is magenta, loud
    /// on screen and never a panic.
    pub fn color(&self, nombre: &str) -> Rgb {
        self.paleta.color(nombre).unwrap_or(Rgb {
            r: 255,
            g: 0,
            b: 255,
        })
    }

    /// The advance width of `s` at `tam` px in the label face, or `None`
    /// without faces or when the face lacks a character.
    pub fn ancho(&self, s: &str, tam: f64) -> Option<f64> {
        let f = self.caras?;
        measure(s, FaceId(self.cara), tam, f)
            .ok()
            .map(|n| n.metrics().width)
    }
}

/// What `eval` returns for one instant (AC1).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Salida {
    /// What to draw, in motoreel's view units (the escena applies the clip's
    /// opacity afterwards).
    pub prims: Vec<Prim2>,
    /// The named states that hold at `t` («maximo», «tope»): the escena
    /// writes the first frame each one holds to `eventos.json` as
    /// `<id>:<nombre>`, so the sound lands on the frame the picture does.
    pub eventos: Vec<String>,
    /// The values computed at `t`, with their provenance: the inspector shows
    /// them, other gadgets bind to them, and each is a series in
    /// `eventos.json`.
    pub valores: Vec<Valor>,
    /// What does not hold at `t` (a label that found no room, a gauge past
    /// its scale): the studio shows them and `guion studio --check` fails on
    /// any (AC1, the per-gadget asserts).
    pub avisos: Vec<String>,
}

/// One oracle: params, an instant, and the value a gadget must give,
/// computed by hand or by another model, with where the number comes from.
#[derive(Debug, Clone, PartialEq)]
pub struct Oraculo {
    pub nombre: &'static str,
    /// Params that differ from the defaults.
    pub params: Vec<(&'static str, f64)>,
    /// The output it checks.
    pub salida: &'static str,
    pub esperado: f64,
    pub tolerancia: f64,
    /// Where the expected number comes from.
    pub fuente: &'static str,
}

/// A gadget: one reusable piece of a reel (R-0015 AC1).
///
/// The contract every implementation keeps, and that
/// [`crate::registro::comprobar_contrato`] tests for each registered gadget:
///
/// - **Pure.** [`Gadget::eval`] reads only its arguments: no clock, no
///   environment, no hash-map order, no state between calls. The same `t`
///   and params give the same primitives, bit for bit, so the same frame.
/// - **Typed.** Every param is in [`Gadget::params`], with its range and
///   unit; physical ones carry a provenance.
/// - **Honest.** Every computed value leaves as `calculado`; the physics
///   comes from physics-lab, never re-derived here.
/// - **Checked.** [`Gadget::comprobar`] holds its static asserts, its
///   [`Salida::avisos`] the ones at an instant, and [`Gadget::oraculos`] the
///   numbers it must reproduce.
pub trait Gadget: Send + Sync {
    /// The name `escena.toml` uses (`tipo = "palanca"`): a slug.
    fn tipo(&self) -> &'static str;
    /// The shelf it sits on.
    fn categoria(&self) -> Categoria;
    /// One line for the library: what it shows, and its formula.
    fn doc(&self) -> &'static str;
    /// The params, in the order the inspector and the file list them.
    fn params(&self) -> &'static [ParamSpec];
    /// The outputs.
    fn salidas(&self) -> &'static [SalidaSpec];
    /// The gadget at `t` seconds since its clip started, with `p` already at
    /// that instant.
    fn eval(&self, t: f64, p: &Params, cx: &Contexto) -> Salida;
    /// Asserts on the params themselves, beyond type and range (each a
    /// sentence naming the param). None by default.
    fn comprobar(&self, _p: &Params, _cx: &Contexto) -> Vec<String> {
        Vec::new()
    }
    /// The numbers it must reproduce.
    fn oraculos(&self) -> Vec<Oraculo> {
        Vec::new()
    }
}
