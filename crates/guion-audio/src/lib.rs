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
//! - **From takes** (R-0019, [`narrate_utterances`]): the natural voice.
//!   Blocks of lines are spoken as one utterance each with prosody noise
//!   ([`Prosody::NATURAL`]); each utterance is spoken once and kept
//!   ([`TakeStore`]), and its lines and sentences are cut where the voice
//!   wrote exact silence, so the same takes give the same bytes.
//!
//! And one way out for a reel's sound (R-0008, [`synth`]): generic,
//! deterministic synthesis (oscillators, envelopes, seeded noise, filters, a
//! level-stable room, pan, buses, ducking under the voice) to a 16-bit stereo
//! WAV. A studio keeps its own instruments and builds them from these.
//!
//! And the sound design as data (R-0011, feature `score`, [`score`]):
//! `sonido.toml` + `eventos.json` (+ `voz.wav`, `voz.json`) → `audio.wav`,
//! with the instruments registered by the caller.

mod compose;
mod engine;
mod error;
mod lexicon;
mod lines;
mod pcm;
mod take;
mod utterance;
mod wav;

pub mod synth;

#[cfg(feature = "score")]
pub mod score;

#[cfg(feature = "screenplay")]
mod bed;
#[cfg(feature = "screenplay")]
mod narrate;
#[cfg(feature = "screenplay")]
mod script;

pub use compose::compose;
pub use engine::{
    engine_by_name, resample_pcm16, NarrationEngine, PiperEngine, Prosody, ScaffoldEngine,
};
pub use error::AudioError;
pub use lexicon::Lexicon;
pub use lines::{
    narrate_cast, narrate_lines, sentence_times, split_sentences, Line, Narration, Pacing,
    Sentence, SilenceRule, TimedLine,
};
pub use pcm::{read_wav_pcm16, write_wav, write_wav_pcm16, Pcm16};
pub use take::{fingerprint_file, Fnv, TakeStore, TakeUse, Voice, INDEX as TAKE_INDEX};
pub use utterance::{cut_take, narrate_utterances, LineCut, TakeNarration, Utterance};
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
