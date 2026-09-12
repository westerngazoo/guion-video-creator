//! Shared studio pipeline used by the Tauri shell and tests.

use std::fs;
use std::path::{Path, PathBuf};

use guion_assemble::{assemble_at, duration};
use guion_audio::timeline_duration;
use guion_audio::{default_narration_path, mix_for_encode, write_narration, NarrateOptions};
use guion_brand::{default_theme, postfx_dir};
use guion_core::load_and_check;
use guion_encode::{default_mp4, encode_ppm_dir};
use guion_motion::ModelRuntime;
use motoreel::{FrameSink, PpmSink};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ScreenplayInfo {
    pub title: String,
    pub slug: String,
    pub fps: f64,
    pub duration_secs: f64,
    pub path: String,
}

#[derive(Debug, Serialize)]
pub struct TemplateEntry {
    pub name: String,
    pub path: String,
}

pub fn load_screenplay(path: &Path) -> Result<ScreenplayInfo, String> {
    let sp = load_and_check(path).map_err(|e| e.to_string())?;
    Ok(ScreenplayInfo {
        title: sp.meta.title.clone(),
        slug: sp.meta.slug.clone(),
        fps: sp.meta.fps,
        duration_secs: timeline_duration(&sp).max(duration(&sp)),
        path: path.display().to_string(),
    })
}

pub fn list_templates(templates_dir: &Path) -> Result<Vec<TemplateEntry>, String> {
    let mut out = Vec::new();
    if !templates_dir.is_dir() {
        return Ok(out);
    }
    for entry in fs::read_dir(templates_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("toml")
            && path
                .file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.contains("screenplay"))
                .unwrap_or(false)
        {
            out.push(TemplateEntry {
                name: path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("?")
                    .to_string(),
                path: path.display().to_string(),
            });
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

pub fn preview_frame(path: &Path, t: f64) -> Result<String, String> {
    let sp = load_and_check(path).map_err(|e| e.to_string())?;
    let theme = default_theme();
    let mut runtime = ModelRuntime::new();
    let scene = assemble_at(&sp, &theme, t, &mut runtime).map_err(|e| e.to_string())?;
    let _ = scene.eval(0.0);
    Ok(format!(
        "preview t={:.2}s · {} objetos",
        t,
        sp.object.len()
    ))
}

pub fn render_screenplay(path: &Path, out_dir: Option<PathBuf>, fps: Option<f64>) -> Result<String, String> {
    let sp = load_and_check(path).map_err(|e| e.to_string())?;
    let theme = default_theme();
    let fps = fps.unwrap_or(sp.meta.fps);
    let frames = (timeline_duration(&sp).max(duration(&sp)) * fps).ceil() as usize;
    let dir = out_dir.unwrap_or_else(|| PathBuf::from("out").join(&sp.meta.slug).join("frames"));
    let (size, view) = ((1080u32, 1920u32), (1.8, 3.2));
    let mut sink = PpmSink::with_view(&dir, size, view).map_err(|e| e.to_string())?;
    let mut runtime = ModelRuntime::new();
    for index in 0..frames {
        let t = index as f64 / fps;
        let scene = assemble_at(&sp, &theme, t, &mut runtime).map_err(|e| e.to_string())?;
        sink.frame(index, &scene.eval(0.0)).map_err(|e| e.to_string())?;
    }
    postfx_dir(&dir, frames, theme.grain, theme.halftone_alpha).map_err(|e| e.to_string())?;
    Ok(format!("{} frames → {}", frames, dir.display()))
}

pub fn narrate_screenplay(path: &Path, engine: &str) -> Result<String, String> {
    let sp = load_and_check(path).map_err(|e| e.to_string())?;
    let screenplay_dir = path.parent().unwrap_or(Path::new("."));
    let out = default_narration_path(&sp.meta.slug);
    let opts = NarrateOptions {
        engine: engine.to_string(),
        piper_model: None,
        script_path: None,
        include_bed: false,
    };
    write_narration(&sp, screenplay_dir, &out, &opts).map_err(|e| e.to_string())?;
    Ok(out.display().to_string())
}

pub fn encode_screenplay(path: &Path, out_mp4: Option<PathBuf>) -> Result<String, String> {
    let sp = load_and_check(path).map_err(|e| e.to_string())?;
    let screenplay_dir = path.parent().unwrap_or(Path::new("."));
    let fps = sp.meta.fps;
    let dir = PathBuf::from("out").join(&sp.meta.slug).join("frames");
    if !dir.join("frame_00000.ppm").exists() {
        render_screenplay(path, Some(dir.clone()), Some(fps))?;
    }
    let narr = default_narration_path(&sp.meta.slug);
    let audio = mix_for_encode(
        &sp,
        screenplay_dir,
        narr.exists().then_some(narr.as_path()),
    )
    .map_err(|e| e.to_string())?;
    let mp4 = out_mp4.unwrap_or_else(|| default_mp4(&sp.meta.slug));
    encode_ppm_dir(&dir, fps, &mp4, audio.as_deref()).map_err(|e| e.to_string())?;
    Ok(mp4.display().to_string())
}

pub fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

pub fn templates_dir() -> PathBuf {
    workspace_root().join("templates")
}
