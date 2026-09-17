use std::fmt;

/// Assembly failed — names the object/field, never panics on valid input.
#[derive(Debug, Clone, PartialEq)]
pub enum AssembleError {
    Unsupported { what: String },
    Unresolved { token: String, at: String },
    UnknownObject { id: String, at: String },
    Track(motoreel::TrackError),
}

impl fmt::Display for AssembleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AssembleError::Unsupported { what } => write!(f, "no soportado: {what}"),
            AssembleError::Unresolved { token, at } => {
                write!(f, "no se resolvió {token} en {at}")
            }
            AssembleError::UnknownObject { id, at } => {
                write!(f, "objeto desconocido `{id}` en {at}")
            }
            AssembleError::Track(e) => write!(f, "track: {e}"),
        }
    }
}

impl std::error::Error for AssembleError {}

impl From<motoreel::TrackError> for AssembleError {
    fn from(e: motoreel::TrackError) -> Self {
        AssembleError::Track(e)
    }
}
