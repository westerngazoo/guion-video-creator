//! One post as the dashboard reads it from its folder (R-0013 AC2): the
//! README's title and `estado:` line, `reel.toml` (the narrated pipeline;
//! without it, «legado»), `voz.toml`, the last gate result (`check.json`),
//! the reel's MP4 and its duration, the contact sheet and the deliverables.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// A post's lifecycle, as the README's `estado:` line names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
pub enum Estado {
    #[serde(rename = "idea")]
    Idea,
    #[serde(rename = "borrador")]
    Borrador,
    #[serde(rename = "revisión")]
    Revision,
    #[serde(rename = "listo")]
    Listo,
    #[serde(rename = "publicado")]
    Publicado,
}

impl Estado {
    /// Every estado, in lifecycle order.
    pub const TODOS: [Estado; 5] = [
        Estado::Idea,
        Estado::Borrador,
        Estado::Revision,
        Estado::Listo,
        Estado::Publicado,
    ];

    pub fn nombre(self) -> &'static str {
        match self {
            Estado::Idea => "idea",
            Estado::Borrador => "borrador",
            Estado::Revision => "revisión",
            Estado::Listo => "listo",
            Estado::Publicado => "publicado",
        }
    }

    /// One word, any case, with or without the accent.
    pub fn parse(palabra: &str) -> Option<Estado> {
        match palabra.to_lowercase().as_str() {
            "idea" => Some(Estado::Idea),
            "borrador" => Some(Estado::Borrador),
            "revisión" | "revision" => Some(Estado::Revision),
            "listo" => Some(Estado::Listo),
            "publicado" => Some(Estado::Publicado),
            _ => None,
        }
    }
}

/// The README's `estado:` line.
#[derive(Debug, Clone, PartialEq)]
pub struct LineaEstado {
    /// Its index among the README's lines.
    pub linea: usize,
    /// Everything up to the value (`- **estado:** `), kept on a rewrite.
    pub prefijo: String,
    /// The value: the estado word and its note.
    pub valor: String,
    /// The estado, if the first word names one.
    pub estado: Option<Estado>,
}

/// Finds the first `estado:` line: after an optional list marker and
/// emphasis (`- **estado:** listo (…)`, `estado: listo`, `* _estado_: idea`).
pub fn leer_estado(readme: &str) -> Option<LineaEstado> {
    for (i, linea) in readme.lines().enumerate() {
        let sin_marca = linea.trim_start_matches([' ', '\t', '-', '*', '_', '>']);
        if !sin_marca
            .get(..6)
            .is_some_and(|k| k.eq_ignore_ascii_case("estado"))
        {
            continue;
        }
        let tras_enf = sin_marca[6..].trim_start_matches(['*', '_']);
        let Some(valor_l) = tras_enf.strip_prefix(':') else {
            continue;
        };
        let valor_l = valor_l.trim_start_matches(['*', '_', ' ', '\t']);
        let inicio = linea.len() - valor_l.len();
        let valor = linea[inicio..].trim_end().to_string();
        let palabra: String = valor
            .chars()
            .take_while(|c| c.is_alphabetic())
            .collect::<String>();
        return Some(LineaEstado {
            linea: i,
            prefijo: linea[..inicio].to_string(),
            estado: Estado::parse(&palabra),
            valor,
        });
    }
    None
}

/// The README's title: its first `# ` heading.
pub fn leer_titulo(readme: &str) -> Option<String> {
    readme
        .lines()
        .find_map(|l| l.strip_prefix("# "))
        .map(|t| t.trim().to_string())
}

/// The last gate result, from `check.json` (R-0012 AC8), read leniently.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Check {
    pub pass: bool,
    pub total: usize,
    pub fallas: usize,
    /// One line per check: `PASS sub-width  22 lines, …`, failures with where.
    pub lineas: Vec<String>,
}

#[derive(Deserialize)]
struct CheckJson {
    pass: bool,
    #[serde(default)]
    findings: Vec<FindingJson>,
}

#[derive(Deserialize)]
struct FindingJson {
    check: String,
    pass: bool,
    #[serde(default)]
    detail: String,
    #[serde(default)]
    at: Vec<String>,
}

/// Parses a `check.json`.
pub fn leer_check(json: &str) -> Result<Check, String> {
    let c: CheckJson = serde_json::from_str(json).map_err(|e| e.to_string())?;
    let lineas = c
        .findings
        .iter()
        .map(|f| {
            let mark = if f.pass { "PASS" } else { "FAIL" };
            let mut l = format!("{mark} {:<13} {}", f.check, f.detail);
            if !f.pass && !f.at.is_empty() {
                let at: Vec<&str> = f.at.iter().take(3).map(String::as_str).collect();
                l.push_str(&format!(" @ {}", at.join("; ")));
            }
            l
        })
        .collect();
    Ok(Check {
        pass: c.pass,
        total: c.findings.len(),
        fallas: c.findings.iter().filter(|f| !f.pass).count(),
        lineas,
    })
}

