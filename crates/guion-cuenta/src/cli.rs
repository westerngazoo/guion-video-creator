//! `guion cuenta` (R-0014 AC3, AC2):
//!
//! ```text
//! guion cuenta nueva <dir> --nombre "…" --tema "…" --revisor NOMBRE --materia "…"
//!     [--slug S] [--handle @X] [--idioma es|en] [--publico "…"] [--dueno "…"]
//!     [--fecha AAAA-MM-DD] [--voz MODELO] [--tono "…"] [--trato "…"]
//!     [--regla "…"]... [--paleta token=#rrggbb,...] [--fuente-titulo F] [--fuente-texto F]
//!     [--hashtags "#a #b"] [--guion RUTA] [--repo RAIZ] [--interactivo] [--al-tablero]
//! guion cuenta perfil <dir> [--check]
//! ```
//!
//! What is missing is asked on the terminal (or always, with `--interactivo`);
//! without a terminal, a missing answer is an error naming its flag. Exit
//! codes: 0 done, 1 `perfil --check` found a drift, 2 usage or input error.

use std::io::{BufRead, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use crate::contrato::Cuenta;
use crate::nueva::{self, parse_paleta, Respuestas};
use crate::perfil;
use crate::texto::a_slug;

/// `guion cuenta`'s usage.
pub const USO: &str = "uso:\n  guion cuenta nueva <dir> --nombre \"…\" --tema \"…\" --revisor NOMBRE --materia \"…\"\n      \
[--slug S] [--handle @X] [--idioma es|en] [--publico \"…\"] [--dueno \"…\"] [--fecha AAAA-MM-DD]\n      \
[--voz MODELO] [--tono \"…\"] [--trato \"…\"] [--regla \"…\"]... [--paleta token=#rrggbb,...]\n      \
[--fuente-titulo F] [--fuente-texto F] [--hashtags \"#a #b\"] [--guion RUTA] [--repo RAIZ]\n      \
[--interactivo] [--al-tablero]\n  guion cuenta perfil <dir> [--check]";

/// Adds a repo to the user's dashboard list (guion-tablero's `repos::agregar`
/// at its default file); `Ok(message)`.
pub type AlTablero<'a> = &'a dyn Fn(&Path) -> Result<String, String>;

fn fallo(msg: impl std::fmt::Display) -> ExitCode {
    eprintln!("guion cuenta: {msg}\n{USO}");
    ExitCode::from(2)
}

/// `guion cuenta …`. `hoy` is the default `--fecha`.
pub fn run(args: &[String], hoy: &str, al_tablero: AlTablero) -> ExitCode {
    match args.first().map(String::as_str) {
        None | Some("-h") | Some("--help") => {
            eprintln!("{USO}");
            ExitCode::from(2)
        }
        Some("nueva") => nueva_cmd(&args[1..], hoy, al_tablero),
        Some("perfil") => perfil_cmd(&args[1..]),
        Some(x) => fallo(format!("subcomando desconocido {x}")),
    }
}

/// The parsed flags of `guion cuenta nueva`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Banderas {
    pub dir: Option<PathBuf>,
    pub repo: Option<PathBuf>,
    pub interactivo: bool,
    pub al_tablero: bool,
    /// (flag without `--`, value), in order.
    pub valores: Vec<(String, String)>,
}

const CON_VALOR: [&str; 20] = [
    "nombre",
    "slug",
    "handle",
    "idioma",
    "tema",
    "publico",
    "dueno",
    "fecha",
    "voz",
    "tono",
    "trato",
    "revisor",
    "materia",
    "regla",
    "paleta",
    "fuente-titulo",
    "fuente-texto",
    "hashtags",
    "guion",
    "repo",
];

/// Parses `nueva`'s flags.
pub fn banderas(args: &[String]) -> Result<Banderas, String> {
    let mut b = Banderas::default();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--interactivo" => b.interactivo = true,
            "--al-tablero" => b.al_tablero = true,
            f if f.starts_with("--") => {
                let k = &f[2..];
                if !CON_VALOR.contains(&k) {
                    return Err(format!("bandera desconocida {f}"));
                }
                let v = it.next().ok_or(format!("{f} necesita un valor"))?;
                if k == "repo" {
                    b.repo = Some(PathBuf::from(v));
                } else {
                    b.valores.push((k.to_string(), v.clone()));
                }
            }
            p if b.dir.is_none() => b.dir = Some(PathBuf::from(p)),
            p => return Err(format!("argumento inesperado {p}")),
        }
    }
    Ok(b)
}

