//! `guion-audio` — narration synthesis, beds, and WAV mixing.
//!
//! The default [`engine::ScaffoldEngine`] produces timed tone placeholders per
//! cue so the encode pipeline can be tested without Piper. Swap in
//! [`engine::PiperEngine`] when `piper` is on PATH (or `GUION_PIPER`).
//!
//! Two ways in:
//!
//! - **From a screenplay** (feature `screenplay`, on by default): cues come
//!   from `[[narration]]` with fixed windows, and speech is fitted to them.
//! - **From a cue list** (R-0007, [`narrate_lines`]): the voice sets the
//!   clock. Lines are spoken one after another with fixed pauses, and the
//!   result reports when each line, and each sentence inside it, starts and
//!   ends, measured on the samples. No screenplay needed.
//!
//! And one way out for a reel's sound (R-0008, [`synth`]): generic,
//! deterministic synthesis (oscillators, envelopes, seeded noise, filters, a
//! level-stable room, pan, buses, ducking under the voice) to a 16-bit stereo
//! WAV. A studio keeps its own instruments and builds them from these.

mod engine;
mod error;
mod lines;
mod pcm;
mod wav;

pub mod synth;

#[cfg(feature = "screenplay")]
mod bed;
#[cfg(feature = "screenplay")]
mod narrate;
#[cfg(feature = "screenplay")]
mod script;

pub use engine::{engine_by_name, NarrationEngine, PiperEngine, ScaffoldEngine};
pub use error::AudioError;
pub use lines::{
    narrate_lines, sentence_times, split_sentences, Line, Narration, Pacing, Sentence, SilenceRule,
    TimedLine,
};
pub use pcm::{read_wav_pcm16, write_wav, write_wav_pcm16, Pcm16};
pub use wav::{mix_buffers, WavBuffer, SAMPLE_RATE};

#[cfg(feature = "screenplay")]
pub use bed::{bed_for_screenplay, generate_bed, timeline_duration};
#[cfg(feature = "screenplay")]
pub use narrate::{
    default_narration_path, fit_to_window, mix_for_encode, narrate_screenplay, resolve_audio_path,
    resolve_narration_script, resolve_relative, write_narration, NarrateOptions,
};
#[cfg(feature = "screenplay")]
pub use script::{collect_cues, parse_script, Cue};
