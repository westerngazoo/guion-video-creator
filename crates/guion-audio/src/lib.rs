//! `guion-audio` — narration synthesis, beds, and WAV mixing.
//!
//! The default [`engine::ScaffoldEngine`] produces timed tone placeholders per
//! cue so the encode pipeline can be tested without Piper. Swap in
//! [`engine::PiperEngine`] when `piper` is on PATH.

mod bed;
mod engine;
mod error;
mod narrate;
mod script;
mod wav;

pub use bed::{bed_for_screenplay, generate_bed, timeline_duration};
pub use engine::{engine_by_name, NarrationEngine, PiperEngine, ScaffoldEngine};
pub use error::AudioError;
pub use narrate::{
    default_narration_path, fit_to_window, mix_for_encode, narrate_screenplay, resolve_audio_path,
    resolve_narration_script, resolve_relative, write_narration, NarrateOptions,
};
pub use script::{collect_cues, parse_script, Cue};
pub use wav::{mix_buffers, WavBuffer, SAMPLE_RATE};
