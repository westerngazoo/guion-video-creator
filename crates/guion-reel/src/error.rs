//! The studio's typed errors (CLAUDE.md §6: typed, located, never a panic on input).

use std::fmt;

use guion_encode::EncodeError;

/// Everything that can go wrong in the studio, each naming what and where.
#[derive(Debug)]
pub enum ReelError {
    Io(std::io::Error),
    /// The encoder (or ffmpeg) failed (R-0005).
    Encode(EncodeError),
    /// A clock entry that cannot go into `events.json` (AC1).
    InvalidClock {
        name: String,
        reason: String,
    },
    /// A narration timeline that does not parse or does not hold together (AC5).
    Narration {
        file: String,
        reason: String,
    },
    /// The scene asked for a line the narration does not have (AC5).
    NoSuchLine {
        id: String,
    },
    /// The scene asked for a phrase its line does not say (AC5).
    NoSuchPhrase {
        id: String,
        phrase: String,
        text: String,
    },
    /// A sentence index past the end of its line (AC5).
    NoSuchSentence {
        id: String,
        index: usize,
        count: usize,
    },
    /// A subtitle that does not fit, or two subtitles on screen at once (AC6).
    Subtitle(String),
    /// A command-line argument the studio does not understand (AC3).
    Usage(String),
    /// What a format asks for that this reel cannot give (R-0016): a
    /// thumbnail from a canvas that is not 16:9, or one over the platform's
    /// weight.
    Format(String),
}

impl fmt::Display for ReelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReelError::Io(e) => write!(f, "{e}"),
            ReelError::Encode(e) => write!(f, "{e}"),
            ReelError::InvalidClock { name, reason } => write!(f, "clock entry «{name}»: {reason}"),
            ReelError::Narration { file, reason } => write!(f, "{file}: {reason}"),
            ReelError::NoSuchLine { id } => write!(f, "the narration has no line «{id}»"),
            ReelError::NoSuchPhrase { id, phrase, text } => {
                write!(f, "line «{id}» does not say «{phrase}»: {text}")
            }
            ReelError::NoSuchSentence { id, index, count } => {
                write!(
                    f,
                    "line «{id}» has {count} sentences; asked for index {index}"
                )
            }
            ReelError::Subtitle(msg) => write!(f, "{msg}"),
            ReelError::Usage(msg) => write!(f, "{msg}"),
            ReelError::Format(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for ReelError {}

impl From<std::io::Error> for ReelError {
    fn from(e: std::io::Error) -> Self {
        ReelError::Io(e)
    }
}

impl From<EncodeError> for ReelError {
    fn from(e: EncodeError) -> Self {
        ReelError::Encode(e)
    }
}

impl From<ReelError> for std::io::Error {
    fn from(e: ReelError) -> Self {
        match e {
            ReelError::Io(e) => e,
            other => std::io::Error::other(other.to_string()),
        }
    }
}
