//! `encargos.toml`: one-line ideas the owner hands to an account (R-0013 AC8).
//!
//! The file lives at the repo's root, next to `guion-cuentas.toml`. The
//! dashboard appends to it (never rewrites it, so comments and the agent's
//! edits stay); the account's agent reads it and moves each encargo on.
//!
//! ```toml
//! [[encargo]]
//! id = "E-0001"
//! fecha = "2026-10-18"
//! cuenta = "Cuenta A"
//! idea = "¿Por qué…?"
//! estado = "pendiente"
//! post = "slug"          # optional: the post `guion reel nuevo` made for it
//! ```

use std::path::Path;

use serde::{Deserialize, Serialize};

/// The file's name, at a repo's root.
pub const ARCHIVO: &str = "encargos.toml";

/// One encargo.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Encargo {
    pub id: String,
    pub fecha: String,
    pub cuenta: String,
    pub idea: String,
    pub estado: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub post: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EncargosToml {
    #[serde(default, rename = "encargo")]
    encargos: Vec<Encargo>,
}

/// Parses the file's text.
pub fn parse(text: &str) -> Result<Vec<Encargo>, String> {
    toml::from_str::<EncargosToml>(text)
        .map(|e| e.encargos)
        .map_err(|e| e.to_string())
}

/// Reads `<raiz>/encargos.toml`; none yet is an empty list.
pub fn load(raiz: &Path) -> Result<Vec<Encargo>, String> {
    let p = raiz.join(ARCHIVO);
    match std::fs::read_to_string(&p) {
        Ok(t) => parse(&t).map_err(|e| format!("{}: {e}", p.display())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(format!("{}: {e}", p.display())),
    }
}

/// The next id after the file's highest `E-NNNN`.
pub fn siguiente_id(encargos: &[Encargo]) -> String {
    let n = encargos
        .iter()
        .filter_map(|e| e.id.strip_prefix("E-")?.parse::<u32>().ok())
        .max()
        .unwrap_or(0);
    format!("E-{:04}", n + 1)
}

const ENCABEZADO: &str =
    "# Encargos: ideas de una línea que el dueño deja a una cuenta (guion tablero, `e`).\n\
# El tablero solo agrega al final; el agente de la cuenta los lee y cambia su estado\n\
# (pendiente → en curso → hecho, o descartado).\n";

/// Appends a `pendiente` encargo to `<raiz>/encargos.toml` (created if
/// missing) and returns it. The idea is one line, not empty.
pub fn agregar(
    raiz: &Path,
    cuenta: &str,
    idea: &str,
    fecha: &str,
    post: Option<&str>,
) -> Result<Encargo, String> {
    let idea = idea.trim();
    if idea.is_empty() || idea.contains('\n') {
        return Err("the idea is one line, not empty".into());
    }
    let p = raiz.join(ARCHIVO);
    let actual = match std::fs::read_to_string(&p) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => ENCABEZADO.to_string(),
        Err(e) => return Err(format!("{}: {e}", p.display())),
    };
    let previos = parse(&actual).map_err(|e| format!("{}: {e}", p.display()))?;
    let e = Encargo {
        id: siguiente_id(&previos),
        fecha: fecha.to_string(),
        cuenta: cuenta.to_string(),
        idea: idea.to_string(),
        estado: "pendiente".into(),
        post: post.map(str::to_string),
    };
    let q = |s: &str| toml::Value::String(s.to_string()).to_string();
    let mut bloque = String::new();
    if !actual.ends_with('\n') && !actual.is_empty() {
        bloque.push('\n');
    }
    bloque.push_str(&format!(
        "\n[[encargo]]\nid = {}\nfecha = {}\ncuenta = {}\nidea = {}\nestado = {}\n",
        q(&e.id),
        q(&e.fecha),
        q(&e.cuenta),
        q(&e.idea),
        q(&e.estado)
    ));
    if let Some(post) = &e.post {
        bloque.push_str(&format!("post = {}\n", q(post)));
    }
    let nuevo = actual + &bloque;
    // what was written must read back: never leave a file the agent cannot parse
    parse(&nuevo).map_err(|err| format!("{}: {err}", p.display()))?;
    std::fs::write(&p, nuevo).map_err(|err| format!("{}: {err}", p.display()))?;
    Ok(e)
}
