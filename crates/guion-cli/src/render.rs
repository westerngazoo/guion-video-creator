use std::path::{Path, PathBuf};
use std::process::ExitCode;

use guion_assemble::{assemble_at, duration};
use guion_audio::timeline_duration;
use guion_brand::{default_theme, postfx_dir};
use guion_core::{load_and_check, Format};
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

    let sp = match load_and_check(&path) {
        Ok(sp) => sp,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(1);
        }
    };

    // R-0008 AC3: `render` también respeta la marca del guion. Antes
    // llamaba a `default_theme()` a secas, así que `render` y `encode`
    // podían sacar la misma pieza con dos identidades distintas.
    let theme = match sp.meta.theme.as_deref() {
        None => default_theme(),
        Some(n) => match guion_brand::theme_by_name(n) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("{e}");
                return ExitCode::from(1);
            }
        },
    };
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
    // R-0008 OQ-1: el cuadro se limpia con el papel de la marca. Antes
    // salía negro de fábrica en las dos marcas y sólo cambiaban los
    // trazos, o sea que en `fbf` —papel crema— el fondo era lo único que
    // no llegaba, y es la mitad de la pieza.
    let mut sink = match PpmSink::with_view(&dir, size, view) {
        Ok(s) => s.with_fonts(fuentes).with_background(theme.paper()),
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
