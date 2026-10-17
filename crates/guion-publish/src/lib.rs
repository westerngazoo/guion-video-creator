//! `guion-publish`: a post's text, exported with the platform's rules checked (R-0010).
//!
//! Publishing stays a person's decision; this crate only prepares what they
//! paste, and refuses what the platform would refuse, before anything is
//! written. Today: Instagram ([`instagram`]).

pub mod error;
pub mod instagram;
pub mod py_unicode;
pub mod text;

pub use error::PublishError;
pub use instagram::{export, export_file, Export, MAX_CAPTION_CHARS, MAX_HASHTAGS};
