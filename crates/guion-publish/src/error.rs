//! Typed, located errors (CLAUDE.md §6). An export that fails writes nothing.

use std::fmt;

#[derive(Debug)]
pub enum PublishError {
    Io {
        path: String,
        source: std::io::Error,
    },
    /// The file is not UTF-8; `at` is the byte offset of the first bad byte.
    NotUtf8 { path: String, at: usize },
    /// A required `## <name>` section is missing.
    MissingSection { path: String, section: String },
    /// More hashtags than the platform accepts.
    TooManyHashtags {
        path: String,
        count: usize,
        max: usize,
    },
    /// The caption as pasted (caption and hashtags) is longer than the platform accepts.
    CaptionTooLong {
        path: String,
        chars: usize,
        max: usize,
    },
}

impl fmt::Display for PublishError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PublishError::Io { path, source } => write!(f, "{path}: {source}"),
            PublishError::NotUtf8 { path, at } => write!(f, "{path}: not UTF-8 (byte {at})"),
            PublishError::MissingSection { path, section } => {
                write!(f, "{path}: the section «## {section}» is missing")
            }
            PublishError::TooManyHashtags { path, count, max } => write!(
                f,
                "{path}: {count} hashtags; Instagram accepts {max} at most"
            ),
            PublishError::CaptionTooLong { path, chars, max } => {
                write!(f, "{path}: the exported caption is {chars} characters; Instagram accepts {max} at most")
            }
        }
    }
}

impl std::error::Error for PublishError {}
