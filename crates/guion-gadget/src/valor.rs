//! Typed params and values with provenance (R-0015 AC1).
//!
//! Every number a reel shows has a provenance (the house rule: *medido*,
//! *modelado* or *reportado*, and never one presented as another). Here it
//! is a type: a physical param cannot be written without its
//! [`Procedencia`], a measured or reported one cannot be written without its
//! source, and every value a gadget computes leaves as
//! [`Procedencia::Calculado`].

use std::collections::BTreeMap;
use std::fmt;

/// Where a number comes from: the four tags of the inspector.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Procedencia {
    /// Computed by a model (a gadget's output, or an input computed
    /// elsewhere and cited).
    Calculado,
    /// Measured: a session, a sensor, a date. Needs a `fuente`.
    Medido,
    /// Reported by someone else: a source with a link. Needs a `fuente`.
    Reportado,
    /// Assumed: an example or a modelling choice, said as such.
    Supuesto,
}

impl Procedencia {
    /// Every tag, in the order the inspector lists them.
    pub const TODAS: [Procedencia; 4] = [
        Procedencia::Calculado,
        Procedencia::Medido,
        Procedencia::Reportado,
        Procedencia::Supuesto,
    ];

    /// The word in `escena.toml` and in the inspector.
    pub fn nombre(self) -> &'static str {
        match self {
            Procedencia::Calculado => "calculado",
            Procedencia::Medido => "medido",
            Procedencia::Reportado => "reportado",
            Procedencia::Supuesto => "supuesto",
        }
    }

    /// The tag a word names; an unknown word lists the valid ones.
    pub fn de(s: &str) -> Result<Procedencia, String> {
        Procedencia::TODAS
            .into_iter()
            .find(|p| p.nombre() == s)
            .ok_or_else(|| {
                format!("procedencia «{s}»: one of calculado, medido, reportado, supuesto")
            })
    }

    /// Whether a value with this tag must cite where it comes from.
    pub fn pide_fuente(self) -> bool {
        matches!(
            self,
            Procedencia::Calculado | Procedencia::Medido | Procedencia::Reportado
        )
    }
}

impl fmt::Display for Procedencia {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.nombre())
    }
}

/// A param's value: a number or a text (a label, an option, a palette token).
#[derive(Debug, Clone, PartialEq)]
pub enum Dato {
    Num(f64),
    Texto(String),
}

/// What kind of value a param holds, with its range and default.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Tipo {
    /// A number in `[min, max]`, in `unidad`, moved by `paso` on a slider.
    Num {
        min: f64,
        max: f64,
        paso: f64,
        unidad: &'static str,
        defecto: f64,
    },
    /// Free text (a label).
    Texto { defecto: &'static str },
    /// One of a fixed list.
    Opcion {
        opciones: &'static [&'static str],
        defecto: &'static str,
    },
    /// A palette token or role (`texto`, `acento`, `secundario`, a token name).
    Token { defecto: &'static str },
}

/// One param a gadget declares (AC1).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParamSpec {
    pub nombre: &'static str,
    /// What it is, in the creator's words (the inspector's tooltip).
    pub doc: &'static str,
    pub tipo: Tipo,
    /// A physical quantity: it must carry its [`Procedencia`]. Layout params
    /// (a position in px, a colour) do not.
    pub fisico: bool,
}

impl ParamSpec {
    /// The default value.
    pub fn defecto(&self) -> Dato {
        match self.tipo {
            Tipo::Num { defecto, .. } => Dato::Num(defecto),
            Tipo::Texto { defecto } | Tipo::Opcion { defecto, .. } | Tipo::Token { defecto } => {
                Dato::Texto(defecto.to_string())
            }
        }
    }

