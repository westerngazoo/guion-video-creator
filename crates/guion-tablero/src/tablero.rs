//! The whole state: every account every workspace declares, its posts and
//! counts, and the encargos (R-0013 AC3). `guion tablero --json` prints it;
//! the TUI draws it.

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::cuentas::{self, CuentaToml};
use crate::encargos::{self, Encargo};
use crate::post::{self, Estado, Post};

/// Two account names are the same regardless of case and surrounding spaces.
pub fn mismo_nombre(a: &str, b: &str) -> bool {
    a.trim().to_lowercase() == b.trim().to_lowercase()
}

/// How many posts of an account are in each estado.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Conteo {
    pub idea: usize,
    pub borrador: usize,
    #[serde(rename = "revisión")]
    pub revision: usize,
    pub listo: usize,
    pub publicado: usize,
    /// A README without an `estado:` line, or with a word that names none.
    #[serde(rename = "sin estado")]
    pub sin_estado: usize,
    /// Posts without `reel.toml`.
    pub legado: usize,
}

impl Conteo {
    pub fn de(&self, e: Option<Estado>) -> usize {
        match e {
            Some(Estado::Idea) => self.idea,
            Some(Estado::Borrador) => self.borrador,
            Some(Estado::Revision) => self.revision,
            Some(Estado::Listo) => self.listo,
            Some(Estado::Publicado) => self.publicado,
            None => self.sin_estado,
        }
    }

    fn sumar(&mut self, p: &Post) {
        match p.estado {
            Some(Estado::Idea) => self.idea += 1,
            Some(Estado::Borrador) => self.borrador += 1,
            Some(Estado::Revision) => self.revision += 1,
            Some(Estado::Listo) => self.listo += 1,
            Some(Estado::Publicado) => self.publicado += 1,
            None => self.sin_estado += 1,
        }
        if !p.reel {
            self.legado += 1;
        }
    }
}

/// One account and its posts.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Cuenta {
    pub nombre: String,
    /// The repo's root, as given.
    pub repo: String,
    /// The gate profile, relative to the repo.
    pub perfil: Option<String>,
    /// The template `guion reel nuevo` copies, relative to the repo.
    pub plantilla: Option<String>,
    pub conteo: Conteo,
    pub posts: Vec<Post>,
    /// The repo's root (not in the JSON).
    #[serde(skip)]
    pub raiz: PathBuf,
    /// What the repo declared (not in the JSON).
    #[serde(skip)]
    pub decl: CuentaToml,
}

impl Cuenta {
    /// The gate profile, absolute.
    pub fn perfil_abs(&self) -> Option<PathBuf> {
        self.perfil.as_ref().map(|p| self.raiz.join(p))
    }
}

/// An encargo and the repo it lives in.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EncargoEn {
    pub repo: String,
    #[serde(flatten)]
    pub encargo: Encargo,
}

/// Everything the dashboard shows.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Tablero {
    pub cuentas: Vec<Cuenta>,
    pub encargos: Vec<EncargoEn>,
}

/// Measures a video's duration; the real one asks ffmpeg, tests inject theirs.
pub type Medidor<'a> = &'a (dyn Fn(&Path) -> Option<f64> + Sync);

/// The real duration probe (guion-encode's banner reader).
pub fn duracion_ffmpeg(p: &Path) -> Option<f64> {
    guion_encode::ffmpeg::duration(p).ok()
}

