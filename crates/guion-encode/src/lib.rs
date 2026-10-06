//! `guion-encode` — cuadros → `.mp4` vertical, listo para el teléfono (M2, R-0005).
//!
//! Tres entradas, una sola salida de video ([`ffmpeg::VIDEO`]: BT.709
//! etiquetado y `+faststart`):
//!
//! - [`encode_ppm_dir`]: PPM numerados en disco, con audio opcional;
//! - [`Encoder`]: cuadros RGB en memoria, uno por uno;
//! - [`encode_ordered`]: cuadros calculados en varios hilos, escritos en orden.
//!
//! Y el audio al nivel de las plataformas: [`mux_at_loudness`] (−14 LUFS,
//! −1 dBTP) y [`measure`].

mod error;
pub mod ffmpeg;
mod loudness;
mod ordered;
mod stream;

pub use error::EncodeError;
pub use loudness::{measure, mux_at_loudness, Loudness, LoudnessTarget};
pub use ordered::encode_ordered;
pub use stream::{Encoder, VideoSpec};

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
///
/// El video sale con las banderas de [`ffmpeg::VIDEO`] (BT.709 etiquetado y
/// `+faststart`, R-0005 AC1–AC3), con o sin audio.
pub fn encode_ppm_dir(
    frames_dir: &Path,
    fps: f64,
    out_mp4: &Path,
    audio: Option<&Path>,
) -> Result<(), EncodeError> {
    ffmpeg::check_fps(fps)?;
    let ff = ffmpeg::ffmpeg()?;
    let pattern = frames_dir.join("frame_%05d.ppm");
    if !frames_dir.join("frame_00000.ppm").exists() {
        return Err(EncodeError::InvalidInput(format!(
            "no hay frames en {}",
            frames_dir.display()
        )));
    }

    let mut cmd = Command::new(&ff);
    cmd.args(["-y", "-nostdin", "-framerate"])
        .arg(ffmpeg::fps_arg(fps))
        .args(["-i"])
        .arg(&pattern);
    let audio = audio.filter(|wav| wav.exists());
    if let Some(wav) = audio {
        cmd.args(["-i"]).arg(wav);
    }
    cmd.args(ffmpeg::VIDEO);
    if audio.is_some() {
        cmd.args(["-c:a", "aac", "-b:a", "160k", "-shortest"]);
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
