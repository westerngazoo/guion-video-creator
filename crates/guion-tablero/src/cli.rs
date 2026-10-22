//! The command lines (R-0013 AC3, AC9, AC10):
//!
//! ```text
//! guion tablero [--repo DIR]... [--todas] [--paralelo N]
//! guion tablero --json [--repo DIR]... [--todas]
//! guion tablero lote <render|check|voz> [--cuenta X] [--estado Y] [--paralelo N] [--dry-run] [--repo DIR]...
//! guion reel nuevo <cuenta> <slug> [--titulo "…"] [--encargo E-NNNN] [--repo DIR]...
//! guion encargo nuevo <cuenta> "<idea>" [--formato F] [--prioridad P] [--para FECHA] [--nivel N]
//!                     [--variante A --variante B]... [--remake POST] [--notas "…"] [--post POST]
//! guion encargo tomar <E-NNNN> <post> [--cuenta X] [--repo DIR]...
//! guion encargo marcar <E-NNNN> <pendiente|en-curso|hecho|descartado> [--post POST] [--cuenta X]
//! guion encargo lista [--cuenta X] [--json] [--repo DIR]...
//! ```
//!
//! `--repo DIR` (repeatable) and `--workspace DIR...` name the repos;
//! `--todas` adds the user's list ([`crate::repos`], R-0014 AC8). With
//! neither, the current folder, or the user's list when the current folder
//! declares no accounts.
//!
//! Exit codes: 0 done, 1 some job failed, 2 usage or input error.

use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::mpsc;

use crate::acciones::{self, Accion};
use crate::post::Estado;
use crate::tablero::{duracion_ffmpeg, Tablero};
use crate::trabajos::{self, Evento, Trabajo};
use crate::{fecha, nuevo, tui};

/// `guion tablero`'s usage.
pub const USO: &str = "uso:\n  guion tablero [--repo DIR]... [--todas] [--paralelo N]\n  \
guion tablero --json [--repo DIR]... [--todas]\n  \
guion tablero lote <render|check|voz> [--cuenta X] [--estado Y] [--paralelo N] [--dry-run] [--repo DIR]... [--todas]\n\
(--workspace DIR... también vale; sin repos: la carpeta actual, o la lista del usuario en $GUION_REPOS o ~/.config/guion/repos.toml)";

/// `guion reel nuevo`'s usage.
pub const USO_NUEVO: &str =
    "uso:\n  guion reel nuevo <cuenta> <slug> [--titulo \"…\"] [--encargo E-NNNN] [--repo DIR]... [--todas]";

/// `guion encargo`'s usage (R-0013 AC11, AC12).
pub const USO_ENCARGO: &str = "uso:\n  \
guion encargo nuevo <cuenta> \"<idea>\" [--formato reel|short|youtube] [--prioridad alta|normal|baja] [--para AAAA-MM-DD]\n    \
[--nivel N] [--variante A --variante B]... [--remake POST] [--notas \"…\"] [--post POST]\n  \
guion encargo tomar <E-NNNN> <post> [--cuenta X]          (en curso, con su post: el tablero lo sigue desde ahí)\n  \
guion encargo marcar <E-NNNN> <pendiente|en-curso|hecho|descartado> [--post POST] [--cuenta X]\n  \
guion encargo lista [--cuenta X] [--json]\n\
(--repo DIR y --todas como en guion tablero)";

/// The parsed flags.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Opciones {
    pub workspaces: Vec<PathBuf>,
    /// `--todas`: the user's list of repos too (R-0014 AC8).
    pub todas: bool,
    pub json: bool,
    pub paralelo: usize,
    pub cuenta: Option<String>,
    pub estado: Option<Estado>,
    pub dry_run: bool,
    pub titulo: Option<String>,
    /// `--encargo E-NNNN`: the encargo `reel nuevo` takes for the new post.
    pub encargo: Option<String>,
    /// `encargo nuevo`'s details (AC11).
    pub detalles: crate::encargos::Detalles,
    /// Positional arguments (`lote <accion>`; `nuevo <cuenta> <slug>`).
    pub resto: Vec<String>,
}

