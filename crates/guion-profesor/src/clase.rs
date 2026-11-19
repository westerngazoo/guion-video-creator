//! The teacher-mode screenplay: a guion class (RFC-0002's `[class]`,
//! `[narration]`, `[[beat]]` with a spoken `say`) extended with what a
//! classroom video needs — chapters, a board, the simulation the teacher
//! runs, and the questions the player pauses on.

use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;

use serde::Deserialize;

/// A whole class.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Clase {
    pub class: Meta,
    #[serde(default)]
    pub narration: Narracion,
    pub escena: Escena,
    #[serde(default)]
    pub beat: Vec<Beat>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Meta {
    pub title: String,
    pub slug: String,
    /// The line above the chapter title (world and star).
    pub kicker: String,
    /// Who teaches: the guide's name, drawn next to the voice meter.
    #[serde(default = "default_guide")]
    pub guide: String,
    #[serde(default = "default_lang")]
    pub lang: String,
    #[serde(default = "default_fps")]
    pub fps: f64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Narracion {
    #[serde(default = "default_provider")]
    pub provider: String,
    #[serde(default = "default_voice")]
    pub voice: String,
    /// Rate multiplier over the voice's natural 175 words per minute.
    #[serde(default = "default_speed")]
    pub speed: f64,
    /// Silence after each beat, in seconds.
    #[serde(default = "default_pause")]
    pub pause: f64,
}

impl Default for Narracion {
    fn default() -> Self {
        Narracion {
            provider: default_provider(),
            voice: default_voice(),
            speed: default_speed(),
            pause: default_pause(),
        }
    }
}

/// The simulation on stage. The physics is closed form ([`crate::fisica`]).
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Escena {
    /// A cart braking at constant deceleration towards a cliff.
    Frenado {
        v0: f64,
        bandera: f64,
        acantilado: f64,
    },
    /// A projectile from the origin; optionally a target, a wall and a zone.
    Tiro {
        #[serde(default)]
        blanco: Option<f64>,
        #[serde(default)]
        muralla: Option<Muralla>,
        #[serde(default)]
        zona: Option<[f64; 2]>,
        /// The farthest x the stage shows, in metres.
        alcance: f64,
    },
    /// A block released on a ramp with friction.
    Plano { largo: f64, mu: f64 },
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Muralla {
    pub x: f64,
    pub ancho: f64,
    pub alto: f64,
}

/// One beat: say a line while the board and the stage show something.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Beat {
    pub say: String,
    /// Starts a new chapter with this title (the board is wiped).
    #[serde(default)]
    pub capitulo: Option<String>,
    /// Lines the teacher writes on the board during this beat.
    #[serde(default)]
    pub pizarra: Vec<String>,
    /// A simulation run during this beat: the scene's parameters by name.
    #[serde(default)]
    pub demo: Option<BTreeMap<String, f64>>,
    /// A question: the classroom player pauses when the line ends.
    #[serde(default)]
    pub pregunta: Option<Pregunta>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Pregunta {
    pub prompt: String,
    pub opciones: Vec<String>,
    pub correcta: String,
    pub explica: String,
}

/// Why a class cannot be rendered — named by field, before any file is written.
#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Toml(toml::de::Error),
    Invalid(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(e) => write!(f, "{e}"),
            Error::Toml(e) => write!(f, "{e}"),
            Error::Invalid(m) => write!(f, "{m}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

pub fn load(path: &Path) -> Result<Clase, Error> {
    parse(&std::fs::read_to_string(path)?)
}

pub fn parse(src: &str) -> Result<Clase, Error> {
    let c: Clase = toml::from_str(src).map_err(Error::Toml)?;
    validate(&c)?;
    Ok(c)
}

fn invalid(m: impl Into<String>) -> Error {
    Error::Invalid(m.into())
}

/// The parameters a `demo` must name for each scene.
pub fn demo_params(e: &Escena) -> &'static [&'static str] {
    match e {
        Escena::Frenado { .. } => &["a"],
        Escena::Tiro { .. } => &["th", "v0"],
        Escena::Plano { .. } => &["th"],
    }
}

fn validate(c: &Clase) -> Result<(), Error> {
    let slug_ok = !c.class.slug.is_empty()
        && c.class
            .slug
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-');
    if !slug_ok {
        return Err(invalid("class.slug: lowercase ascii, digits and '-' only"));
    }
    if !(c.class.fps.is_finite() && c.class.fps > 0.0) {
        return Err(invalid("class.fps: must be finite and > 0"));
    }
    if c.narration.provider != "say" {
        return Err(invalid("narration.provider: only \"say\" is supported"));
    }
    if !(c.narration.speed.is_finite() && c.narration.speed > 0.0) {
        return Err(invalid("narration.speed: must be finite and > 0"));
    }
    if !(c.narration.pause.is_finite() && c.narration.pause >= 0.0) {
        return Err(invalid("narration.pause: must be finite and >= 0"));
    }
    validate_escena(&c.escena)?;
    if c.beat.is_empty() {
        return Err(invalid("beat: a class needs at least one beat"));
    }
    if c.beat[0].capitulo.is_none() {
        return Err(invalid(
            "beat[0].capitulo: the first beat must open a chapter",
        ));
    }
    let need = demo_params(&c.escena);
    for (i, b) in c.beat.iter().enumerate() {
        if b.say.trim().is_empty() {
            return Err(invalid(format!("beat[{i}].say: empty line")));
        }
        if let Some(d) = &b.demo {
            for k in need {
                match d.get(*k) {
                    Some(v) if v.is_finite() && *v > 0.0 => {}
                    _ => {
                        return Err(invalid(format!(
                            "beat[{i}].demo.{k}: required, finite and > 0"
                        )))
                    }
                }
            }
            if let Some(k) = d.keys().find(|k| !need.contains(&k.as_str())) {
                return Err(invalid(format!(
                    "beat[{i}].demo.{k}: unknown for this scene"
                )));
            }
        }
        if let Some(q) = &b.pregunta {
            if q.opciones.len() < 2 || !q.opciones.contains(&q.correcta) {
                return Err(invalid(format!(
                    "beat[{i}].pregunta: needs two or more options and `correcta` among them"
                )));
            }
        }
    }
    Ok(())
}

fn validate_escena(e: &Escena) -> Result<(), Error> {
    let pos = |v: f64| v.is_finite() && v > 0.0;
    let ok = match e {
        Escena::Frenado {
            v0,
            bandera,
            acantilado,
        } => pos(*v0) && pos(*bandera) && bandera < acantilado,
        Escena::Tiro {
            blanco,
            muralla,
            zona,
            alcance,
        } => {
            pos(*alcance)
                && blanco.is_none_or(pos)
                && muralla.is_none_or(|m| pos(m.x) && pos(m.ancho) && pos(m.alto))
                && zona.is_none_or(|z| pos(z[0]) && z[0] < z[1])
        }
        Escena::Plano { largo, mu } => pos(*largo) && *largo > 1.0 && mu.is_finite() && *mu >= 0.0,
    };
    if ok {
        Ok(())
    } else {
        Err(invalid(
            "escena: parameters must be finite and positive (bandera < acantilado, zona ordered)",
        ))
    }
}

fn default_guide() -> String {
    "Nova".into()
}
fn default_lang() -> String {
    "es".into()
}
fn default_fps() -> f64 {
    30.0
}
fn default_provider() -> String {
    "say".into()
}
fn default_voice() -> String {
    "Paulina".into()
}
fn default_speed() -> f64 {
    1.0
}
fn default_pause() -> f64 {
    0.6
}

#[cfg(test)]
mod tests {
    use super::*;

    const OK: &str = r#"
        [class]
        title = "Frenado"
        slug = "frenado"
        kicker = "Mecánica"
        [escena]
        kind = "frenado"
        v0 = 8.0
        bandera = 16.0
        acantilado = 20.0
        [[beat]]
        capitulo = "La pregunta"
        say = "Hola."
        demo = { a = 4.0 }
    "#;

    #[test]
    fn a_minimal_class_loads_with_defaults() {
        let c = parse(OK).unwrap();
        assert_eq!(c.narration.voice, "Paulina");
        assert_eq!(c.class.fps, 30.0);
        assert_eq!(
            c.escena,
            Escena::Frenado {
                v0: 8.0,
                bandera: 16.0,
                acantilado: 20.0
            }
        );
    }

    #[test]
    fn the_first_beat_must_open_a_chapter() {
        let src = OK.replace("capitulo = \"La pregunta\"", "");
        assert!(parse(&src)
            .unwrap_err()
            .to_string()
            .contains("beat[0].capitulo"));
    }

    #[test]
    fn a_demo_names_the_scene_parameters() {
        let src = OK.replace("a = 4.0", "th = 4.0");
        assert!(parse(&src).unwrap_err().to_string().contains("demo.a"));
    }

    #[test]
    fn a_question_needs_its_answer_among_the_options() {
        let src = format!(
            "{OK}\n[[beat]]\nsay = \"¿?\"\npregunta = {{ prompt = \"x\", opciones = [\"a\", \"b\"], correcta = \"c\", explica = \"\" }}\n"
        );
        assert!(parse(&src).unwrap_err().to_string().contains("pregunta"));
    }
}
