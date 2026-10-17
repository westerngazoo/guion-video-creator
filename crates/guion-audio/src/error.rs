use std::fmt;

#[derive(Debug)]
pub enum AudioError {
    Io(std::io::Error),
    NoCues,
    Engine(String),
    PiperMissing,
    InvalidScript(String),
    /// A cue-list line that cannot be spoken as given (R-0007 AC6): an empty
    /// text, a negative or non-finite pause. Names the line and the field.
    InvalidLine {
        line: String,
        message: String,
    },
    /// A line has more sentences than measured silences between them, so the
    /// sentence times cannot be placed (R-0007 AC6). Nothing is written.
    SentenceTiming {
        line: String,
        sentences: usize,
        gaps: usize,
        min_gap: f64,
    },
    /// A WAV the engine or ffmpeg wrote that this crate cannot read.
    Wav(String),
    /// ffmpeg was not found or failed while resampling.
    Resample(String),
    /// A synthesis parameter out of range (R-0008 AC12): names the
    /// parameter (`room.t60`, `duck.ramp`) and what it must be.
    InvalidSynth {
        what: String,
        message: String,
    },
    /// A score (R-0011) that cannot be read or played: the message names the
    /// file and the table (`sonido.toml: [[golpe]] 3 («qed»): …`).
    InvalidScore(String),
}

impl fmt::Display for AudioError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AudioError::Io(e) => write!(f, "io: {e}"),
            AudioError::NoCues => write!(f, "no hay cues de narración en el screenplay"),
            AudioError::Engine(msg) => write!(f, "motor TTS: {msg}"),
            AudioError::PiperMissing => write!(
                f,
                "piper no está en PATH ni en GUION_PIPER — instala piper o usa --engine scaffold"
            ),
            AudioError::InvalidScript(msg) => write!(f, "guion de narración inválido: {msg}"),
            AudioError::InvalidLine { line, message } => {
                write!(f, "línea «{line}»: {message}")
            }
            AudioError::SentenceTiming {
                line,
                sentences,
                gaps,
                min_gap,
            } => write!(
                f,
                "línea «{line}»: tiene {sentences} frases y solo {gaps} silencios de {min_gap} s entre ellas"
            ),
            AudioError::Wav(msg) => write!(f, "wav: {msg}"),
            AudioError::Resample(msg) => write!(f, "remuestreo (ffmpeg): {msg}"),
            AudioError::InvalidSynth { what, message } => write!(f, "síntesis, {what}: {message}"),
            AudioError::InvalidScore(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for AudioError {}

impl From<std::io::Error> for AudioError {
    fn from(e: std::io::Error) -> Self {
        AudioError::Io(e)
    }
}
