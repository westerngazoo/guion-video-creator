//! `guion-cuentas.toml`: the accounts a repo hosts (R-0013 AC1).
//!
//! Every brand value (an account's name, where its posts live, its gate
//! profile, its deliverables, its template) is the repo's, in this file at its
//! root. The crate holds none.
//!
//! ```toml
//! [[cuenta]]
//! nombre = "Cuenta A"
//! posts = ["content/posts/*"]          # globs of post folders (`*`, `?` per segment)
//! perfil = "brand/reel-perfil.toml"    # optional: the gate profile (R-0012 AC1)
//! publicar = ["publicar/nivel-*"]      # optional: folders holding <post>/ deliverables
//! plantilla = "plantillas/reel"        # optional: the template `guion reel nuevo` copies
//! nuevos = "content/posts"             # optional: where new posts go (default: from `posts`)
//!
//! [[cuenta.registro]]                  # optional: text appended on `guion reel nuevo`
//! archivo = "motor/Cargo.toml"
//! texto = "[[bin]]\nname = \"{{slug}}\"\n"
//! ```

use std::path::{Path, PathBuf};

use serde::Deserialize;

/// The file's name, at a repo's root.
pub const ARCHIVO: &str = "guion-cuentas.toml";

/// The whole file.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CuentasToml {
    #[serde(rename = "cuenta")]
    pub cuentas: Vec<CuentaToml>,
}

/// One account, as the repo declares it.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CuentaToml {
    /// The account's name, as the dashboard shows it and `--cuenta` matches it.
    pub nombre: String,
    /// Globs, relative to the repo, of the folders that may be posts (a post
    /// is one holding a `README.md`).
    pub posts: Vec<String>,
    /// The gate profile `guion reel check --brand` takes.
    #[serde(default)]
    pub perfil: Option<String>,
    /// Folders (globs) whose `<post>/` child holds the post's deliverables.
    #[serde(default)]
    pub publicar: Vec<String>,
    /// The template folder `guion reel nuevo` copies (R-0013 AC10).
    #[serde(default)]
    pub plantilla: Option<String>,
    /// Where `guion reel nuevo` creates posts; default: the first `posts`
    /// glob ending in `/*`, without it.
    #[serde(default)]
    pub nuevos: Option<String>,
    /// Text appended to a repo file when a post is created (a scene binary
    /// registered in its manifest, say), with the template's placeholders.
    #[serde(default)]
    pub registro: Vec<Registro>,
}

/// A snippet `guion reel nuevo` appends to one of the repo's files.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Registro {
    pub archivo: String,
    pub texto: String,
}

impl CuentaToml {
    /// The folder new posts go in, relative to the repo.
    pub fn carpeta_nuevos(&self) -> Option<String> {
        if let Some(n) = &self.nuevos {
            return Some(n.trim_end_matches('/').to_string());
        }
        self.posts.iter().find_map(|p| {
            let base = p.strip_suffix("/*")?;
            (!tiene_comodin(base)).then(|| base.to_string())
        })
    }
}

/// Parses and validates the file's text: at least one account, names unique
/// and not empty, at least one post glob each, every path relative.
pub fn parse(text: &str) -> Result<CuentasToml, String> {
    let c: CuentasToml = toml::from_str(text).map_err(|e| e.to_string())?;
    if c.cuentas.is_empty() {
        return Err("no account ([[cuenta]])".into());
    }
    let mut vistos: Vec<String> = Vec::new();
    for cuenta in &c.cuentas {
        let n = cuenta.nombre.trim();
        if n.is_empty() {
            return Err("an account with an empty nombre".into());
        }
        if vistos.iter().any(|v| v.to_lowercase() == n.to_lowercase()) {
            return Err(format!("account «{n}» declared twice"));
        }
        vistos.push(n.to_string());
        if cuenta.posts.is_empty() {
            return Err(format!("account «{n}»: posts is empty"));
        }
        let rutas = cuenta
            .posts
            .iter()
            .chain(&cuenta.publicar)
            .chain(&cuenta.perfil)
            .chain(&cuenta.plantilla)
            .chain(&cuenta.nuevos)
            .chain(cuenta.registro.iter().map(|r| &r.archivo));
        for r in rutas {
            if Path::new(r).is_absolute() {
                return Err(format!("account «{n}»: «{r}» must be relative to the repo"));
            }
        }
    }
    Ok(c)
}

/// Reads `<raiz>/guion-cuentas.toml`.
pub fn load(raiz: &Path) -> Result<CuentasToml, String> {
    let p = raiz.join(ARCHIVO);
    let text = std::fs::read_to_string(&p).map_err(|e| format!("{}: {e}", p.display()))?;
    parse(&text).map_err(|e| format!("{}: {e}", p.display()))
}

fn tiene_comodin(s: &str) -> bool {
    s.contains(['*', '?'])
}

/// The directories under `raiz` matching `patron` (`*` and `?` inside a
/// segment), sorted by path. No new crate: segments are matched one by one.
pub fn glob(raiz: &Path, patron: &str) -> Vec<PathBuf> {
    let mut actuales = vec![raiz.to_path_buf()];
    for seg in patron.split('/').filter(|s| !s.is_empty() && *s != ".") {
        let mut siguientes = Vec::new();
        for dir in &actuales {
            if tiene_comodin(seg) {
                let Ok(rd) = std::fs::read_dir(dir) else {
                    continue;
                };
                for e in rd.flatten() {
                    let nombre = e.file_name().to_string_lossy().into_owned();
                    if !nombre.starts_with('.') && comodin(seg, &nombre) && e.path().is_dir() {
                        siguientes.push(e.path());
                    }
                }
            } else {
                let p = dir.join(seg);
                if p.is_dir() {
                    siguientes.push(p);
                }
            }
        }
        actuales = siguientes;
    }
    actuales.sort();
    actuales.dedup();
    actuales
}

/// `*` (any run) and `?` (one char) against a whole name.
pub fn comodin(patron: &str, nombre: &str) -> bool {
    let p: Vec<char> = patron.chars().collect();
    let n: Vec<char> = nombre.chars().collect();
    let (mut i, mut j) = (0, 0);
    let (mut estrella, mut desde) = (None, 0);
    while j < n.len() {
        if i < p.len() && (p[i] == '?' || p[i] == n[j]) {
            i += 1;
            j += 1;
        } else if i < p.len() && p[i] == '*' {
            estrella = Some(i);
            desde = j;
            i += 1;
        } else if let Some(s) = estrella {
            i = s + 1;
            desde += 1;
            j = desde;
        } else {
            return false;
        }
    }
    while i < p.len() && p[i] == '*' {
        i += 1;
    }
    i == p.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wildcards_match_whole_names() {
        assert!(comodin("nivel-*", "nivel-0"));
        assert!(comodin("*", "x"));
        assert!(comodin("a?c", "abc"));
        assert!(!comodin("nivel-*", "pmmo"));
        assert!(comodin("*-audio.mp4", "x-y-audio.mp4"));
        assert!(!comodin("a*b", "acbx"));
    }

    #[test]
    fn new_posts_go_under_the_first_plain_glob() {
        let c = parse("[[cuenta]]\nnombre = \"A\"\nposts = [\"x/*/y\", \"content/posts/*\"]\n")
            .unwrap();
        assert_eq!(
            c.cuentas[0].carpeta_nuevos().as_deref(),
            Some("content/posts")
        );
    }
}
