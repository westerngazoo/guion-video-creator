use std::path::Path;
use std::process::ExitCode;

use guion_core::{dialecto, load_and_check, Dialecto};

use crate::path::{resolve_screenplay_path, screenplay_not_found};

pub fn run(path: Option<&String>) -> ExitCode {
    let path = match path {
        Some(p) => Path::new(p),
        None => {
            eprintln!("falta la ruta del screenplay");
            return ExitCode::from(2);
        }
    };
    let resolved = resolve_screenplay_path(path);
    if !resolved.exists() {
        eprintln!("{}", screenplay_not_found(path, &resolved));
        return ExitCode::from(1);
    }
    // Qué clase de guion es, ANTES de intentar leerlo. Adivinar —probar
    // un dialecto y si falla el otro— haría que un `[meta]` con una llave
    // mal escrita reportara el error del dialecto equivocado.
    let texto = match std::fs::read_to_string(&resolved) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("{}: {e}", resolved.display());
            return ExitCode::from(1);
        }
    };
    match dialecto(&texto) {
        Ok(Dialecto::Comparacion) => return crate::comparacion::check(&resolved),
        Ok(Dialecto::Escena) => {}
        Err(e) => {
            eprintln!("{}: {e}", resolved.display());
            return ExitCode::from(1);
        }
    }
    match load_and_check(&resolved) {
        Ok(sp) => {
            println!("ok: {} ({})", sp.meta.title, sp.meta.slug);
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{e}");
            ExitCode::from(1)
        }
    }
}
