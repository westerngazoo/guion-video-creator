//! The command lines (R-0013 AC3, AC9, AC10):
//!
//! ```text
//! guion tablero [--workspace DIR...] [--paralelo N]
//! guion tablero --json [--workspace DIR...]
//! guion tablero lote <render|check|voz> [--cuenta X] [--estado Y] [--paralelo N] [--dry-run] [--workspace DIR...]
//! guion reel nuevo <cuenta> <slug> [--titulo "…"] [--workspace DIR...]
//! ```
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
pub const USO: &str = "uso:\n  guion tablero [--workspace DIR...] [--paralelo N]\n  \
guion tablero --json [--workspace DIR...]\n  \
guion tablero lote <render|check|voz> [--cuenta X] [--estado Y] [--paralelo N] [--dry-run] [--workspace DIR...]";

/// `guion reel nuevo`'s usage.
pub const USO_NUEVO: &str =
    "uso:\n  guion reel nuevo <cuenta> <slug> [--titulo \"…\"] [--workspace DIR...]";

/// The parsed flags.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Opciones {
    pub workspaces: Vec<PathBuf>,
    pub json: bool,
    pub paralelo: usize,
    pub cuenta: Option<String>,
    pub estado: Option<Estado>,
    pub dry_run: bool,
    pub titulo: Option<String>,
    /// Positional arguments (`lote <accion>`; `nuevo <cuenta> <slug>`).
    pub resto: Vec<String>,
}

/// Parses the flags; `--workspace` takes every following argument up to the
/// next flag (or `lote`).
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
            f if f.starts_with('-') => return Err(format!("bandera desconocida {f}")),
            p => o.resto.push(p.to_string()),
        }
        i += 1;
    }
    if o.workspaces.is_empty() {
        o.workspaces
            .push(std::env::current_dir().map_err(|e| e.to_string())?);
    }
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
    match nuevo::crear(c, slug, o.titulo.as_deref(), &fecha::hoy()) {
        Ok(n) => {
            println!("nuevo post: {}", n.dir.display());
            for a in &n.archivos {
                println!("  {a}");
            }
            for r in &n.registros {
                println!("  registrado en {}", r.display());
            }
            ExitCode::SUCCESS
        }
        Err(e) => fallo(e, USO_NUEVO),
    }
}
