//! `guion-core` — the screenplay schema, loader and validator (R-0001).
//!
//! A *screenplay* is one declarative file that fully describes a physics
//! animation — metadata, camera, objects and shapes, labels, and a motion
//! timeline — with no Rust and no imperative drawing (RFC-0001 §4). This crate
//! is the creator-facing contract every other guion crate consumes: it loads a
//! screenplay into a typed [`Screenplay`] and validates it, turning every
//! problem into a typed, human-readable error that names the offending field.
//!
//! ```no_run
//! use guion_core::load_and_check;
//! use std::path::Path;
//!
//! let screenplay = load_and_check(Path::new("biceps.screenplay.toml"))?;
//! println!("{} @ {} fps", screenplay.meta.title, screenplay.meta.fps);
//! # Ok::<(), guion_core::GuionError>(())
//! ```
//!
//! The pipeline has two phases behind a format boundary (SPEC-0001 §2):
//! [`load`] parses bytes → model (all structural/grammar errors), then
//! [`validate`] runs the semantic checks deserialization cannot. [`check`] is
//! an alias for [`validate`]; [`load_and_check`] chains both.

pub mod bind;
pub mod dialecto;
pub mod error;
pub mod load;
pub mod model;
pub mod validate;

pub use bind::{SymbolTable, Token};
pub use dialecto::{dialecto, Dialecto, NoSeSabe};
pub use error::{GuionError, LoadError, ValidateError};
pub use load::{from_str, load, to_string, Syntax};
pub use model::{
    Anchor, AnchorAt, Audio, Camera, CameraKind, EaseName, Footer, Format, Hold, Hook, LabelSpec,
    Meta, ModelRef, Motion, Narration, ObjectSpec, Pose, PoseAnchor, Scale, Screenplay, Shape,
    Span, Style,
};
pub use validate::validate;

use std::path::Path;

/// Validate an already-loaded screenplay (SPEC-0001's `check()` entry).
pub fn check(sp: &Screenplay) -> Result<(), ValidateError> {
    validate(sp)
}

/// Load a screenplay from disk and validate it — the one-call happy path
/// (SPEC-0001 §3).
pub fn load_and_check(path: &Path) -> Result<Screenplay, GuionError> {
    let sp = load(path)?;
    validate(&sp)?;
    Ok(sp)
}