/// Parses the flags; `--workspace` takes every following argument up to the
/// next flag (or `lote`), `--repo` takes one (and repeats). Which repos are
/// read when none is named: [`crate::repos::elegir`].
pub fn opciones(args: &[String]) -> Result<Opciones, String> {
    let mut o = Opciones {
        paralelo: 2,
        ..Opciones::default()
    };
    let mut i = 0;
    let valor = |i: usize, flag: &str| -> Result<String, String> {
        args.get(i + 1)
            .cloned()
            .ok_or(format!("{flag} necesita un valor"))
    };
    while i < args.len() {
        let a = args[i].as_str();
        match a {
            "--workspace" | "-w" => {
                let mut n = 0;
                while let Some(d) = args
                    .get(i + 1)
                    .filter(|d| !d.starts_with('-') && *d != "lote")
                {
                    o.workspaces.push(PathBuf::from(d));
                    i += 1;
                    n += 1;
                }
                if n == 0 {
                    return Err("--workspace necesita al menos una carpeta".into());
                }
            }
            "--repo" | "-r" => {
                o.workspaces.push(PathBuf::from(valor(i, a)?));
                i += 1;
            }
            "--todas" => o.todas = true,
            "--json" => o.json = true,
            "--dry-run" => o.dry_run = true,
            "--paralelo" | "-j" => {
                let v = valor(i, a)?;
                o.paralelo = v
                    .parse()
                    .ok()
                    .filter(|n| *n > 0)
                    .ok_or(format!("--paralelo {v}: un entero ≥ 1"))?;
                i += 1;
            }
            "--cuenta" => {
                o.cuenta = Some(valor(i, a)?);
                i += 1;
            }
            "--estado" => {
                let v = valor(i, a)?;
                o.estado = Some(Estado::parse(&v).ok_or(format!(
                    "--estado {v}: idea, borrador, revisión, listo o publicado"
                ))?);
                i += 1;
            }
            "--titulo" => {
                o.titulo = Some(valor(i, a)?);
                i += 1;
            }
            "--encargo" => {
                o.encargo = Some(valor(i, a)?);
                i += 1;
            }
            "--formato" | "--prioridad" | "--para" | "--nivel" | "--variante" | "--remake"
            | "--notas" | "--post" => {
                let v = valor(i, a)?;
                let d = &mut o.detalles;
                match a {
                    "--formato" => d.formato = Some(v),
                    "--prioridad" => d.prioridad = Some(v),
                    "--para" => d.para = Some(v),
                    "--nivel" => d.nivel = Some(v),
                    "--variante" => d.variantes.push(v),
                    "--remake" => d.remake = Some(v),
                    "--notas" => d.notas = Some(v),
                    _ => d.post = Some(v),
                }
                i += 1;
            }
            f if f.starts_with('-') => return Err(format!("bandera desconocida {f}")),
            p => o.resto.push(p.to_string()),
        }
        i += 1;
    }
    // the user's list is read only when it may be used
    let hace_falta = o.todas || o.workspaces.is_empty();
    let usuario = match crate::repos::archivo().filter(|f| hace_falta && f.is_file()) {
        Some(f) => Some(crate::repos::leer(&f)?),
        None if o.todas => {
            return Err(format!(
                "--todas: no user list of repos (${} or ~/.config/guion/repos.toml)",
                crate::repos::ENV
            ))
        }
        None => None,
    };
    let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
    o.workspaces = crate::repos::elegir(&o.workspaces, o.todas, &cwd, usuario);
    Ok(o)
}

fn fallo(msg: impl std::fmt::Display, uso: &str) -> ExitCode {
    eprintln!("guion tablero: {msg}\n{uso}");
    ExitCode::from(2)
}

