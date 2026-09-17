use std::fmt;

#[derive(Debug)]
pub enum EncodeError {
    Io(std::io::Error),
    Ffmpeg { status: i32, cmd: String },
    NoFfmpeg,
    InvalidInput(String),
}

impl fmt::Display for EncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EncodeError::Io(e) => write!(f, "{e}"),
            EncodeError::Ffmpeg { status, cmd } => {
                write!(f, "ffmpeg falló (status {status}): {cmd}")
            }
            EncodeError::NoFfmpeg => write!(f, "ffmpeg no está en PATH"),
            EncodeError::InvalidInput(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for EncodeError {}

impl From<std::io::Error> for EncodeError {
    fn from(e: std::io::Error) -> Self {
        EncodeError::Io(e)
    }
}
