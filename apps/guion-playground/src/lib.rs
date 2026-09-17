//! Physics Playground — garust-physics simulation with motoreel preview/export.

mod render;
mod sim;

use std::path::PathBuf;
use std::sync::Mutex;

use serde::Serialize;
use sim::{SimParams, SimulationState, EXPORT_FPS};

pub use sim::{SimParams as PlaygroundParams, EXPORT_FPS as PLAYGROUND_FPS};

/// Serializable frame snapshot for the UI scrubber.
#[derive(Debug, Serialize)]
pub struct FrameState {
    pub t: f64,
    pub duration: f64,
    pub bob_x: f64,
    pub bob_y: f64,
    pub angle_deg: f64,
    pub length: f64,
    pub gravity: f64,
}

/// Current simulation metadata.
#[derive(Debug, Serialize)]
pub struct SimInfo {
    pub params: SimParamsDto,
    pub duration: f64,
    pub fps: f64,
}

#[derive(Debug, Serialize)]
pub struct SimParamsDto {
    pub length: f64,
    pub gravity: f64,
    pub angle_deg: f64,
}

impl From<SimParams> for SimParamsDto {
    fn from(p: SimParams) -> Self {
        SimParamsDto {
            length: p.length,
            gravity: p.gravity,
            angle_deg: p.angle_deg,
        }
    }
}

pub struct PlaygroundState(pub Mutex<SimulationState>);

impl PlaygroundState {
    pub fn new() -> Result<Self, String> {
        Ok(PlaygroundState(Mutex::new(SimulationState::new(
            SimParams::default(),
        )?)))
    }
}

pub fn reset_sim(state: &SimulationState) -> Result<SimulationState, String> {
    let mut next = SimulationState::new(state.params)?;
    next.duration = state.duration;
    next.bake(state.duration)?;
    Ok(next)
}

pub fn state_at(state: &SimulationState, t: f64) -> FrameState {
    let (bob_x, bob_y, _) = state.bob_position(t);
    let ell = state.params.length;
    let angle_deg = if ell > 0.0 {
        (bob_x / ell).asin().to_degrees()
    } else {
        0.0
    };
    FrameState {
        t: t.clamp(0.0, state.duration),
        duration: state.duration,
        bob_x,
        bob_y,
        angle_deg,
        length: state.params.length,
        gravity: state.params.gravity,
    }
}

pub fn preview_png(state: &SimulationState, t: f64) -> Result<String, String> {
    render::preview_png_base64(&state.track, t, state.params.length)
}

pub fn export_mp4(state: &SimulationState, out_mp4: Option<PathBuf>) -> Result<String, String> {
    let slug = "pendulum";
    let frames_dir = PathBuf::from("out").join(slug).join("frames");
    let mp4 = out_mp4.unwrap_or_else(|| PathBuf::from("out").join(slug).join("playground.mp4"));
    render::export_mp4(
        &state.track,
        state.params.length,
        state.duration,
        EXPORT_FPS,
        &frames_dir,
        &mp4,
    )
}

pub fn sim_info(state: &SimulationState) -> SimInfo {
    SimInfo {
        params: state.params.into(),
        duration: state.duration,
        fps: EXPORT_FPS,
    }
}

pub fn set_params(
    state: &mut SimulationState,
    length: f64,
    gravity: f64,
    angle_deg: f64,
) -> Result<(), String> {
    state.set_params(SimParams {
        length,
        gravity,
        angle_deg,
    })
}

pub fn simulate_to(state: &mut SimulationState, duration: f64) -> Result<(), String> {
    state.bake(duration)
}

pub fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

pub fn default_export_path() -> PathBuf {
    PathBuf::from("out").join("pendulum").join("playground.mp4")
}

#[cfg(test)]
mod tests {
    use base64::Engine as _;

    use super::*;

    #[test]
    fn pendulum_bakes_and_previews() {
        let state = SimulationState::new(SimParams::default()).expect("default sim");
        assert!(state.duration > 0.0);
        let png = preview_png(&state, 0.0).expect("preview frame");
        assert!(png.starts_with("data:image/png;base64,"));
        let b64 = png
            .strip_prefix("data:image/png;base64,")
            .expect("data url");
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(b64)
            .expect("base64 png");
        let img = image::load_from_memory(&bytes).expect("decode png");
        let rgba = img.to_rgba8();
        let cream = rgba
            .pixels()
            .filter(|p| p[0] > 200 && p[1] > 200 && p[2] > 200)
            .count();
        assert!(
            cream > 50,
            "preview frame should contain visible rod strokes (cream pixels), got {cream}"
        );
    }

    #[test]
    fn pendulum_exports_mp4_when_ffmpeg_present() {
        if std::process::Command::new("which")
            .arg("ffmpeg")
            .output()
            .map(|o| !o.status.success())
            .unwrap_or(true)
        {
            return;
        }
        let mut state = SimulationState::new(SimParams::default()).expect("default sim");
        state.bake(0.5).expect("short bake");
        let dir = std::env::temp_dir().join("guion-playground-smoke");
        let frames = dir.join("frames");
        let mp4 = dir.join("smoke.mp4");
        let msg = render::export_mp4(
            &state.track,
            state.params.length,
            state.duration,
            EXPORT_FPS,
            &frames,
            &mp4,
        )
        .expect("export mp4");
        assert!(mp4.is_file(), "{msg}");
    }
}
