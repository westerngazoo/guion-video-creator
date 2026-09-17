//! `guion-encode` — numbered PPM frames → vertical `.mp4` (M2).

mod error;

pub use error::EncodeError;

use std::path::{Path, PathBuf};
use std::process::Command;

use guion_core::Format;

/// Vertical reel preset metadata.
#[derive(Debug, Clone, Copy)]
pub struct VerticalPreset {
    pub width: u32,
    pub height: u32,
    pub safe_top_px: u32,
    pub safe_bottom_px: u32,
}

impl VerticalPreset {
    pub const REEL: VerticalPreset = VerticalPreset {
        width: 1080,
        height: 1920,
        safe_top_px: 120,
        safe_bottom_px: 200,
    };
}

pub fn preset_for(format: Format) -> VerticalPreset {
    match format {
        Format::Vertical => VerticalPreset::REEL,
    }
}

/// Encode `frame_%05d.ppm` in `frames_dir` to `out_mp4` at `fps`.
pub fn encode_ppm_dir(
    frames_dir: &Path,
    fps: f64,
    out_mp4: &Path,
    audio: Option<&Path>,
) -> Result<(), EncodeError> {
    if !(fps.is_finite() && fps > 0.0) {
        return Err(EncodeError::InvalidInput("fps debe ser finito y > 0".into()));
    }
    let ffmpeg = which_ffmpeg()?;
    let pattern = frames_dir.join("frame_%05d.ppm");
    if !frames_dir.join("frame_00000.ppm").exists() {
        return Err(EncodeError::InvalidInput(format!(
            "no hay frames en {}",
            frames_dir.display()
        )));
    }

    let mut cmd = Command::new(&ffmpeg);
    cmd.args(["-y", "-nostdin", "-framerate"])
        .arg(format_fps(fps))
        .args(["-i"])
        .arg(&pattern);

    if let Some(wav) = audio {
        if wav.exists() {
            cmd.args(["-i"]).arg(wav);
            cmd.args([
                "-c:v",
                "libx264",
                "-pix_fmt",
                "yuv420p",
                "-crf",
                "18",
                "-c:a",
                "aac",
                "-b:a",
                "160k",
                "-shortest",
                "-movflags",
                "+faststart",
            ]);
        } else {
            cmd.args(["-c:v", "libx264", "-pix_fmt", "yuv420p", "-crf", "18"]);
        }
    } else {
        cmd.args(["-c:v", "libx264", "-pix_fmt", "yuv420p", "-crf", "18"]);
    }

    if let Some(parent) = out_mp4.parent() {
        std::fs::create_dir_all(parent)?;
    }
    cmd.arg(out_mp4);

    let display = format!("{cmd:?}");
    let status = cmd.status()?;
    if !status.success() {
        return Err(EncodeError::Ffmpeg {
            status: status.code().unwrap_or(-1),
            cmd: display,
        });
    }
    Ok(())
}

/// Default output mp4 beside frames: `out/<slug>/reel.mp4`.
pub fn default_mp4(slug: &str) -> PathBuf {
    PathBuf::from("out").join(slug).join("reel.mp4")
}

fn which_ffmpeg() -> Result<String, EncodeError> {
    let out = Command::new("which")
        .arg("ffmpeg")
        .output()
        .map_err(EncodeError::Io)?;
    if !out.status.success() {
        return Err(EncodeError::NoFfmpeg);
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn format_fps(fps: f64) -> String {
    if (fps - fps.round()).abs() < 1e-9 {
        format!("{}", fps as i64)
    } else {
        format!("{fps}")
    }
}
