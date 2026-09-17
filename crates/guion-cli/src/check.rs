use std::path::Path;
use std::process::ExitCode;

use guion_core::load_and_check;

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
