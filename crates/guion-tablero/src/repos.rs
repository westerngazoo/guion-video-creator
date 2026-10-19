//! The user-level list of repos the dashboard reads when it is not told
//! which (R-0014 AC8): Rotor and PMMO live in one repo, @fisicobuenfisico in
//! another, a new account in a third; one list shows them side by side.
//!
//! The file is `$GUION_REPOS`, else `$XDG_CONFIG_HOME/guion/repos.toml`, else
//! `~/.config/guion/repos.toml`. It only grows by appending (`guion cuenta
//! nueva --al-tablero` adds the new repo), so it never rewrites what is there:
//!
//! ```toml
//! [[repo]]
//! ruta = "~/projects/rotorf-sico"      # `~/` is $HOME; relative: to this file
//! [[repo]]
//! ruta = "../fisicobuenfisico"
//! ```

use std::path::{Path, PathBuf};

use serde::Deserialize;

/// The variable that names the list's file.
pub const ENV: &str = "GUION_REPOS";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Lista {
    #[serde(default)]
    repo: Vec<Entrada>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Entrada {
    ruta: String,
}

/// Where the list lives (it may not exist).
pub fn archivo() -> Option<PathBuf> {
    if let Some(f) = std::env::var_os(ENV).filter(|v| !v.is_empty()) {
        return Some(PathBuf::from(f));
    }
    if let Some(x) = std::env::var_os("XDG_CONFIG_HOME").filter(|v| !v.is_empty()) {
        return Some(PathBuf::from(x).join("guion/repos.toml"));
    }
    std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config/guion/repos.toml"))
}

fn home() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

/// The repos of a list's text: `~/` is `home`, a relative path is relative
/// to `base` (the file's folder). Unknown keys fail by name.
pub fn parse(text: &str, base: &Path, home: Option<&Path>) -> Result<Vec<PathBuf>, String> {
    let l: Lista = toml::from_str(text).map_err(|e| e.to_string())?;
    l.repo
        .into_iter()
        .map(|e| {
            if let Some(r) = e.ruta.strip_prefix("~/") {
                let h = home.ok_or(format!("«{}»: HOME is not set", e.ruta))?;
                Ok(h.join(r))
            } else if Path::new(&e.ruta).is_absolute() {
                Ok(PathBuf::from(&e.ruta))
            } else {
                Ok(base.join(&e.ruta))
            }
        })
        .collect()
}

/// Reads the list at `f`.
pub fn leer(f: &Path) -> Result<Vec<PathBuf>, String> {
    let text = std::fs::read_to_string(f).map_err(|e| format!("{}: {e}", f.display()))?;
    let base = f.parent().unwrap_or(Path::new("."));
    parse(&text, base, home().as_deref()).map_err(|e| format!("{}: {e}", f.display()))
}

/// The same folder, by its canonical path when it exists.
pub fn mismo(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(x), Ok(y)) => x == y,
        _ => a == b,
    }
}

/// Appends `repo` (absolute) to the list at `f`, creating it; `Ok(false)`
/// when it was already there.
pub fn agregar(f: &Path, repo: &Path) -> Result<bool, String> {
    let repo = repo
        .canonicalize()
        .map_err(|e| format!("{}: {e}", repo.display()))?;
    let actual = if f.exists() {
        std::fs::read_to_string(f).map_err(|e| format!("{}: {e}", f.display()))?
    } else {
        String::new()
    };
    let base = f.parent().unwrap_or(Path::new("."));
    let hay =
        parse(&actual, base, home().as_deref()).map_err(|e| format!("{}: {e}", f.display()))?;
    if hay.iter().any(|r| mismo(r, &repo)) {
        return Ok(false);
    }
    let mut nuevo = if actual.is_empty() {
        "# Los repos que `guion tablero` lee cuando no se le dice cuáles\n\
# (guion-video-creator R-0014). Solo se agrega al final.\n"
            .to_string()
    } else {
        actual
    };
    if !nuevo.ends_with('\n') {
        nuevo.push('\n');
    }
    let ruta = repo
        .display()
        .to_string()
        .replace('\\', "\\\\")
        .replace('"', "\\\"");
    nuevo.push_str(&format!("\n[[repo]]\nruta = \"{ruta}\"\n"));
    if let Some(p) = f.parent() {
        std::fs::create_dir_all(p).map_err(|e| format!("{}: {e}", p.display()))?;
    }
    std::fs::write(f, nuevo).map_err(|e| format!("{}: {e}", f.display()))?;
    Ok(true)
}

/// The workspaces the dashboard reads: the ones named (`--workspace`,
/// `--repo`), plus the user's list with `--todas`; with none named and no
/// `--todas`, the current folder if it declares accounts, else the user's
/// list if there is one, else the current folder (whose missing file is then
/// the error). The same repo twice is read once, as first named.
pub fn elegir(
    nombrados: &[PathBuf],
    todas: bool,
    cwd: &Path,
    usuario: Option<Vec<PathBuf>>,
) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    let mut poner = |p: PathBuf| {
        if !out.iter().any(|q| mismo(q, &p)) {
            out.push(p);
        }
    };
    for p in nombrados {
        poner(p.clone());
    }
    let declara = cwd.join(crate::cuentas::ARCHIVO).is_file();
    match usuario {
        Some(u) if todas || (nombrados.is_empty() && !declara) => {
            u.into_iter().for_each(&mut poner)
        }
        _ => {}
    }
    if out.is_empty() {
        out.push(cwd.to_path_buf());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_list_resolves_home_and_relative_paths() {
        let r = parse(
            "[[repo]]\nruta = \"~/p/rotor\"\n[[repo]]\nruta = \"../fbf\"\n[[repo]]\nruta = \"/abs\"\n",
            Path::new("/cfg"),
            Some(Path::new("/home/u")),
        )
        .unwrap();
        assert_eq!(
            r,
            [
                PathBuf::from("/home/u/p/rotor"),
                PathBuf::from("/cfg/../fbf"),
                PathBuf::from("/abs")
            ]
        );
        assert!(parse("[[repo]]\npath = \"x\"\n", Path::new("/"), None)
            .unwrap_err()
            .contains("path"));
    }
}