/// One post.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Post {
    /// The folder's name.
    pub nombre: String,
    /// The folder, relative to the repo.
    pub ruta: String,
    pub titulo: Option<String>,
    pub estado: Option<Estado>,
    /// The `estado:` line's value (the word and its note).
    pub nota: Option<String>,
    /// `reel.toml`: the post makes itself with `guion reel`. Without it, «legado».
    pub reel: bool,
    /// `voz.toml`: the reel is narrated.
    pub narrado: bool,
    pub check: Option<Check>,
    /// The reel's MP4 (`<name>-audio.mp4`, else `<name>.mp4`), relative to the repo.
    pub video: Option<String>,
    /// Its duration, s (ffmpeg's banner).
    pub duracion: Option<f64>,
    /// `cuadros/hoja.png`, relative to the repo.
    pub hoja: Option<String>,
    /// The deliverables' folder, relative to the repo.
    pub publicar: Option<String>,
    /// The folder, absolute (not in the JSON).
    #[serde(skip)]
    pub dir: PathBuf,
}

impl Post {
    /// «legado» when there is no `reel.toml`.
    pub fn tipo(&self) -> &'static str {
        if self.reel {
            "reel"
        } else {
            "legado"
        }
    }
}

fn rel(raiz: &Path, p: &Path) -> String {
    p.strip_prefix(raiz)
        .unwrap_or(p)
        .to_string_lossy()
        .into_owned()
}

/// `reel.toml`'s `[reel] name` and `[publish] dir`, read leniently (the
/// pipeline validates the file; the dashboard only shows it).
fn reel_toml(dir: &Path) -> (Option<String>, Option<String>) {
    let Ok(text) = std::fs::read_to_string(dir.join("reel.toml")) else {
        return (None, None);
    };
    let Ok(v) = text.parse::<toml::Table>() else {
        return (None, None);
    };
    let s = |t: &str, k: &str| {
        v.get(t)
            .and_then(|x| x.get(k))
            .and_then(|x| x.as_str())
            .map(str::to_string)
    };
    (s("reel", "name"), s("publish", "dir"))
}

/// Reads the post in `dir` (which holds `README.md`). `publicar` are the
/// account's deliverable folders (already globbed); the duration is left to
/// the caller (it costs one ffmpeg call).
pub fn leer(raiz: &Path, dir: &Path, publicar: &[PathBuf]) -> Result<Post, String> {
    let readme_p = dir.join("README.md");
    let readme =
        std::fs::read_to_string(&readme_p).map_err(|e| format!("{}: {e}", readme_p.display()))?;
    let nombre = dir
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let linea = leer_estado(&readme);
    let reel = dir.join("reel.toml").is_file();
    let (nombre_reel, publish_dir) = reel_toml(dir);
    let check = match std::fs::read_to_string(dir.join("check.json")) {
        Ok(j) => Some(leer_check(&j).map_err(|e| format!("{}/check.json: {e}", dir.display()))?),
        Err(_) => None,
    };
    let video = buscar_video(dir, nombre_reel.as_deref().unwrap_or(&nombre));
    let hoja = Some(dir.join("cuadros").join("hoja.png")).filter(|p| p.is_file());
    let publicar_dir = publish_dir
        .map(|d| normalizar(&dir.join(d)))
        .filter(|p| p.is_dir())
        .or_else(|| {
            publicar
                .iter()
                .map(|d| d.join(&nombre))
                .find(|p| p.is_dir())
        });
    Ok(Post {
        ruta: rel(raiz, dir),
        titulo: leer_titulo(&readme),
        estado: linea.as_ref().and_then(|l| l.estado),
        nota: linea.map(|l| l.valor),
        reel,
        narrado: dir.join("voz.toml").is_file(),
        check,
        video: video.map(|v| rel(raiz, &v)),
        duracion: None,
        hoja: hoja.map(|h| rel(raiz, &h)),
        publicar: publicar_dir.map(|p| rel(raiz, &p)),
        dir: dir.to_path_buf(),
        nombre,
    })
}

/// `a/b/../c` → `a/c`, without touching the disk.
fn normalizar(p: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in p.components() {
        match c {
            std::path::Component::ParentDir => {
                out.pop();
            }
            std::path::Component::CurDir => {}
            c => out.push(c),
        }
    }
    out
}

/// The reel's MP4: `<name>-audio.mp4`, else the only `*-audio.mp4`, else `<name>.mp4`.
fn buscar_video(dir: &Path, nombre: &str) -> Option<PathBuf> {
    let con_audio = dir.join(format!("{nombre}-audio.mp4"));
    if con_audio.is_file() {
        return Some(con_audio);
    }
    let mut otros: Vec<PathBuf> = std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .is_some_and(|n| n.to_string_lossy().ends_with("-audio.mp4"))
        })
        .collect();
    if otros.len() == 1 {
        return otros.pop();
    }
    Some(dir.join(format!("{nombre}.mp4"))).filter(|p| p.is_file())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_estado_word_ends_at_punctuation() {
        let l = leer_estado("# T\n\n- **estado:** revisión — falta fisico\n").unwrap();
        assert_eq!(l.estado, Some(Estado::Revision));
        assert_eq!(l.prefijo, "- **estado:** ");
        assert_eq!(l.valor, "revisión — falta fisico");
        assert_eq!(
            leer_estado("estado: Listo.").unwrap().estado,
            Some(Estado::Listo)
        );
        assert_eq!(
            leer_estado("- estado: revision(2026)").unwrap().estado,
            Some(Estado::Revision)
        );
        assert_eq!(
            leer_estado("- **estado:** quién sabe").unwrap().estado,
            None
        );
        assert!(leer_estado("el estado de las cosas: bien").is_none());
    }
}
