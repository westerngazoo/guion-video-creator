//! The MP4, decoded back to RGB frames by ffmpeg (R-0012 AC4, AC5).
//!
//! The reel was encoded with BT.709 in TV range (R-0005); it is decoded with
//! the same matrix and range, so a token comes back within a few levels of
//! its value.

use std::io::Read;
use std::path::Path;
use std::process::{Child, ChildStdout, Command, Stdio};

use super::GateError;

/// A running decode: one RGB24 frame at a time, in order.
pub struct Frames {
    child: Child,
    out: ChildStdout,
    pub width: usize,
    pub height: usize,
}

impl Frames {
    pub fn open(mp4: &Path, width: usize, height: usize) -> Result<Frames, GateError> {
        let ff = guion_encode::ffmpeg::ffmpeg().map_err(|e| GateError::Input(e.to_string()))?;
        let mut child = Command::new(&ff)
            .args(["-v", "error", "-nostdin", "-i"])
            .arg(mp4)
            .args([
                "-map",
                "0:v:0",
                "-vf",
                "scale=in_color_matrix=bt709:in_range=tv:out_range=full:flags=bilinear,format=rgb24",
                "-f",
                "rawvideo",
                "-",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| GateError::Input(format!("{ff}: {e}")))?;
        let out = child.stdout.take().expect("stdout is piped");
        Ok(Frames {
            child,
            out,
            width,
            height,
        })
    }

    /// The next frame into `buf` (resized to w × h × 3); `false` at the end.
    pub fn next_into(&mut self, buf: &mut Vec<u8>) -> Result<bool, GateError> {
        buf.resize(self.width * self.height * 3, 0);
        let mut got = 0;
        while got < buf.len() {
            match self.out.read(&mut buf[got..]) {
                Ok(0) => break,
                Ok(n) => got += n,
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
                Err(e) => return Err(GateError::Input(e.to_string())),
            }
        }
        if got == 0 {
            return Ok(false);
        }
        if got < buf.len() {
            return Err(GateError::Input(format!(
                "a partial frame ({got} of {} bytes): is the canvas {}×{}?",
                buf.len(),
                self.width,
                self.height
            )));
        }
        Ok(true)
    }

    pub fn finish(mut self) -> Result<(), GateError> {
        let st = self
            .child
            .wait()
            .map_err(|e| GateError::Input(e.to_string()))?;
        if st.success() {
            Ok(())
        } else {
            Err(GateError::Input("ffmpeg could not decode the video".into()))
        }
    }
}

/// The video stream's width and height, from ffmpeg's banner.
pub fn size(mp4: &Path) -> Result<(usize, usize), GateError> {
    let ff = guion_encode::ffmpeg::ffmpeg().map_err(|e| GateError::Input(e.to_string()))?;
    let out = Command::new(&ff)
        .args(["-hide_banner", "-nostdin", "-i"])
        .arg(mp4)
        .output()
        .map_err(|e| GateError::Input(format!("{ff}: {e}")))?;
    let log = String::from_utf8_lossy(&out.stderr);
    let line = log
        .lines()
        .find(|l| l.contains("Video:"))
        .ok_or_else(|| GateError::Input(format!("{}: no video stream", mp4.display())))?;
    // "…, yuv420p(tv, bt709, progressive), 1080x1920, …"
    for tok in line.split([',', ' ']) {
        if let Some((a, b)) = tok.split_once('x') {
            if let (Ok(w), Ok(h)) = (a.parse::<usize>(), b.parse::<usize>()) {
                return Ok((w, h));
            }
        }
    }
    Err(GateError::Input(format!(
        "{}: no frame size in «{line}»",
        mp4.display()
    )))
}

/// The audio track, decoded to mono f32 at `rate`.
pub fn audio_mono(mp4: &Path, rate: u32) -> Result<Vec<f32>, GateError> {
    let ff = guion_encode::ffmpeg::ffmpeg().map_err(|e| GateError::Input(e.to_string()))?;
    let out = Command::new(&ff)
        .args(["-v", "error", "-nostdin", "-i"])
        .arg(mp4)
        .args([
            "-map",
            "0:a:0",
            "-ac",
            "1",
            "-ar",
            &rate.to_string(),
            "-f",
            "f32le",
            "-",
        ])
        .output()
        .map_err(|e| GateError::Input(format!("{ff}: {e}")))?;
    if !out.status.success() {
        return Err(GateError::Input(format!(
            "{}: no audio track",
            mp4.display()
        )));
    }
    Ok(out
        .stdout
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| f32::from_le_bytes(*b))
        .collect())
}