fn get<'a>(b: &'a Banderas, k: &str) -> Option<&'a str> {
    b.valores
        .iter()
        .rev()
        .find(|(n, _)| n == k)
        .map(|(_, v)| v.as_str())
}

/// The answers from the flags (defaults for the rest).
pub fn respuestas(b: &Banderas, hoy: &str) -> Result<Respuestas, String> {
    let nombre = get(b, "nombre").unwrap_or("");
    let mut r = Respuestas::con_defaults(
        nombre,
        get(b, "tema").unwrap_or(""),
        get(b, "revisor").unwrap_or(""),
        get(b, "materia").unwrap_or(""),
    );
    r.fecha = get(b, "fecha").unwrap_or(hoy).to_string();
    let texto = |k: &str, campo: &mut String| {
        if let Some(v) = get(b, k) {
            *campo = v.to_string();
        }
    };
    texto("slug", &mut r.slug);
    texto("handle", &mut r.handle);
    texto("idioma", &mut r.idioma);
    texto("publico", &mut r.publico);
    texto("dueno", &mut r.dueno);
    texto("voz", &mut r.voz_modelo);
    texto("tono", &mut r.tono);
    texto("trato", &mut r.trato);
    texto("fuente-titulo", &mut r.fuente_titulo);
    texto("fuente-texto", &mut r.fuente_texto);
    r.reglas = b
        .valores
        .iter()
        .filter(|(k, _)| k == "regla")
        .map(|(_, v)| v.clone())
        .collect();
    if let Some(p) = get(b, "paleta") {
        r.paleta = parse_paleta(p)?;
    }
    if let Some(h) = get(b, "hashtags") {
        r.hashtags = h.split_whitespace().map(String::from).collect();
    }
    r.guion = get(b, "guion").map(String::from);
    Ok(r)
}

/// The answers a person must give (no default makes sense).
fn faltan(r: &Respuestas) -> Vec<&'static str> {
    let mut f = Vec::new();
    if r.nombre.trim().is_empty() {
        f.push("nombre");
    }
    if r.tema.trim().is_empty() {
        f.push("tema");
    }
    if r.revisor.trim().is_empty() {
        f.push("revisor");
    }
    if r.materia.trim().is_empty() {
        f.push("materia");
    }
    f
}

/// Asks on `entrada`/`salida`: the missing answers, or every one with
/// `todo`. Enter keeps the value in brackets. Rules are asked one per line
/// until an empty one.
pub fn preguntar(
    r: &mut Respuestas,
    todo: bool,
    entrada: &mut dyn BufRead,
    salida: &mut dyn Write,
) -> Result<(), String> {
    fn leer_en(
        entrada: &mut dyn BufRead,
        salida: &mut dyn Write,
        pregunta: &str,
        actual: &str,
    ) -> Result<String, String> {
        if actual.is_empty() {
            write!(salida, "{pregunta}: ")
        } else {
            write!(salida, "{pregunta} [{actual}]: ")
        }
        .and_then(|()| salida.flush())
        .map_err(|e| e.to_string())?;
        let mut l = String::new();
        entrada.read_line(&mut l).map_err(|e| e.to_string())?;
        let l = l.trim();
        Ok(if l.is_empty() { actual } else { l }.to_string())
    }
    let mut leer = |pregunta: &str, actual: &str| leer_en(entrada, salida, pregunta, actual);
    let pedir = |vacio: bool| todo || vacio;
    if pedir(r.nombre.is_empty()) {
        r.nombre = leer("Nombre de la cuenta", &r.nombre)?;
        if r.slug.is_empty() || todo {
            r.slug = a_slug(&r.nombre);
        }
    }
    if todo {
        r.slug = leer("Slug (carpetas y crate)", &r.slug)?;
        r.handle = leer("Usuario de Instagram (vacío: por reservar)", &r.handle)?;
        r.idioma = leer("Idioma de lo publicado (es | en)", &r.idioma)?;
    }
    if pedir(r.tema.is_empty()) {
        r.tema = leer("Tema (una línea)", &r.tema)?;
    }
    if todo {
        r.publico = leer("Público", &r.publico)?;
        r.dueno = leer("Dueño", &r.dueno)?;
        r.tono = leer("Tono de la voz", &r.tono)?;
    }
    if pedir(r.revisor.is_empty()) {
        r.revisor = leer("Revisor de la materia (nombre del agente)", &r.revisor)?;
    }
    if pedir(r.materia.is_empty()) {
        r.materia = leer("Materia que revisa", &r.materia)?;
    }
    if todo || r.reglas.is_empty() {
        loop {
            let l = leer("Regla propia de la materia (vacía: terminar)", "")?;
            if l.is_empty() {
                break;
            }
            r.reglas.push(l);
        }
    }
    Ok(())
}

