use std::path::{Path, PathBuf};
use std::process::ExitCode;

use guion_assemble::{assemble_at, duration};
use guion_brand::{default_theme, postfx_dir, Theme};
use guion_core::load_and_check;
use guion_encode::{default_mp4, encode_ppm_dir};
use guion_motion::ModelRuntime;
use motoreel::{FrameSink, PpmSink};

use crate::path::{resolve_screenplay_path, screenplay_not_found};
use crate::{default_out, frame_count};

pub fn run(args: &[String]) -> ExitCode {
    let mut path: Option<String> = None;
    let mut frames_dir: Option<PathBuf> = None;
    let mut out_mp4: Option<PathBuf> = None;
    let mut fps: Option<f64> = None;
    let mut brand = true;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--frames" => {
                i += 1;
                frames_dir = args.get(i).map(PathBuf::from);
            }
            "--out" => {
                i += 1;
                out_mp4 = args.get(i).map(PathBuf::from);
            }
            "--fps" => {
                i += 1;
                fps = args.get(i).and_then(|s| s.parse().ok());
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

    let theme = theme_for(&sp);
    let fps = fps.unwrap_or(sp.meta.fps);
    let dur = duration(&sp);
    let frames = frame_count(dur, fps);
    let dir = frames_dir.unwrap_or_else(|| default_out(&sp.meta.slug));
    let mp4 = out_mp4.unwrap_or_else(|| default_mp4(&sp.meta.slug));

    if !dir.join("frame_00000.ppm").exists() {
        if let Some(code) = render_frames(&sp, &theme, &dir, fps, frames, brand) {
            return code;
        }
    }

    let audio_path = sp
        .audio
        .as_ref()
        .and_then(|a| a.path.as_ref())
        .map(PathBuf::from);
    match encode_ppm_dir(&dir, fps, &mp4, audio_path.as_deref()) {
        Ok(()) => {
            println!("{} → {}", frames, mp4.display());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{e}");
            ExitCode::from(1)
        }
    }
}

fn theme_for(sp: &guion_core::Screenplay) -> Theme {
    match sp.meta.theme.as_deref() {
        Some("fbf") | None => default_theme(),
        _ => default_theme(),
    }
}

fn render_frames(
    sp: &guion_core::Screenplay,
    theme: &Theme,
    dir: &PathBuf,
    fps: f64,
    frames: usize,
    brand: bool,
) -> Option<ExitCode> {
    let (size, view) = match sp.meta.format {
        guion_core::Format::Vertical => ((1080, 1920), (1.8, 3.2)),
    };
    let mut sink = match PpmSink::with_view(dir, size, view) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{e}");
            return Some(ExitCode::from(1));
        }
    };
    let mut runtime = ModelRuntime::new();
    for index in 0..frames {
        let t = index as f64 / fps;
        let scene = match assemble_at(sp, theme, t, &mut runtime) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("{e}");
                return Some(ExitCode::from(1));
            }
        };
        if let Err(e) = sink.frame(index, &scene.eval(0.0)) {
            eprintln!("{e}");
            return Some(ExitCode::from(1));
        }
    }
    if brand {
        if let Err(e) = postfx_dir(dir, frames, theme.grain, theme.halftone_alpha) {
            eprintln!("post-fx: {e}");
            return Some(ExitCode::from(1));
        }
    }
    None
}