    /// The unit ("" for a text).
    pub fn unidad(&self) -> &'static str {
        match self.tipo {
            Tipo::Num { unidad, .. } => unidad,
            _ => "",
        }
    }

    /// Whether `d` fits this param's type and range; the error says why,
    /// without the location (the caller adds it).
    pub fn admite(&self, d: &Dato) -> Result<(), String> {
        match (self.tipo, d) {
            (
                Tipo::Num {
                    min, max, unidad, ..
                },
                Dato::Num(x),
            ) => {
                if x.is_finite() && *x >= min && *x <= max {
                    Ok(())
                } else {
                    Err(format!("{x} is outside [{min}, {max}] {unidad}"))
                }
            }
            (Tipo::Opcion { opciones, .. }, Dato::Texto(s)) => {
                if opciones.contains(&s.as_str()) {
                    Ok(())
                } else {
                    Err(format!("«{s}» is not one of {}", opciones.join(", ")))
                }
            }
            (Tipo::Texto { .. } | Tipo::Token { .. }, Dato::Texto(_)) => Ok(()),
            (Tipo::Num { .. }, Dato::Texto(s)) => Err(format!("«{s}» is not a number")),
            (_, Dato::Num(x)) => Err(format!("{x} is a number; this param is a text")),
        }
    }
}

/// One output a gadget declares: what other gadgets can bind to, what the
/// inspector shows and what goes to `eventos.json` as a series.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SalidaSpec {
    pub nombre: &'static str,
    pub unidad: &'static str,
    pub doc: &'static str,
}

/// A value a gadget computed at `t`, with its provenance (AC1).
#[derive(Debug, Clone, PartialEq)]
pub struct Valor {
    pub nombre: String,
    pub cantidad: f64,
    pub unidad: String,
    pub procedencia: Procedencia,
    /// How it was computed, in words ("τ = F·L·sen θ").
    pub nota: String,
}

impl Valor {
    /// A value computed by the gadget.
    pub fn calculado(nombre: &str, cantidad: f64, unidad: &str, nota: &str) -> Valor {
        Valor {
            nombre: nombre.to_string(),
            cantidad,
            unidad: unidad.to_string(),
            procedencia: Procedencia::Calculado,
            nota: nota.to_string(),
        }
    }
}

/// The params a gadget evaluates with: every declared param present (the
/// escena fills defaults and resolves animations and bindings first), each
/// with its provenance when physical.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Params {
    datos: BTreeMap<String, (Dato, Option<Procedencia>)>,
}

impl Params {
    /// Every param of `spec` at its default (physical ones as *supuesto*).
    pub fn defectos(spec: &[ParamSpec]) -> Params {
        let mut p = Params::default();
        for s in spec {
            let pr = s.fisico.then_some(Procedencia::Supuesto);
            p.datos.insert(s.nombre.to_string(), (s.defecto(), pr));
        }
        p
    }

    /// Sets one param.
    pub fn fijar(&mut self, nombre: &str, d: Dato, procedencia: Option<Procedencia>) {
        self.datos.insert(nombre.to_string(), (d, procedencia));
    }

    /// The number `nombre` holds. A name the gadget did not declare, or a
    /// text, is a bug in the gadget and reads as NaN (it never panics).
    pub fn num(&self, nombre: &str) -> f64 {
        match self.datos.get(nombre) {
            Some((Dato::Num(x), _)) => *x,
            _ => f64::NAN,
        }
    }

    /// The text `nombre` holds ("" if it is not a text).
    pub fn texto(&self, nombre: &str) -> &str {
        match self.datos.get(nombre) {
            Some((Dato::Texto(s), _)) => s,
            _ => "",
        }
    }

    /// The value `nombre` holds.
    pub fn dato(&self, nombre: &str) -> Option<&Dato> {
        self.datos.get(nombre).map(|(d, _)| d)
    }

    /// The provenance of `nombre` (None for a layout param).
    pub fn procedencia(&self, nombre: &str) -> Option<Procedencia> {
        self.datos.get(nombre).and_then(|(_, p)| *p)
    }
}