impl Tablero {
    /// Reads every workspace (a repo root with `guion-cuentas.toml`). A
    /// missing or bad file is an error naming it; durations are measured in
    /// a few threads.
    pub fn leer(workspaces: &[PathBuf], medir: Medidor) -> Result<Tablero, String> {
        let mut cuentas = Vec::new();
        let mut todos_encargos = Vec::new();
        for raiz in workspaces {
            let decl = cuentas::load(raiz)?;
            for c in decl.cuentas {
                let publicar: Vec<PathBuf> = c
                    .publicar
                    .iter()
                    .flat_map(|g| cuentas::glob(raiz, g))
                    .collect();
                let mut posts = Vec::new();
                for g in &c.posts {
                    for dir in cuentas::glob(raiz, g) {
                        if dir.join("README.md").is_file()
                            && !posts.iter().any(|p: &Post| p.dir == dir)
                        {
                            posts.push(post::leer(raiz, &dir, &publicar)?);
                        }
                    }
                }
                let mut conteo = Conteo::default();
                posts.iter().for_each(|p| conteo.sumar(p));
                cuentas.push(Cuenta {
                    nombre: c.nombre.clone(),
                    repo: raiz.display().to_string(),
                    perfil: c.perfil.clone(),
                    plantilla: c.plantilla.clone(),
                    conteo,
                    posts,
                    raiz: raiz.clone(),
                    decl: c,
                });
            }
            for e in encargos::load(raiz)? {
                todos_encargos.push(EncargoEn {
                    repo: raiz.display().to_string(),
                    encargo: e,
                });
            }
        }
        let mut t = Tablero {
            cuentas,
            encargos: todos_encargos,
        };
        t.medir(medir);
        Ok(t)
    }

    fn medir(&mut self, medir: Medidor) {
        let mut pendientes: Vec<(&mut Post, PathBuf)> = Vec::new();
        for c in &mut self.cuentas {
            for p in &mut c.posts {
                if let Some(v) = &p.video {
                    let abs = c.raiz.join(v);
                    pendientes.push((p, abs));
                }
            }
        }
        let hilos = 8.min(pendientes.len().max(1));
        let por = pendientes.len().div_ceil(hilos).max(1);
        std::thread::scope(|s| {
            for trozo in pendientes.chunks_mut(por) {
                s.spawn(move || {
                    for (p, abs) in trozo.iter_mut() {
                        p.duracion = medir(abs);
                    }
                });
            }
        });
    }

    /// The JSON (`guion tablero --json`), pretty, keys in the structs' order.
    pub fn to_json(&self) -> String {
        let mut s = serde_json::to_string_pretty(self).expect("the state serializes");
        s.push('\n');
        s
    }

    /// The account named `nombre` (case-insensitive).
    pub fn cuenta(&self, nombre: &str) -> Option<&Cuenta> {
        self.cuentas
            .iter()
            .find(|c| mismo_nombre(&c.nombre, nombre))
    }

    /// The encargos of an account, newest last.
    pub fn encargos_de<'a>(&'a self, c: &'a Cuenta) -> impl Iterator<Item = &'a Encargo> + 'a {
        let repo = c.raiz.display().to_string();
        self.encargos
            .iter()
            .filter(move |e| e.repo == repo && mismo_nombre(&e.encargo.cuenta, &c.nombre))
            .map(|e| &e.encargo)
    }

    /// Appends an encargo for the account named `cuenta` in its repo.
    pub fn encargar(
        &self,
        cuenta: &str,
        idea: &str,
        fecha: &str,
        post: Option<&str>,
    ) -> Result<Encargo, String> {
        let c = self
            .cuenta(cuenta)
            .ok_or_else(|| format!("no account named «{cuenta}»"))?;
        encargos::agregar(&c.raiz, &c.nombre, idea, fecha, post)
    }

    /// Posts filtered by account (`None`: all) and estado (`None`: any), as
    /// (account index, post index).
    pub fn filtrar(&self, cuenta: Option<&str>, estado: Option<Estado>) -> Vec<(usize, usize)> {
        let mut out = Vec::new();
        for (i, c) in self.cuentas.iter().enumerate() {
            if cuenta.is_some_and(|n| !mismo_nombre(&c.nombre, n)) {
                continue;
            }
            for (j, p) in c.posts.iter().enumerate() {
                if estado.is_none() || p.estado == estado {
                    out.push((i, j));
                }
            }
        }
        out
    }
}
