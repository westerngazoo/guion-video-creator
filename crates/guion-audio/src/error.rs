use std::fmt;

#[derive(Debug)]
pub enum AudioError {
    Io(std::io::Error),
    NoCues,
    Engine(String),
    PiperMissing,
    InvalidScript(String),
}

impl fmt::Display for AudioError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AudioError::Io(e) => write!(f, "io: {e}"),
            AudioError::NoCues => write!(f, "no hay cues de narración en el screenplay"),
            AudioError::Engine(msg) => write!(f, "motor TTS: {msg}"),
            AudioError::PiperMissing => write!(
                f,
                "piper no está en PATH — instala piper-tts o usa --engine scaffold"
            ),
            AudioError::InvalidScript(msg) => write!(f, "guion de narración inválido: {msg}"),
        }
    }
}

impl std::error::Error for AudioError {}

impl From<std::io::Error> for AudioError {
    fn from(e: std::io::Error) -> Self {
        AudioError::Io(e)
    }
}