/// `guion tablero …`.
pub fn run(args: &[String]) -> ExitCode {
    if args.iter().any(|a| a == "-h" || a == "--help") {
        eprintln!("{USO}");
        return ExitCode::SUCCESS;
    }
    let o = match opciones(args) {
        Ok(o) => o,
        Err(e) => return fallo(e, USO),
    };
    let tablero = match Tablero::leer(&o.workspaces, &duracion_ffmpeg) {
        Ok(t) => t,
        Err(e) => return fallo(e, USO),
    };
    match o.resto.first().map(String::as_str) {
        Some("lote") => lote(&o, &tablero),
        Some(x) => fallo(format!("argumento inesperado {x}"), USO),
        None if o.json => {
            print!("{}", tablero.to_json());
            ExitCode::SUCCESS
        }
        None => match tui::correr(o.workspaces.clone(), tablero, o.paralelo) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => fallo(e, USO),
        },
    }
}

/// The jobs `lote` would run over the filtered posts, and the posts it
/// skips (with why).
pub fn plan_lote(
    t: &Tablero,
    accion: Accion,
    cuenta: Option<&str>,
    estado: Option<Estado>,
    guion: &std::path::Path,
) -> Result<(Vec<Trabajo>, Vec<String>), String> {
    if let Some(c) = cuenta {
        if t.cuenta(c).is_none() {
            let hay: Vec<&str> = t.cuentas.iter().map(|c| c.nombre.as_str()).collect();
            return Err(format!("no hay cuenta «{c}» (hay: {})", hay.join(", ")));
        }
    }
    let (mut trabajos, mut omitidos) = (Vec::new(), Vec::new());
    for (i, j) in t.filtrar(cuenta, estado) {
        let c = &t.cuentas[i];
        let p = &c.posts[j];
        match acciones::comando(accion, p, c, guion) {
            Ok(comando) => trabajos.push(Trabajo {
                accion,
                post: p.nombre.clone(),
                dir: p.dir.clone(),
                comando,
            }),
            Err(e) => omitidos.push(e),
        }
    }
    Ok((trabajos, omitidos))
}

