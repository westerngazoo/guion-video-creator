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

/// An encargo, the repo it lives in, and what it is now (AC12).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EncargoEn {
    pub repo: String,
    #[serde(flatten)]
    pub encargo: Encargo,
    /// What the dashboard shows: the linked post's estado, else the stored
    /// one ([`encargos::vivo`]).
    pub vivo: String,
    /// The linked post's folder, relative to the repo, when the account has it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_ruta: Option<String>,
    /// The linked post's deliverables' folder, relative to the repo.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paquete: Option<String>,
}

/// The post of `c` that `nombre` names: its folder's name or its path.
pub fn post_de<'a>(c: &'a Cuenta, nombre: &str) -> Option<&'a Post> {
    let n = nombre.trim().trim_end_matches('/');
    c.posts.iter().find(|p| p.nombre == n || p.ruta == n)
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
                let vivo = e.estado.clone();
                todos_encargos.push(EncargoEn {
                    repo: raiz.display().to_string(),
                    encargo: e,
                    vivo,
                    post_ruta: None,
                    paquete: None,
                });
            }
        }
        for e in &mut todos_encargos {
            let post = e.encargo.post.as_deref().and_then(|n| {
                cuentas
                    .iter()
                    .filter(|c| c.repo == e.repo && mismo_nombre(&c.nombre, &e.encargo.cuenta))
                    .find_map(|c| post_de(c, n))
            });
            e.vivo = encargos::vivo(&e.encargo, post.map(|p| p.estado));
            e.post_ruta = post.map(|p| p.ruta.clone());
            e.paquete = post.and_then(|p| p.publicar.clone());
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

    /// The encargos of an account with what they are now, newest last.
    pub fn pedidos_de<'a>(&'a self, c: &'a Cuenta) -> impl Iterator<Item = &'a EncargoEn> + 'a {
        let repo = c.raiz.display().to_string();
        self.encargos
            .iter()
            .filter(move |e| e.repo == repo && mismo_nombre(&e.encargo.cuenta, &c.nombre))
    }

    /// Appends an encargo with the owner's details (AC11).
    pub fn encargar_con(
        &self,
        cuenta: &str,
        idea: &str,
        fecha: &str,
        detalles: &encargos::Detalles,
    ) -> Result<Encargo, String> {
        let c = self
            .cuenta(cuenta)
            .ok_or_else(|| format!("no account named «{cuenta}»"))?;
        if let Some(n) = detalles.post.as_deref().filter(|n| !n.trim().is_empty()) {
            post_de(c, n).ok_or_else(|| format!("{} has no post «{n}»", c.nombre))?;
        }
        encargos::agregar_con(&c.raiz, &c.nombre, idea, fecha, detalles)
    }

    /// Moves encargo `id` to `estado` and, when given, links `post`, which
    /// must be a post of the encargo's account (AC12). An id two repos share
    /// needs `cuenta`.
    pub fn marcar(
        &self,
        id: &str,
        estado: &str,
        post: Option<&str>,
        cuenta: Option<&str>,
    ) -> Result<Encargo, String> {
        let hay: Vec<&EncargoEn> = self
            .encargos
            .iter()
            .filter(|e| e.encargo.id == id)
            .filter(|e| cuenta.is_none_or(|n| mismo_nombre(&e.encargo.cuenta, n)))
            .collect();
        let e = match hay.as_slice() {
            [e] => *e,
            [] => return Err(format!("no encargo {id}")),
            _ => return Err(format!("{id} is in {} repos: name its account", hay.len())),
        };
        if let Some(n) = post {
            let c = self
                .cuentas
                .iter()
                .find(|c| c.repo == e.repo && mismo_nombre(&c.nombre, &e.encargo.cuenta))
                .ok_or_else(|| format!("{id}: no account «{}» in its repo", e.encargo.cuenta))?;
            if post_de(c, n).is_none() {
                let hay: Vec<&str> = c.posts.iter().map(|p| p.nombre.as_str()).collect();
                return Err(format!(
                    "{} has no post «{n}» (hay: {})",
                    c.nombre,
                    hay.join(", ")
                ));
            }
        }
        encargos::marcar(Path::new(&e.repo), id, estado, post)
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
