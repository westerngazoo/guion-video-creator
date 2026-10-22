//! `encargos.toml`: one-line ideas the owner hands to an account (R-0013 AC8,
//! AC11, AC12).
//!
//! The file lives at the repo's root, next to `guion-cuentas.toml`. The
//! dashboard appends to it; [`tomar`] and [`marcar`] rewrite only the
//! `estado` and `post` lines of one block, so comments and the agent's other
//! edits stay. The account's agent reads it and moves each encargo on.
//!
//! ```toml
//! [[encargo]]
//! id = "E-0001"
//! fecha = "2026-10-18"
//! cuenta = "Cuenta A"
//! idea = "¿Por qué…?"
//! estado = "pendiente"         # pendiente | en curso | hecho | descartado
//! formato = "reel"             # optional (AC11): reel | short | youtube
//! prioridad = "alta"           # optional: alta | normal | baja
//! para = "2026-10-25"          # optional: the day the owner wants it by
//! nivel = "1.3"                # optional: the curriculum's level, the series' stop…
//! variantes = ["A", "B"]       # optional: what the reel compares (2 to 4)
//! remake = "04-press"          # optional: the old post it remakes
//! notas = "…"                  # optional: sources, constraints
//! post = "slug"                # optional (AC12): the post made for it, its folder's name or path
//! ```
//!
//! What the dashboard shows is not the stored `estado` alone: an encargo
//! whose `post` is found follows that post's README ([`vivo`]).

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
    pub formato: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prioridad: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub para: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nivel: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub variantes: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remake: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notas: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub post: Option<String>,
}

/// The stored estados, in order.
pub const ESTADOS: [&str; 4] = ["pendiente", "en curso", "hecho", "descartado"];
/// The formats a reel can be asked in (`GUION_FORMATO`'s names, and `short`).
pub const FORMATOS: [&str; 3] = ["reel", "short", "youtube"];
/// The priorities.
pub const PRIORIDADES: [&str; 3] = ["alta", "normal", "baja"];

/// What the owner may add to an idea (AC11). Every field is optional.
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Detalles {
    pub formato: Option<String>,
    pub prioridad: Option<String>,
    pub para: Option<String>,
    pub nivel: Option<String>,
    pub variantes: Vec<String>,
    pub remake: Option<String>,
    pub notas: Option<String>,
    pub post: Option<String>,
}

fn una_linea(campo: &str, v: &str, max: usize) -> Result<String, String> {
    let v = v.trim();
    if v.is_empty() || v.contains('\n') {
        return Err(format!("{campo}: one line, not empty"));
    }
    if v.chars().count() > max {
        return Err(format!("{campo}: at most {max} characters"));
    }
    Ok(v.to_string())
}

fn uno_de(campo: &str, v: &str, validos: &[&str]) -> Result<String, String> {
    let v = v.trim().to_lowercase();
    if validos.contains(&v.as_str()) {
        Ok(v)
    } else {
        Err(format!("{campo} «{v}»: {}", validos.join(", ")))
    }
}

