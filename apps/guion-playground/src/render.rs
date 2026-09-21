//! motoreel 2D rendering for the pendulum side view.

use std::fs;
use std::io::Cursor;
use std::path::Path;

use base64::{engine::general_purpose::STANDARD, Engine as _};
use garust::{pga, Motor3};
use guion_brand::default_theme;
use guion_encode::encode_ppm_dir;
use motoreel::{Camera, FrameSink, Object, PpmSink, Prim2, Rgb, Scene, Style, Track};

/// Centred image-space window for the side-view pendulum.
pub const VIEW: (f64, f64) = (2.8, 2.2);

/// Preview raster size (px).
pub const PREVIEW_SIZE: (u32, u32) = (960, 720);

/// Export raster size (px) — vertical-friendly 16:9 landscape panel.
pub const EXPORT_SIZE: (u32, u32) = (1920, 1080);

fn anchor_style(accent: Rgb) -> Style {
    Style {
        stroke: accent,
        width: 0.02,
        alpha: 1.0,
    }
}

/// Slightly lifted from pure black so the rod reads clearly in the UI.
const SCENE_BACKGROUND: Rgb = Rgb {
    r: 0x1a,
    g: 0x1a,
    b: 0x1a,
};

fn rod_style(paper: Rgb) -> Style {
    Style {
        stroke: paper,
        width: 0.02,
        alpha: 1.0,
    }
}

fn bob_style(accent: Rgb) -> Style {
    Style {
        stroke: accent,
        width: 0.05,
        alpha: 1.0,
    }
}

fn side_camera() -> Camera {
    Camera::orthographic(Motor3::translator(0.0, 0.0, 5.0))
}

/// Build a single-frame scene for the pendulum at simulation time `t`.
pub fn pendulum_scene_at(track: &Track, t: f64, _length: f64) -> Scene {
    let theme = default_theme();
    let accent = theme.stroke_color("accent", None);
    let paper = theme.stroke_color("paper", None);
    let pose = track.eval(t);
    // Rod runs pivot → bob centre. The hinge anchor `(0, ℓ, 0)` in body space
    // is constrained to the pivot, so it must not be used as the visible endpoint.
    let bob_world = pga::Point::new(0.0, 0.0, 0.0).transform(&pose);

    let mut scene = Scene::new(1.0);
    scene.view = VIEW;
    scene.camera = side_camera();

    scene.add(Object::point(pga::Point::new(0.0, 0.0, 0.0)).with_style(anchor_style(accent)));
    scene.add(
        Object::segment(pga::Point::new(0.0, 0.0, 0.0), bob_world).with_style(rod_style(paper)),
    );
    scene.add(
        Object::point(pga::Point::new(0.0, 0.0, 0.0))
            .with_style(bob_style(accent))
            .at(pose),
    );
    scene
}

pub fn eval_frame(track: &Track, t: f64, length: f64) -> Vec<Prim2> {
    pendulum_scene_at(track, t, length).eval(0.0)
}

/// Render one preview frame as a base64 PNG data URL.
pub fn preview_png_base64(track: &Track, t: f64, length: f64) -> Result<String, String> {
    let dir = std::env::temp_dir().join("guion-playground-preview");
    if dir.exists() {
        fs::remove_dir_all(&dir).map_err(|e| e.to_string())?;
    }
    let mut sink = PpmSink::with_view(&dir, PREVIEW_SIZE, VIEW)
        .map_err(|e| e.to_string())?
        .with_background(SCENE_BACKGROUND);
    sink.frame(0, &eval_frame(track, t, length))
        .map_err(|e| e.to_string())?;
    let ppm_path = dir.join("frame_00000.ppm");
    let img = image::open(&ppm_path).map_err(|e| e.to_string())?;
    let mut png_bytes = Vec::new();
    img.write_to(&mut Cursor::new(&mut png_bytes), image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    Ok(format!(
        "data:image/png;base64,{}",
        STANDARD.encode(png_bytes)
    ))
}

/// Render all frames to `frames_dir`, then mux to `out_mp4`.
pub fn export_mp4(
    track: &Track,
    length: f64,
    duration: f64,
    fps: f64,
    frames_dir: &Path,
    out_mp4: &Path,
) -> Result<String, String> {
    if frames_dir.exists() {
        fs::remove_dir_all(frames_dir).map_err(|e| e.to_string())?;
    }
    let mut sink = PpmSink::with_view(frames_dir, EXPORT_SIZE, VIEW)
        .map_err(|e| e.to_string())?
        .with_background(SCENE_BACKGROUND);
    let frames = (duration * fps).ceil() as usize;
    for index in 0..frames {
        let t = index as f64 / fps;
        sink.frame(index, &eval_frame(track, t, length))
            .map_err(|e| e.to_string())?;
    }
    encode_ppm_dir(frames_dir, fps, out_mp4, None).map_err(|e| e.to_string())?;
    Ok(format!("{} frames → {}", frames, out_mp4.display()))
}

// El módulo de pruebas va al FINAL: clippy pide que nada quede
// después de un `#[cfg(test)]`, porque lo que viene detrás se lee como
// parte de las pruebas y no lo es.
#[cfg(test)]
mod render_tests {
    use super::*;
    use crate::sim::{SimParams, SimulationState};
    use motoreel::Prim2;

    #[test]
    fn rod_segment_spans_pivot_to_bob() {
        let state = SimulationState::new(SimParams::default()).expect("sim");
        let prims = eval_frame(&state.track, 0.0, state.params.length);
        let segment = prims
            .iter()
            .find_map(|p| match p {
                Prim2::Segment { a, b, style } => Some((a, b, style)),
                _ => None,
            })
            .expect("pendulum scene should emit a rod segment");
        let dist =
            ((segment.0.x - segment.1.x).powi(2) + (segment.0.y - segment.1.y).powi(2)).sqrt();
        assert!(dist > 0.5, "rod should span pivot to bob, got {dist}");
        assert!(
            segment.2.stroke.r > 200 && segment.2.stroke.g > 200,
            "rod stroke should use a light paper tone"
        );
        assert!(
            segment.2.width >= 0.02,
            "rod should be thick enough to rasterize"
        );
    }
}
