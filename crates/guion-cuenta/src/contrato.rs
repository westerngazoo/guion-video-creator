//! `cuenta.toml`: the account contract (R-0014 AC1).
//!
//! What an account provides, in one file at its root: its identity and voice,
//! its brand tokens (palette roles, faces, subtitle box, safe area, the gate's
//! limits), and its domain reviewer. The shared layer (the pipeline, the gate,
//! voice and score, the dashboard, the scaffold, the generic process and the
//! reviewer prompts) reads it and holds no value of its own.
//!
//! ```toml
//! [cuenta]
//! nombre = "El extraño arte de computar"
//! slug = "extrano-arte"            # folders, the scene crate, publicar/<slug>/
//! handle = ""                      # empty: not reserved yet
//! idioma = "es"                    # es | en: what is published
//! tema = "tecnología, IA y computación"
//! publico = "…"
//! dueno = "…"
//! creada = "2026-10-18"
//!
//! [voz]
//! modelo = "es-carlfm-x-low"       # the Piper model (voz.toml must name it)
//! tono = "…"
//! trato = "tuteo"
//! prohibido = ["increíble"]
//!
//! [marca]
//! direccion = "por decidir"        # the design direction, decided by hand
//! logo = ""                        # phase 2 (R-0014 §5)
//! lienzo = { ancho = 1080, alto = 1920, fps = 30.0 }
//! [marca.tokens]                   # name = [r, g, b]
//! [marca.roles]                    # fondo, texto, secundario, acento, foco
//! [marca.fuentes]                  # titulo, texto: TTF/OTF, relative to the root
//! [marca.titulo]                   # tam (em px), y (baseline)
//! [marca.subtitulo]                # the subtitle box and its rules
//! [marca.seguro]                   # the safe area
//! [compuerta]                      # the gate's motion, audio and caption limits
//! [revisor]                        # the domain reviewer: nombre, materia, reglas
//! ```

use std::collections::BTreeMap;
use std::path::Path;

use serde::Deserialize;

/// The file's name, at an account's root.
pub const ARCHIVO: &str = "cuenta.toml";

/// The languages a published account may use.
pub const IDIOMAS: [&str; 2] = ["es", "en"];

/// The whole contract.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cuenta {
    pub cuenta: Identidad,
    pub voz: Voz,
    pub marca: Marca,
    #[serde(default)]
    pub compuerta: Compuerta,
    pub revisor: Revisor,
    /// The account's own studio gadgets (R-0015), if it has any.
    #[serde(default)]
    pub estudio: Option<Estudio>,
}

/// The account's gadgets for the studio (R-0015 AC3): Rust in its motor
/// crate, whose `bin` registers them next to the built-ins and serves the
/// studio (`guion_estudio::run_con`). `guion studio` reads `gadgets` to say
/// which binary a post that uses one needs.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Estudio {
    /// The `tipo` of each gadget the account adds.
    pub gadgets: Vec<String>,
    /// The crate that holds them, relative to the account's root.
    #[serde(default = "Estudio::motor_por_defecto")]
    pub motor: String,
    /// Its binary that serves the studio.
    #[serde(default = "Estudio::bin_por_defecto")]
    pub bin: String,
}

impl Estudio {
    fn motor_por_defecto() -> String {
        "motor/Cargo.toml".into()
    }
    fn bin_por_defecto() -> String {
        "estudio".into()
    }
}

/// Who the account is.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Identidad {
    pub nombre: String,
    pub slug: String,
    /// The Instagram user; empty while it is not reserved.
    #[serde(default)]
    pub handle: String,
    pub idioma: String,
    pub tema: String,
    #[serde(default)]
    pub publico: String,
    #[serde(default)]
    pub dueno: String,
    pub creada: String,
}

/// How the account speaks.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Voz {
    /// The voice model (`reel.toml` `[voice] model`).
    pub modelo: String,
    #[serde(default)]
    pub tono: String,
    #[serde(default)]
    pub trato: String,
    /// Words never published.
    #[serde(default)]
    pub prohibido: Vec<String>,
}

/// The brand tokens: everything a neutral primitive or the gate needs.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Marca {
    #[serde(default)]
    pub direccion: String,
    #[serde(default)]
    pub logo: String,
    pub lienzo: Lienzo,
    pub tokens: BTreeMap<String, [u8; 3]>,
    pub roles: Roles,
    pub fuentes: Fuentes,
    pub titulo: Titulo,
    pub subtitulo: Subtitulo,
    pub seguro: Seguro,
    /// An optional grid behind everything (a PCB, a ruled page), drawn in a
    /// background token, so the gate counts it as empty.
    #[serde(default)]
    pub rejilla: Option<Rejilla>,
}

