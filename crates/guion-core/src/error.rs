//! Typed errors for loading and validating a screenplay.
//!
//! Two families, mirroring the two phases (SPEC-0001 §2 "Errors"):
//!
//! * [`LoadError`] — everything the *format boundary* catches: I/O, a syntax
//!   error, a missing required field, an unknown field (`deny_unknown_fields`),
//!   an unknown enum `kind`, a malformed binding token. These are produced by
//!   `serde`/`toml` during deserialization and always name the offending
//!   field/key/kind in `msg` (R-0001 AC2, AC3, AC5).
//! * [`ValidateError`] — the semantic checks "deserialization cannot" do
//!   (SPEC-0001 §2): numeric ranges, non-empty tags, and cross-reference
//!   resolution (R-0001 AC4, AC6, AC7).
//!
//! No `thiserror`: R-0001 §4 restricts deps to `std` + serde-family, so the
//! `Display`/`Error` impls are hand-written.

use std::fmt;

/// A problem reading or parsing the screenplay bytes into the typed model.
#[derive(Debug)]
pub enum LoadError {
    /// The file could not be read from disk.
    Io { path: String, msg: String },
    /// Deserialization failed. `msg` carries the serde/toml diagnostic, which
    /// names the field, the unexpected key, or the unknown `kind` and its
    /// valid alternatives (AC2/AC3/AC5).
    Parse { path: String, msg: String },
    /// The file extension maps to no known syntax (M1 knows only `.toml`).
    UnknownFormat { path: String, ext: String },
    /// Serializing a screenplay back out failed (used by the round-trip path).
    Emit { msg: String },
}

/// A semantic problem in an otherwise well-formed screenplay.
#[derive(Debug, Clone, PartialEq)]
pub enum ValidateError {
    /// A number falls outside its allowed range (`fps > 0`, `hold ∈ [0,1]`).
    OutOfRange {
        field: String,
        value: f64,
        bound: &'static str,
    },
    /// A tag that must carry content is empty (`meta.lang`).
    Empty { field: String },
    /// A binding token (or bare id reference) points at an id that no
    /// `model`/`object` declares. `at` is where it was used (AC6).
    DanglingRef { token: String, at: String },
}

/// Either phase's failure, for the combined [`crate::load_and_check`] entry.
#[derive(Debug)]
pub enum GuionError {
    Load(LoadError),
    Validate(ValidateError),
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoadError::Io { path, msg } => {
                write!(f, "no se pudo leer {path}: {msg}")
            }
            LoadError::Parse { path, msg } => {
                write!(f, "error al parsear {path}: {msg}")
            }
            LoadError::UnknownFormat { path, ext } => write!(
                f,
                "formato desconocido para {path}: extensión {ext:?} \
                 (M1 sólo entiende .toml)"
            ),
            LoadError::Emit { msg } => {
                write!(f, "error al serializar el screenplay: {msg}")
            }
        }
    }
}

impl fmt::Display for ValidateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidateError::OutOfRange {
                field,
                value,
                bound,
            } => write!(
                f,
                "campo {field} fuera de rango: {value} (se exige {bound})"
            ),
            ValidateError::Empty { field } => {
                write!(f, "campo {field} no puede ir vacío")
            }
            ValidateError::DanglingRef { token, at } => write!(
                f,
                "referencia colgante {token:?} en {at}: ningún model/object \
                 declara ese id"
            ),
        }
    }
}

impl fmt::Display for GuionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GuionError::Load(e) => write!(f, "{e}"),
            GuionError::Validate(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for LoadError {}
impl std::error::Error for ValidateError {}
impl std::error::Error for GuionError {}

impl From<LoadError> for GuionError {
    fn from(e: LoadError) -> Self {
        GuionError::Load(e)
    }
}

impl From<ValidateError> for GuionError {
    fn from(e: ValidateError) -> Self {
        GuionError::Validate(e)
    }
}