fn nueva_cmd(args: &[String], hoy: &str, al_tablero: AlTablero) -> ExitCode {
    let b = match banderas(args) {
        Ok(b) => b,
        Err(e) => return fallo(e),
    };
    let Some(dir) = b.dir.clone() else {
        return fallo("falta la carpeta de la cuenta");
    };
    let mut r = match respuestas(&b, hoy) {
        Ok(r) => r,
        Err(e) => return fallo(e),
    };
    let falta = faltan(&r);
    if b.interactivo || !falta.is_empty() {
        if !std::io::stdin().is_terminal() {
            if !falta.is_empty() {
                let f: Vec<String> = falta.iter().map(|k| format!("--{k}")).collect();
                return fallo(format!(
                    "faltan {} (sin terminal no se pregunta)",
                    f.join(", ")
                ));
            }
        } else {
            let stdin = std::io::stdin();
            let mut entrada = stdin.lock();
            let mut salida = std::io::stderr();
            if let Err(e) = preguntar(&mut r, b.interactivo, &mut entrada, &mut salida) {
                return fallo(e);
            }
        }
    }
    match nueva::crear(&r, &dir, b.repo.as_deref()) {
        Ok(c) => {
            println!("cuenta nueva: {} ({})", r.nombre, c.dir.display());
            for a in &c.archivos {
                println!("  {a}");
            }
            println!("  cuentas: {}", c.cuentas.display());
            if b.al_tablero {
                match al_tablero(&c.raiz) {
                    Ok(m) => println!("  {m}"),
                    Err(e) => eprintln!("guion cuenta: --al-tablero: {e}"),
                }
            }
            println!(
                "por decidir a mano: la dirección de diseño y la paleta, las caras (brand/fonts/), la voz y el logo (CLAUDE.md §4)"
            );
            ExitCode::SUCCESS
        }
        Err(e) => fallo(e),
    }
}

fn perfil_cmd(args: &[String]) -> ExitCode {
    let mut dir = None;
    let mut check = false;
    for a in args {
        match a.as_str() {
            "--check" => check = true,
            f if f.starts_with('-') => return fallo(format!("bandera desconocida {f}")),
            p if dir.is_none() => dir = Some(PathBuf::from(p)),
            p => return fallo(format!("argumento inesperado {p}")),
        }
    }
    let dir = dir.unwrap_or_else(|| PathBuf::from("."));
    let c = match Cuenta::load(&dir) {
        Ok(c) => c,
        Err(e) => return fallo(e),
    };
    let texto = perfil::perfil(&c);
    let f = dir.join(perfil::RUTA);
    let actual = std::fs::read_to_string(&f).ok();
    if check {
        return if actual.as_deref() == Some(texto.as_str()) {
            println!("{}: al día con cuenta.toml", f.display());
            ExitCode::SUCCESS
        } else {
            eprintln!(
                "{}: no coincide con cuenta.toml (guion cuenta perfil {})",
                f.display(),
                dir.display()
            );
            ExitCode::FAILURE
        };
    }
    if let Some(p) = f.parent() {
        if let Err(e) = std::fs::create_dir_all(p) {
            return fallo(format!("{}: {e}", p.display()));
        }
    }
    match std::fs::write(&f, texto) {
        Ok(()) => {
            println!("{}", f.display());
            ExitCode::SUCCESS
        }
        Err(e) => fallo(format!("{}: {e}", f.display())),
    }
}