impl Detalles {
    /// Trimmed and checked: the closed fields name one of their values, the
    /// date is `YYYY-MM-DD`, the variants are 2 to 4 short lines, empty text
    /// is no field.
    pub fn limpios(&self) -> Result<Detalles, String> {
        fn txt(v: &Option<String>) -> Option<&str> {
            v.as_deref().map(str::trim).filter(|v| !v.is_empty())
        }
        let formato = txt(&self.formato)
            .map(|v| uno_de("formato", v, &FORMATOS))
            .transpose()?;
        let prioridad = txt(&self.prioridad)
            .map(|v| uno_de("prioridad", v, &PRIORIDADES))
            .transpose()?;
        let para = match txt(&self.para) {
            Some(v) if crate::fecha::es_fecha(v) => Some(v.to_string()),
            Some(v) => return Err(format!("para «{v}»: YYYY-MM-DD")),
            None => None,
        };
        let nivel = txt(&self.nivel)
            .map(|v| una_linea("nivel", v, 80))
            .transpose()?;
        let variantes: Vec<String> = self
            .variantes
            .iter()
            .map(|v| v.trim())
            .filter(|v| !v.is_empty())
            .map(|v| una_linea("variante", v, 60))
            .collect::<Result<_, _>>()?;
        if variantes.len() == 1 || variantes.len() > 4 {
            return Err("variantes: from 2 to 4 (what the reel compares)".into());
        }
        let remake = txt(&self.remake)
            .map(|v| una_linea("remake", v, 120))
            .transpose()?;
        let notas = match txt(&self.notas) {
            Some(v) if v.chars().count() > 1000 => {
                return Err("notas: at most 1000 characters".into())
            }
            v => v.map(str::to_string),
        };
        let post = txt(&self.post)
            .map(|v| una_linea("post", v, 200))
            .transpose()?;
        Ok(Detalles {
            formato,
            prioridad,
            para,
            nivel,
            variantes,
            remake,
            notas,
            post,
        })
    }
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
    let d = Detalles {
        post: post.map(str::to_string),
        ..Detalles::default()
    };
    agregar_con(raiz, cuenta, idea, fecha, &d)
}

fn q(s: &str) -> String {
    toml::Value::String(s.to_string()).to_string()
}

/// [`agregar`] with the owner's details (AC11), checked by
/// [`Detalles::limpios`].
pub fn agregar_con(
    raiz: &Path,
    cuenta: &str,
    idea: &str,
    fecha: &str,
    detalles: &Detalles,
) -> Result<Encargo, String> {
    let idea = idea.trim();
    if idea.is_empty() || idea.contains('\n') {
        return Err("the idea is one line, not empty".into());
    }
    let d = detalles.limpios()?;
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
        formato: d.formato,
        prioridad: d.prioridad,
        para: d.para,
        nivel: d.nivel,
        variantes: d.variantes,
        remake: d.remake,
        notas: d.notas,
        post: d.post,
    };
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
    for (k, v) in [
        ("formato", &e.formato),
        ("prioridad", &e.prioridad),
        ("para", &e.para),
        ("nivel", &e.nivel),
    ] {
        if let Some(v) = v {
            bloque.push_str(&format!("{k} = {}\n", q(v)));
        }
    }
    if !e.variantes.is_empty() {
        let vs: Vec<String> = e.variantes.iter().map(|v| q(v)).collect();
        bloque.push_str(&format!("variantes = [{}]\n", vs.join(", ")));
    }
    for (k, v) in [
        ("remake", &e.remake),
        ("notas", &e.notas),
        ("post", &e.post),
    ] {
        if let Some(v) = v {
            bloque.push_str(&format!("{k} = {}\n", q(v)));
        }
    }
    let nuevo = actual + &bloque;
    // what was written must read back: never leave a file the agent cannot parse
    let leido = parse(&nuevo).map_err(|err| format!("{}: {err}", p.display()))?;
    if leido.last() != Some(&e) {
        return Err(format!(
            "{}: the new encargo does not read back",
            p.display()
        ));
    }
    std::fs::write(&p, nuevo).map_err(|err| format!("{}: {err}", p.display()))?;
    Ok(e)
}

/// The block of encargo `id`: the lines from its `[[encargo]]` to the next
/// table (or the end), as a range of line indices.
fn bloque_de(lineas: &[&str], id: &str) -> Option<std::ops::Range<usize>> {
    let inicios: Vec<usize> = lineas
        .iter()
        .enumerate()
        .filter(|(_, l)| l.trim() == "[[encargo]]")
        .map(|(i, _)| i)
        .collect();
    // the string value of a `id = "…"` line
    let id_de = |l: &str| -> Option<String> {
        let (k, _) = l.split_once('=')?;
        if k.trim() != "id" {
            return None;
        }
        let t: toml::Table = toml::from_str(l.trim()).ok()?;
        t.get("id")?.as_str().map(str::to_string)
    };
    for (n, &a) in inicios.iter().enumerate() {
        let b = inicios.get(n + 1).copied().unwrap_or_else(|| {
            // a later [table] ends the block too
            lineas[a + 1..]
                .iter()
                .position(|l| l.trim_start().starts_with('['))
                .map_or(lineas.len(), |k| a + 1 + k)
        });
        let tiene = lineas[a + 1..b]
            .iter()
            .any(|l| id_de(l).as_deref() == Some(id));
        if tiene {
            return Some(a..b);
        }
    }
    None
}

