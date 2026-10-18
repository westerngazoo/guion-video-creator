//! What the dashboard does to a post (R-0013 AC5–AC7): each action is an
//! existing command run as a subprocess (`guion reel …`), the system's opener,
//! or a one-line rewrite of the README. No agent is involved.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::fecha;
use crate::post::{self, Post};
use crate::tablero::Cuenta;

/// An action that runs as a subprocess, over one post or a batch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Accion {
    /// `guion reel <post>`: the whole pipeline (R-0012 AC10).
    Render,
    /// `guion reel check <post> --brand <perfil>`: the gate (R-0012).
    Check,
    /// `guion reel voice <post>`: `voz.toml` → `voz.wav` + `voz.json`.
    Voz,
}

impl Accion {
    pub fn nombre(self) -> &'static str {
        match self {
            Accion::Render => "render",
            Accion::Check => "check",
            Accion::Voz => "voz",
        }
    }

    /// `render`, `check`, `voz` (or `voice`), or their keys `r`, `c`, `v`.
    pub fn parse(s: &str) -> Option<Accion> {
        match s {
            "render" | "r" => Some(Accion::Render),
            "check" | "c" => Some(Accion::Check),
            "voz" | "voice" | "v" => Some(Accion::Voz),
            _ => None,
        }
    }
}

/// A command line, built before it runs (so it can be shown and tested).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Comando {
    pub programa: PathBuf,
    pub args: Vec<String>,
    /// Where it runs: the repo's root.
    pub dir: PathBuf,
}

impl Comando {
    /// `guion reel check content/posts/x --brand …`, for the log.
    pub fn mostrar(&self) -> String {
        let mut s = self.programa.display().to_string();
        for a in &self.args {
            s.push(' ');
            if a.contains(' ') {
                s.push_str(&format!("'{a}'"));
            } else {
                s.push_str(a);
            }
        }
        s
    }

    pub fn command(&self) -> Command {
        let mut c = Command::new(&self.programa);
        c.args(&self.args).current_dir(&self.dir);
        c
    }
}

/// The `guion` to call: `GUION_BIN`, else this very binary when it is
/// `guion`, else `guion` from `PATH`.
pub fn guion_bin() -> PathBuf {
    if let Some(p) = std::env::var_os("GUION_BIN").filter(|p| !p.is_empty()) {
        return PathBuf::from(p);
    }
    match std::env::current_exe() {
        Ok(p) if p.file_stem().is_some_and(|s| s == "guion") => p,
        _ => PathBuf::from("guion"),
    }
}

/// The command an action runs on a post. Render and voice need `reel.toml`
/// (a «legado» post is refused, naming why); check needs the account's
/// profile or the post's `reel.toml`.
pub fn comando(accion: Accion, p: &Post, c: &Cuenta, guion: &Path) -> Result<Comando, String> {
    let dir = p.dir.display().to_string();
    let args: Vec<String> = match accion {
        Accion::Render | Accion::Voz if !p.reel => {
            return Err(format!("{}: legado (sin reel.toml)", p.nombre));
        }
        Accion::Render => vec!["reel".into(), dir],
        Accion::Voz => vec!["reel".into(), "voice".into(), dir],
        Accion::Check => {
            let mut a = vec!["reel".into(), "check".into(), dir];
            match c.perfil_abs() {
                Some(perfil) => {
                    a.push("--brand".into());
                    a.push(perfil.display().to_string());
                }
                None if p.reel => {}
                None => {
                    return Err(format!(
                        "{}: sin perfil en la cuenta y sin reel.toml",
                        p.nombre
                    ))
                }
            }
            a
        }
    };
    Ok(Comando {
        programa: guion.to_path_buf(),
        args,
        dir: c.raiz.clone(),
    })
}

/// Rewrites the README's `estado:` line as `publicado (fecha, enlace)`,
/// keeping what it said before (`Antes: …`). The date is `YYYY-MM-DD`; the
/// link may be empty. A README without the line is an error.
pub fn marcar_publicado(readme: &str, fecha_s: &str, enlace: &str) -> Result<String, String> {
    if !fecha::es_fecha(fecha_s) {
        return Err(format!("«{fecha_s}» no es una fecha AAAA-MM-DD"));
    }
    let l = post::leer_estado(readme).ok_or("el README no tiene línea de estado")?;
    let enlace = enlace.trim();
    let mut nuevo = format!("{}publicado ({fecha_s}", l.prefijo);
    if !enlace.is_empty() {
        nuevo.push_str(&format!(", {enlace}"));
    }
    nuevo.push(')');
    let antes = l.valor.trim().trim_end_matches('.');
    if !antes.is_empty() {
        nuevo.push_str(&format!(". Antes: {antes}"));
    }
    let mut out: Vec<String> = readme.lines().map(str::to_string).collect();
    out[l.linea] = nuevo;
    let mut s = out.join("\n");
    if readme.ends_with('\n') {
        s.push('\n');
    }
    Ok(s)
}

/// Marks a post `publicado` on disk.
pub fn publicar(p: &Post, fecha_s: &str, enlace: &str) -> Result<(), String> {
    let f = p.dir.join("README.md");
    let t = std::fs::read_to_string(&f).map_err(|e| format!("{}: {e}", f.display()))?;
    let n = marcar_publicado(&t, fecha_s, enlace).map_err(|e| format!("{}: {e}", p.nombre))?;
    std::fs::write(&f, n).map_err(|e| format!("{}: {e}", f.display()))
}

/// What `o` does: open a file with the system's opener, or (headless) say where it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Abrir {
    Con(Comando),
    Ruta(PathBuf),
}

/// The system's opener: `open` on macOS; `xdg-open` with a display; none headless.
pub fn abridor(os: &str, hay_pantalla: bool) -> Option<&'static str> {
    match os {
        "macos" => Some("open"),
        "windows" => None,
        _ if hay_pantalla => Some("xdg-open"),
        _ => None,
    }
}

/// What to open for a post: its contact sheet, else its MP4.
pub fn abrir(p: &Post, c: &Cuenta, os: &str, hay_pantalla: bool) -> Result<Abrir, String> {
    let rel = p
        .hoja
        .as_ref()
        .or(p.video.as_ref())
        .ok_or_else(|| format!("{}: sin hoja ni MP4", p.nombre))?;
    let f = c.raiz.join(rel);
    Ok(match abridor(os, hay_pantalla) {
        Some(prog) => Abrir::Con(Comando {
            programa: prog.into(),
            args: vec![f.display().to_string()],
            dir: c.raiz.clone(),
        }),
        None => Abrir::Ruta(f),
    })
}

/// Whether this process has a display to open things on.
pub fn hay_pantalla() -> bool {
    ["DISPLAY", "WAYLAND_DISPLAY"]
        .iter()
        .any(|v| std::env::var_os(v).is_some_and(|x| !x.is_empty()))
}
