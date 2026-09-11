//! The format boundary: bytes ⇄ [`Screenplay`] (SPEC-0001 §"Loading").
//!
//! All syntax knowledge lives here so `model.rs` never learns which file format
//! it came from. M1 speaks TOML; RON/JSON slot in later by adding a [`Syntax`]
//! arm without touching the typed model (R-0001 §4, decision log).
//!
//! Loading is deterministic and side-effect free: the only I/O is reading the
//! path handed in — no env, no network (R-0001 AC8).

use std::path::Path;

use crate::error::LoadError;
use crate::model::Screenplay;

/// The concrete on-disk syntaxes the boundary can speak. M1: TOML only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Syntax {
    Toml,
}

/// Read and parse a screenplay from disk, choosing the syntax by extension.
///
/// The extension is checked *before* touching the disk, so an unsupported
/// format fails the same way whether or not the file exists.
pub fn load(path: &Path) -> Result<Screenplay, LoadError> {
    let syntax = syntax_for(path)?;
    let src = std::fs::read_to_string(path).map_err(|e| LoadError::Io {
        path: path.display().to_string(),
        msg: e.to_string(),
    })?;
    parse(&src, syntax, &path.display().to_string())
}

/// Parse a screenplay from an in-memory string in the given syntax.
pub fn from_str(src: &str, syntax: Syntax) -> Result<Screenplay, LoadError> {
    parse(src, syntax, "<str>")
}

/// Serialize a screenplay back out — the inverse of [`from_str`], used by the
/// round-trip guarantee (R-0001 AC1).
pub fn to_string(sp: &Screenplay, syntax: Syntax) -> Result<String, LoadError> {
    match syntax {
        Syntax::Toml => toml::to_string(sp).map_err(|e| LoadError::Emit { msg: e.to_string() }),
    }
}

fn parse(src: &str, syntax: Syntax, path: &str) -> Result<Screenplay, LoadError> {
    match syntax {
        Syntax::Toml => toml::from_str::<Screenplay>(src).map_err(|e| LoadError::Parse {
            path: path.to_string(),
            msg: e.to_string(),
        }),
    }
}

fn syntax_for(path: &Path) -> Result<Syntax, LoadError> {
    match path.extension().and_then(|e| e.to_str()) {
        Some("toml") => Ok(Syntax::Toml),
        other => Err(LoadError::UnknownFormat {
            path: path.display().to_string(),
            ext: other.unwrap_or("").to_string(),
        }),
    }
}