/// Sets `campo = valor` inside the block: rewrites its line if there is one,
/// else adds it after `despues` (or at the block's last key).
fn poner(
    lineas: &mut Vec<String>,
    r: &mut std::ops::Range<usize>,
    campo: &str,
    valor: &str,
    despues: &str,
) {
    let es = |l: &str, k: &str| l.split_once('=').is_some_and(|(a, _)| a.trim() == k);
    let nueva = format!("{campo} = {}", q(valor));
    if let Some(i) = (r.start..r.end).find(|&i| es(&lineas[i], campo)) {
        lineas[i] = nueva;
        return;
    }
    let ultima = (r.start..r.end)
        .rev()
        .find(|&i| !lineas[i].trim().is_empty() && !lineas[i].trim_start().starts_with('#'))
        .unwrap_or(r.start);
    let i = (r.start..r.end)
        .find(|&i| es(&lineas[i], despues))
        .unwrap_or(ultima)
        + 1;
    lineas.insert(i, nueva);
    r.end += 1;
}

/// Rewrites encargo `id`'s `estado` (one of [`ESTADOS`]) and, when given,
/// its `post`, touching no other line (AC12). Returns the encargo as it now
/// reads.
pub fn marcar(raiz: &Path, id: &str, estado: &str, post: Option<&str>) -> Result<Encargo, String> {
    let estado = uno_de("estado", estado, &ESTADOS)?;
    let post = post.map(|v| una_linea("post", v, 200)).transpose()?;
    let p = raiz.join(ARCHIVO);
    let texto = std::fs::read_to_string(&p).map_err(|e| format!("{}: {e}", p.display()))?;
    parse(&texto).map_err(|e| format!("{}: {e}", p.display()))?;
    let crudas: Vec<&str> = texto.lines().collect();
    let mut r =
        bloque_de(&crudas, id).ok_or_else(|| format!("{}: no encargo {id}", p.display()))?;
    let mut lineas: Vec<String> = crudas.iter().map(|l| l.to_string()).collect();
    poner(&mut lineas, &mut r, "estado", &estado, "idea");
    if let Some(post) = &post {
        poner(&mut lineas, &mut r, "post", post, "estado");
    }
    let mut nuevo = lineas.join("\n");
    if texto.ends_with('\n') {
        nuevo.push('\n');
    }
    let leidos = parse(&nuevo).map_err(|e| format!("{}: {e}", p.display()))?;
    let e = leidos
        .into_iter()
        .find(|e| e.id == id)
        .ok_or_else(|| format!("{}: no encargo {id}", p.display()))?;
    if e.estado != estado || (post.is_some() && e.post != post) {
        return Err(format!("{}: {id} does not read back", p.display()));
    }
    std::fs::write(&p, nuevo).map_err(|err| format!("{}: {err}", p.display()))?;
    Ok(e)
}

/// The agent takes encargo `id` for `post` (its folder's name or path):
/// `en curso`, with the post linked, so the dashboard follows it from here.
pub fn tomar(raiz: &Path, id: &str, post: &str) -> Result<Encargo, String> {
    marcar(raiz, id, "en curso", Some(post))
}

/// What the dashboard shows for an encargo (AC12): `descartado` stays; with
/// a linked post that the account has, the post's estado decides (`listo`
/// and `publicado` as they are, anything before them `en curso`); else the
/// stored estado.
pub fn vivo(e: &Encargo, estado_del_post: Option<Option<crate::post::Estado>>) -> String {
    use crate::post::Estado;
    if e.estado == "descartado" {
        return e.estado.clone();
    }
    match estado_del_post {
        Some(Some(Estado::Publicado)) => "publicado".into(),
        Some(Some(Estado::Listo)) => "listo".into(),
        Some(_) => "en curso".into(),
        None => e.estado.clone(),
    }
}
