use std::fmt;

#[derive(Debug)]
pub enum EncodeError {
    Io(std::io::Error),
    Ffmpeg {
        status: i32,
        cmd: String,
    },
    NoFfmpeg,
    InvalidInput(String),
    /// El render del cuadro `frame` falló (R-0005 AC7): el error dice cuál.
    Render {
        frame: usize,
        message: String,
    },
    /// La mezcla no llegó a la meta de sonoridad en las vueltas permitidas.
    Loudness {
        reached: crate::Loudness,
        target: crate::LoudnessTarget,
        rounds: usize,
    },
}

impl fmt::Display for EncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EncodeError::Io(e) => write!(f, "{e}"),
            EncodeError::Ffmpeg { status, cmd } => {
                write!(f, "ffmpeg falló (status {status}): {cmd}")
            }
            EncodeError::NoFfmpeg => {
                write!(f, "ffmpeg no está en PATH (o pon su ruta en GUION_FFMPEG)")
            }
            EncodeError::InvalidInput(msg) => write!(f, "{msg}"),
            EncodeError::Render { frame, message } => {
                write!(f, "el cuadro {frame} no se pudo calcular: {message}")
            }
            EncodeError::Loudness {
                reached,
                target,
                rounds,
            } => write!(
                f,
                "la mezcla no llegó a {} LUFS ± {} con pico ≤ {} dBTP en {rounds} vueltas: \
                 quedó en {:.1} LUFS, {:.1} dBTP",
                target.integrated_lufs,
                target.tolerance_lu,
                target.max_true_peak_dbtp,
                reached.integrated_lufs,
                reached.true_peak_dbtp
            ),
        }
    }
}

impl std::error::Error for EncodeError {}

impl From<std::io::Error> for EncodeError {
    fn from(e: std::io::Error) -> Self {
        EncodeError::Io(e)
    }
}
