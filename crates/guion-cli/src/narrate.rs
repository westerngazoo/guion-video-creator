use std::path::{Path, PathBuf};
use std::process::ExitCode;

use guion_audio::{default_narration_path, write_narration, NarrateOptions};
use guion_core::load_and_check;

use crate::path::{resolve_screenplay_path, screenplay_not_found};

pub fn run(args: &[String]) -> ExitCode {
    let mut path: Option<String> = None;
    let mut out: Option<PathBuf> = None;
    let mut engine = "scaffold".to_string();
    let mut piper_model: Option<String> = None;
    let mut with_bed = false;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--out" => {
                i += 1;
                out = args.get(i).map(PathBuf::from);
            }
            "--engine" => {
                i += 1;
                engine = args.get(i).cloned().unwrap_or_else(|| "scaffold".into());
            }
            "--piper-model" => {
                i += 1;
                piper_model = args.get(i).cloned();
            }
            "--with-bed" => with_bed = true,
            flag if flag.starts_with('-') => {
                eprintln!("flag desconocido: {flag}");
                return ExitCode::from(2);
            }
            p => {
                if path.is_some() {
                    eprintln!("argumento inesperado: {p}");
                    return ExitCode::from(2);
                }
                path = Some(p.to_string());
            }
        }
        i += 1;
    }

    let raw = match path {
        Some(p) => p,
        None => {
            eprintln!("falta la ruta del screenplay");
            return ExitCode::from(2);
        }
    };
    let path = resolve_screenplay_path(Path::new(&raw));
    if !path.exists() {
        eprintln!("{}", screenplay_not_found(Path::new(&raw), &path));
        return ExitCode::from(1);
    }

    let sp = match load_and_check(&path) {
        Ok(sp) => sp,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(1);
        }
    };

    let screenplay_dir = path.parent().unwrap_or(Path::new(".")).to_path_buf();
    let out = out.unwrap_or_else(|| default_narration_path(&sp.meta.slug));
    let opts = NarrateOptions {
        engine,
        piper_model,
        script_path: None,
        include_bed: with_bed,
    };

    match write_narration(&sp, &screenplay_dir, &out, &opts) {
        Ok(written) => {
            println!(
                "narración ({}) → {}",
                opts.engine,
                written.display()
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{e}");
            ExitCode::from(1)
        }
    }
}