/// The grid: lines every `paso` px, `grosor` px wide, in `token`; with
/// `nodos` = k > 0, a square pad on every k-th crossing.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rejilla {
    pub paso: f64,
    pub token: String,
    #[serde(default = "dos")]
    pub grosor: f64,
    #[serde(default)]
    pub nodos: usize,
}

fn dos() -> f64 {
    2.0
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Lienzo {
    pub ancho: u32,
    pub alto: u32,
    pub fps: f64,
}

/// What each token is for. Every name must be a key of `[marca.tokens]`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Roles {
    /// What counts as empty: the background and its panels (the first is the
    /// background a frame is cleared to).
    pub fondo: Vec<String>,
    /// The text: titles and subtitles.
    pub texto: String,
    /// Secondary text (the handle on the closing card).
    pub secundario: String,
    /// The one accent: at most one region per frame.
    pub acento: Vec<String>,
    /// The focus colour: at most one region per frame; empty, not checked.
    #[serde(default)]
    pub foco: Vec<String>,
}

/// The faces, relative to the account's root (or absolute).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fuentes {
    pub titulo: String,
    pub texto: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Titulo {
    /// The size, px of em.
    pub tam: f64,
    /// The baseline, px.
    pub y: f64,
}

/// The subtitle: its box and its rules (the gate's `[subtitle]`).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Subtitulo {
    /// The size, px of em.
    pub tam: f64,
    pub ancho: f64,
    pub base: f64,
    pub franja: [f64; 2],
    pub palabras: usize,
    pub fundido: f64,
    pub linea_larga: f64,
    #[serde(default)]
    pub prohibidos: String,
    #[serde(default)]
    pub sin_cifras: bool,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Seguro {
    pub x: [f64; 2],
    pub y: [f64; 2],
    /// Rectangles exempt (a watermark), `[x0, y0, x1, y1]`.
    #[serde(default)]
    pub excepto: Vec<[f64; 4]>,
}

/// The gate's limits that are not drawing: motion, audio, caption. Defaults
/// are the platform's (Instagram reels) and the house's measured values.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Compuerta {
    pub quieto_con_voz: f64,
    pub cierre_quieto: f64,
    pub lufs: f64,
    pub tolerancia_lufs: f64,
    pub pico: f64,
    pub hueco_corto: f64,
    pub hueco_max_db: f64,
    pub eventos_de_beat: Vec<String>,
    pub max_hashtags: usize,
    pub max_caracteres: usize,
    pub unidades: Vec<String>,
    pub interno: Vec<String>,
}

impl Default for Compuerta {
    fn default() -> Compuerta {
        Compuerta {
            quieto_con_voz: 4.0,
            cierre_quieto: 1.5,
            lufs: -14.0,
            tolerancia_lufs: 0.5,
            pico: -1.0,
            hueco_corto: 0.5,
            hueco_max_db: -15.0,
            eventos_de_beat: vec!["beat-".into()],
            max_hashtags: 5,
            max_caracteres: 2200,
            unidades: Vec::new(),
            interno: Vec::new(),
        }
    }
}

/// The domain reviewer: the agent that checks the subject before design.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Revisor {
    /// Its name: `.claude/agents/<nombre>.md`.
    pub nombre: String,
    /// What it knows: «física del racing», «computación, IA y tecnología».
    pub materia: String,
    /// The rules it enforces, one sentence each.
    #[serde(default)]
    pub reglas: Vec<String>,
}

/// A slug: lowercase ASCII letters, digits and inner hyphens, starting with a
/// letter (it names a Cargo package).
pub fn slug_valido(s: &str) -> bool {
    s.starts_with(|c: char| c.is_ascii_lowercase())
        && !s.ends_with('-')
        && !s.contains("--")
        && s.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// A date `AAAA-MM-DD`.
pub fn fecha_valida(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 10
        && b[4] == b'-'
        && b[7] == b'-'
        && b.iter()
            .enumerate()
            .all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit())
        && (1..=12).contains(&s[5..7].parse::<u8>().unwrap_or(0))
        && (1..=31).contains(&s[8..10].parse::<u8>().unwrap_or(0))
}

impl Cuenta {
    /// Parses and validates the contract's text.
    pub fn parse(text: &str) -> Result<Cuenta, String> {
        let c: Cuenta = toml::from_str(text).map_err(|e| e.to_string())?;
        c.validar()?;
        Ok(c)
    }

