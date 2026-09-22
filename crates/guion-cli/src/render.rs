use std::path::{Path, PathBuf};
use std::process::ExitCode;

use guion_assemble::{assemble_at, duration};
use guion_audio::timeline_duration;
use guion_brand::{default_theme, postfx_dir};
use guion_core::{dialecto, load_and_check, Dialecto, Format};
use guion_motion::ModelRuntime;
use motoreel::{FrameSink, PpmSink};

use crate::path::{resolve_screenplay_path, screenplay_not_found};
use crate::{default_out, frame_count};

pub fn run(args: &[String]) -> ExitCode {
    let mut path: Option<String> = None;
    let mut out: Option<PathBuf> = None;
    let mut fps: Option<f64> = None;
    let mut brand = true;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--out" => {
                i += 1;
                out = args.get(i).map(PathBuf::from);
                if out.is_none() {
                    eprintln!("--out requiere un directorio");
                    return ExitCode::from(2);
                }
            }
            "--fps" => {
                i += 1;
                fps = args.get(i).and_then(|s| s.parse().ok());
                if fps.is_none() {
                    eprintln!("--fps requiere un número");
                    return ExitCode::from(2);
                }
            }
            "--no-brand" => brand = false,
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

    // El dialecto se decide antes de leer. Ver `guion_core::dialecto`.
    let texto = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("{}: {e}", path.display());
            return ExitCode::from(1);
        }
    };
    match dialecto(&texto) {
        Ok(Dialecto::Comparacion) => {
            // Una repetición a los fps que pida el creador, 2 s por
            // omisión: la gráfica no tiene timeline propio del que
            // sacar una duración.
            let fps = fps.unwrap_or(30.0);
            let cuadros = frame_count(2.0, fps);
            let dir = out.unwrap_or_else(|| default_out("comparacion"));
            return crate::comparacion::render(&path, &dir, cuadros);
        }
        Ok(Dialecto::Escena) => {}
        Err(e) => {
            eprintln!("{}: {e}", path.display());
            return ExitCode::from(1);
        }
    }

    let sp = match load_and_check(&path) {
        Ok(sp) => sp,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(1);
        }
    };

    let theme = default_theme();
    let dur = timeline_duration(&sp).max(duration(&sp));
    let fps = fps.unwrap_or(sp.meta.fps);
    let frames = frame_count(dur, fps);
    let dir = out.unwrap_or_else(|| default_out(&sp.meta.slug));

    let (size, view) = raster_for(&sp.meta);
    // R-0009: el motor ya no trae tipografía adentro. Sin registro,
    // un primitivo de texto es un error y no un cuadro en blanco.
    let fuentes = match guion_brand::fuentes::marca() {
        Ok(f) => f,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(1);
        }
    };
    let mut sink = match PpmSink::with_view(&dir, size, view) {
        Ok(s) => s.with_fonts(fuentes),
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(1);
        }
    };

    let mut runtime = ModelRuntime::new();
    for index in 0..frames {
        let t = index as f64 / fps;
        let scene = match assemble_at(&sp, &theme, t, &mut runtime) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("{e}");
                return ExitCode::from(1);
            }
        };
        let prims = scene.eval(0.0);
        if let Err(e) = sink.frame(index, &prims) {
            eprintln!("{e}");
            return ExitCode::from(1);
        }
    }

    if brand {
        if let Err(e) = postfx_dir(&dir, frames, theme.grain, theme.halftone_alpha) {
            eprintln!("post-fx: {e}");
            return ExitCode::from(1);
        }
    }

    println!("{} frames → {}", frames, dir.display());
    ExitCode::SUCCESS
}

fn raster_for(meta: &guion_core::Meta) -> ((u32, u32), (f64, f64)) {
    match meta.format {
        Format::Vertical => ((1080, 1920), (1.8, 3.2)),
    }
}