fn lote(o: &Opciones, t: &Tablero) -> ExitCode {
    let Some(accion) = o.resto.get(1).and_then(|a| Accion::parse(a)) else {
        return fallo("lote necesita una acción: render, check o voz", USO);
    };
    let (trabajos, omitidos) = match plan_lote(
        t,
        accion,
        o.cuenta.as_deref(),
        o.estado,
        &acciones::guion_bin(),
    ) {
        Ok(x) => x,
        Err(e) => return fallo(e, USO),
    };
    for s in &omitidos {
        println!("omitido: {s}");
    }
    if o.dry_run {
        for tr in &trabajos {
            println!("{}", tr.comando.mostrar());
        }
        return ExitCode::SUCCESS;
    }
    let n = trabajos.len();
    let (tx, rx) = mpsc::channel();
    trabajos::lanzar(trabajos, o.paralelo, tx);
    let (mut fin, mut fallas) = (0, 0);
    while fin < n {
        match rx.recv() {
            Ok(Evento::Empieza { post, comando, .. }) => println!("[{post}] $ {comando}"),
            Ok(Evento::Linea { post, texto }) => println!("[{post}] {texto}"),
            Ok(Evento::Fin {
                post, ok, codigo, ..
            }) => {
                fin += 1;
                if !ok {
                    fallas += 1;
                }
                println!(
                    "[{post}] {} {}",
                    accion.nombre(),
                    if ok {
                        "ok".to_string()
                    } else {
                        format!("falló ({codigo:?})")
                    }
                );
            }
            Err(_) => break,
        }
    }
    println!(
        "lote {}: {} ok, {fallas} fallaron, {} omitidos",
        accion.nombre(),
        fin - fallas,
        omitidos.len()
    );
    if fallas > 0 {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

/// `guion reel nuevo <cuenta> <slug> [--titulo "…"]`.
pub fn run_nuevo(args: &[String]) -> ExitCode {
    let o = match opciones(args) {
        Ok(o) => o,
        Err(e) => return fallo(e, USO_NUEVO),
    };
    let [cuenta, slug] = o.resto.as_slice() else {
        return fallo("faltan la cuenta y el slug", USO_NUEVO);
    };
    let t = match Tablero::leer(&o.workspaces, &|_| None) {
        Ok(t) => t,
        Err(e) => return fallo(e, USO_NUEVO),
    };
    let Some(c) = t.cuenta(cuenta) else {
        let hay: Vec<&str> = t.cuentas.iter().map(|c| c.nombre.as_str()).collect();
        return fallo(
            format!("no hay cuenta «{cuenta}» (hay: {})", hay.join(", ")),
            USO_NUEVO,
        );
    };
    if let Some(id) = &o.encargo {
        // the encargo must be this account's before anything is created
        if !t.pedidos_de(c).any(|e| &e.encargo.id == id) {
            return fallo(format!("{} no tiene el encargo {id}", c.nombre), USO_NUEVO);
        }
    }
    match nuevo::crear(c, slug, o.titulo.as_deref(), &fecha::hoy()) {
        Ok(n) => {
            println!("nuevo post: {}", n.dir.display());
            for a in &n.archivos {
                println!("  {a}");
            }
            for r in &n.registros {
                println!("  registrado en {}", r.display());
            }
            if let Some(id) = &o.encargo {
                match crate::encargos::tomar(&c.raiz, id, slug) {
                    Ok(e) => println!("  encargo {}: {} → {slug}", e.id, e.estado),
                    Err(e) => return fallo(e, USO_NUEVO),
                }
            }
            ExitCode::SUCCESS
        }
        Err(e) => fallo(e, USO_NUEVO),
    }
}

/// `guion encargo …` (R-0013 AC11, AC12): what the owner asks and the agent
/// takes, from the command line.
pub fn run_encargo(args: &[String]) -> ExitCode {
    if args.is_empty() || args.iter().any(|a| a == "-h" || a == "--help") {
        eprintln!("{USO_ENCARGO}");
        return if args.is_empty() {
            ExitCode::from(2)
        } else {
            ExitCode::SUCCESS
        };
    }
    let o = match opciones(args) {
        Ok(o) => o,
        Err(e) => return fallo(e, USO_ENCARGO),
    };
    let t = match Tablero::leer(&o.workspaces, &|_| None) {
        Ok(t) => t,
        Err(e) => return fallo(e, USO_ENCARGO),
    };
    let estado_de = |v: &str| v.replace('-', " ");
    let r = match o
        .resto
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        ["nuevo", cuenta, idea @ ..] if !idea.is_empty() => t
            .encargar_con(cuenta, &idea.join(" "), &fecha::hoy(), &o.detalles)
            .map(|e| format!("encargo {} para {}: {}", e.id, e.cuenta, e.idea)),
        ["tomar", id, post] => t
            .marcar(id, "en curso", Some(post), o.cuenta.as_deref())
            .map(|e| format!("encargo {}: en curso → {post}", e.id)),
        ["marcar", id, estado @ ..] if !estado.is_empty() => t
            .marcar(
                id,
                &estado_de(&estado.join(" ")),
                o.detalles.post.as_deref(),
                o.cuenta.as_deref(),
            )
            .map(|e| format!("encargo {}: {}", e.id, e.estado)),
        ["lista"] => {
            let filas: Vec<&crate::tablero::EncargoEn> = t
                .encargos
                .iter()
                .filter(|e| {
                    o.cuenta
                        .as_deref()
                        .is_none_or(|n| crate::tablero::mismo_nombre(&e.encargo.cuenta, n))
                })
                .collect();
            if o.json {
                Ok(serde_json::to_string_pretty(&filas).expect("the encargos serialize"))
            } else {
                Ok(filas
                    .iter()
                    .map(|e| {
                        let post = e.post_ruta.as_deref().or(e.encargo.post.as_deref());
                        format!(
                            "{}  {:<10}  {}  {}{}",
                            e.encargo.id,
                            e.vivo,
                            e.encargo.cuenta,
                            e.encargo.idea,
                            post.map(|p| format!("  → {p}")).unwrap_or_default()
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n"))
            }
        }
        _ => Err("¿nuevo, tomar, marcar o lista?".to_string()),
    };
    match r {
        Ok(m) => {
            println!("{m}");
            ExitCode::SUCCESS
        }
        Err(e) => fallo(e, USO_ENCARGO),
    }
}