    /// Reads `<raiz>/cuenta.toml`.
    pub fn load(raiz: &Path) -> Result<Cuenta, String> {
        let p = raiz.join(ARCHIVO);
        let text = std::fs::read_to_string(&p).map_err(|e| format!("{}: {e}", p.display()))?;
        Cuenta::parse(&text).map_err(|e| format!("{}: {e}", p.display()))
    }

    /// Every rule a value must keep, each failure naming its key.
    pub fn validar(&self) -> Result<(), String> {
        let i = &self.cuenta;
        if i.nombre.trim().is_empty() {
            return Err("cuenta.nombre is empty".into());
        }
        if !slug_valido(&i.slug) {
            return Err(format!(
                "cuenta.slug «{}»: lowercase letters, digits and hyphens, starting with a letter",
                i.slug
            ));
        }
        if !IDIOMAS.contains(&i.idioma.as_str()) {
            return Err(format!("cuenta.idioma «{}»: es or en", i.idioma));
        }
        if !i.handle.is_empty() && !i.handle.starts_with('@') {
            return Err(format!("cuenta.handle «{}» starts with @", i.handle));
        }
        if !fecha_valida(&i.creada) {
            return Err(format!("cuenta.creada «{}»: AAAA-MM-DD", i.creada));
        }
        if self.voz.modelo.trim().is_empty() {
            return Err("voz.modelo is empty".into());
        }
        let m = &self.marca;
        if m.lienzo.ancho == 0 || m.lienzo.alto == 0 || m.lienzo.fps.is_nan() || m.lienzo.fps <= 0.0
        {
            return Err("marca.lienzo: ancho, alto and fps must be > 0".into());
        }
        let r = &m.roles;
        let nombres = r
            .fondo
            .iter()
            .map(|n| ("marca.roles.fondo", n))
            .chain([("marca.roles.texto", &r.texto)])
            .chain([("marca.roles.secundario", &r.secundario)])
            .chain(r.acento.iter().map(|n| ("marca.roles.acento", n)))
            .chain(r.foco.iter().map(|n| ("marca.roles.foco", n)));
        for (donde, n) in nombres {
            if !m.tokens.contains_key(n) {
                return Err(format!(
                    "{donde} names «{n}», which is not in [marca.tokens]"
                ));
            }
        }
        if r.fondo.is_empty() || r.acento.is_empty() {
            return Err("marca.roles.fondo and marca.roles.acento need a token".into());
        }
        if r.acento.iter().any(|a| r.fondo.contains(a)) {
            return Err("marca.roles: a token cannot be background and accent".into());
        }
        if let Some(g) = &m.rejilla {
            if !r.fondo.contains(&g.token) {
                return Err(format!(
                    "marca.rejilla.token «{}» must be one of marca.roles.fondo (the gate counts the grid as empty)",
                    g.token
                ));
            }
            if g.paso.is_nan() || g.paso < 8.0 || g.grosor.is_nan() || g.grosor <= 0.0 {
                return Err("marca.rejilla: paso ≥ 8 and grosor > 0".into());
            }
        }
        let s = &m.subtitulo;
        if !(s.tam > 0.0 && s.ancho > 0.0 && s.franja[0] < s.franja[1] && s.palabras > 0) {
            return Err("marca.subtitulo: tam, ancho, palabras > 0 and franja = [y0 < y1]".into());
        }
        if m.titulo.tam.is_nan() || m.titulo.tam <= 0.0 {
            return Err("marca.titulo.tam must be > 0".into());
        }
        let rv = &self.revisor;
        if !slug_valido(&rv.nombre) || rv.nombre == "designer" {
            return Err(format!(
                "revisor.nombre «{}»: a slug (it names .claude/agents/<nombre>.md), not «designer»",
                rv.nombre
            ));
        }
        if rv.materia.trim().is_empty() {
            return Err("revisor.materia is empty".into());
        }
        if let Some(e) = &self.estudio {
            if let Some(g) = e.gadgets.iter().find(|g| !slug_valido(g)) {
                return Err(format!("estudio.gadgets «{g}»: a gadget tipo is a slug"));
            }
        }
        Ok(())
    }

    /// A token's colour (validated names only; an unknown name is a bug).
    pub fn color(&self, token: &str) -> [u8; 3] {
        self.marca.tokens[token]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugs_and_dates() {
        assert!(slug_valido("extrano-arte"));
        assert!(slug_valido("a1"));
        assert!(!slug_valido("1a"));
        assert!(!slug_valido("a--b"));
        assert!(!slug_valido("Arte"));
        assert!(!slug_valido("arte-"));
        assert!(fecha_valida("2026-10-18"));
        assert!(!fecha_valida("2026-13-01"));
        assert!(!fecha_valida("18-10-2026"));
    }
}
