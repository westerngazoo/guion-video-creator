//! Cuadros RGB en memoria → MP4, sin pasar por el disco (R-0005 AC4, AC7).
//!
//! El motor da cada cuadro como bytes RGB; escribirlos como PPM numerados y
//! volver a leerlos es trabajo y disco de sobra (un reel de 45 s a 1080×1920
//! son 8 GB de PPM). Aquí van directo a la entrada de ffmpeg.
//!
//! **Sin salida a medias** (CLAUDE.md §6): ffmpeg escribe a un archivo
//! temporal junto a la salida, y solo un [`Encoder::finish`] exitoso lo
//! renombra. Un error, o un `Encoder` que se suelta sin terminar, borra el
//! temporal.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};

use crate::ffmpeg::{check_fps, ffmpeg, fps_arg, VIDEO};
use crate::EncodeError;

/// Tamaño y ritmo del video que se va a escribir.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VideoSpec {
    pub width: u32,
    pub height: u32,
    pub fps: f64,
}

impl VideoSpec {
    /// Bytes de un cuadro RGB24.
    pub fn frame_len(&self) -> usize {
        self.width as usize * self.height as usize * 3
    }

    fn check(&self) -> Result<(), EncodeError> {
        check_fps(self.fps)?;
        // yuv420p guarda el color a la mitad en cada eje: lados pares
        if self.width == 0
            || self.height == 0
            || !self.width.is_multiple_of(2)
            || !self.height.is_multiple_of(2)
        {
            return Err(EncodeError::InvalidInput(format!(
                "el video debe tener lados pares y no nulos (yuv420p), no {}×{}",
                self.width, self.height
            )));
        }
        Ok(())
    }
}

/// Un MP4 que se va escribiendo cuadro por cuadro.
pub struct Encoder {
    child: Option<Child>,
    stdin: Option<ChildStdin>,
    spec: VideoSpec,
    frames: usize,
    tmp: PathBuf,
    out: PathBuf,
    cmd: String,
}

impl Encoder {
    /// Arranca ffmpeg para escribir `out`.
    pub fn start(out: &Path, spec: VideoSpec) -> Result<Encoder, EncodeError> {
        spec.check()?;
        if let Some(parent) = out.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent)?;
        }
        let tmp = temporal(out);
        let mut cmd = Command::new(ffmpeg()?);
        cmd.args(["-y", "-nostdin", "-hide_banner", "-loglevel", "error"])
            .args(["-f", "rawvideo", "-pix_fmt", "rgb24", "-s"])
            .arg(format!("{}x{}", spec.width, spec.height))
            .args(["-framerate", &fps_arg(spec.fps), "-i", "-"])
            .args(VIDEO)
            .args(["-f", "mp4"])
            .arg(&tmp)
            .stdin(Stdio::piped());
        let display = format!("{cmd:?}");
        let mut child = cmd.spawn()?;
        let stdin = child.stdin.take();
        Ok(Encoder {
            child: Some(child),
            stdin,
            spec,
            frames: 0,
            tmp,
            out: out.to_path_buf(),
            cmd: display,
        })
    }

    /// Agrega un cuadro: exactamente ancho × alto × 3 bytes, RGB, fila por fila.
    pub fn push(&mut self, rgb: &[u8]) -> Result<(), EncodeError> {
        if rgb.len() != self.spec.frame_len() {
            return Err(EncodeError::InvalidInput(format!(
                "el cuadro {} trae {} bytes; un cuadro RGB de {}×{} lleva {}",
                self.frames,
                rgb.len(),
                self.spec.width,
                self.spec.height,
                self.spec.frame_len()
            )));
        }
        let stdin = self.stdin.as_mut().expect("abierto hasta finish");
        if let Err(e) = stdin.write_all(rgb) {
            // ffmpeg se fue: lo que dijo al salir es más útil que el EPIPE
            return Err(self.fallo().unwrap_or(EncodeError::Io(e)));
        }
        self.frames += 1;
        Ok(())
    }

    /// Cuadros escritos hasta ahora.
    pub fn frames(&self) -> usize {
        self.frames
    }

    /// Cierra la entrada, espera a ffmpeg y deja el MP4 en su lugar.
    /// Devuelve los cuadros escritos.
    pub fn finish(mut self) -> Result<usize, EncodeError> {
        if self.frames == 0 {
            return Err(EncodeError::InvalidInput(
                "un video sin cuadros no es un video".into(),
            ));
        }
        drop(self.stdin.take());
        let status = self.child.take().expect("vivo hasta finish").wait()?;
        if !status.success() {
            return Err(EncodeError::Ffmpeg {
                status: status.code().unwrap_or(-1),
                cmd: self.cmd.clone(),
            });
        }
        std::fs::rename(&self.tmp, &self.out)?;
        Ok(self.frames)
    }

    fn fallo(&mut self) -> Option<EncodeError> {
        drop(self.stdin.take());
        let status = self.child.take()?.wait().ok()?;
        Some(EncodeError::Ffmpeg {
            status: status.code().unwrap_or(-1),
            cmd: self.cmd.clone(),
        })
    }
}

impl Drop for Encoder {
    /// Un `Encoder` que no terminó no deja nada: ni proceso ni archivo.
    fn drop(&mut self) {
        drop(self.stdin.take());
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        let _ = std::fs::remove_file(&self.tmp);
    }
}

/// `dir/.<nombre>.parcial.mp4`: junto a la salida, para que el rename no
/// cruce de disco.
pub(crate) fn temporal(out: &Path) -> PathBuf {
    let nombre = out
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "video".into());
    out.with_file_name(format!(".{nombre}.parcial.mp4"))
}
