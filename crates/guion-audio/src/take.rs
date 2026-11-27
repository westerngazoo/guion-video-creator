//! Takes: an utterance as the voice spoke it, kept so it is spoken once (R-0019).
//!
//! A voice with prosody noise (Piper with `noise_scale > 0`) says the same
//! text differently on every run, so the clock it sets (`voz.json`) and the
//! track (`voz.wav`) would change with every re-run. A take breaks that: the
//! first run speaks each utterance and stores it under a key; every later run
//! with the same inputs reads it back. Everything downstream (the layout, the
//! resampling, the render, the gate's two renders) is deterministic, so the
//! same takes give the same bytes.
//!
//! The key is FNV-1a 64 of the voice's [`Voice::identity`] and the text, the
//! same stable, dependency-free hash as `guion-profesor`'s voice cache
//! (`std`'s `DefaultHasher` may change between Rust releases; a key on disk
//! must not).

use std::collections::BTreeSet;
use std::io::Read;
use std::path::{Path, PathBuf};

use crate::error::AudioError;
use crate::pcm::{read_wav_pcm16, write_wav_pcm16, Pcm16};

/// A voice that speaks a whole utterance at its own sample rate (R-0019).
///
/// Contract: [`Voice::take`] returns mono 16-bit samples in which every two
/// sentences are separated by a run of exact zero samples
/// [`Voice::sentence_silence`] seconds long, and nothing else in the speech
/// is such a run (Piper's behaviour, measured; R-0019 AC6). The take ends
/// where its speech ends.
pub trait Voice {
    /// Everything besides the text that decides a take: the model's bytes,
    /// the engine's build, the prosody. Two voices with the same identity
    /// are interchangeable. Fails when a file it names cannot be read.
    fn identity(&self) -> Result<String, AudioError>;

    /// Speaks `text` as one utterance.
    fn take(&self, text: &str) -> Result<Pcm16, AudioError>;

    /// Seconds of digital silence between two sentences of a take.
    fn sentence_silence(&self) -> f64;
}

/// FNV-1a 64, fed in pieces.
#[derive(Debug, Clone, Copy)]
pub struct Fnv(u64);

impl Default for Fnv {
    fn default() -> Self {
        Fnv(0xcbf2_9ce4_8422_2325)
    }
}

impl Fnv {
    pub fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.0 = (self.0 ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3);
        }
    }

    pub fn finish(&self) -> u64 {
        self.0
    }
}

/// FNV-1a 64 of a file's bytes, read in 1 MiB pieces (a voice model is
/// 20–110 MB). Fails when the file cannot be read.
pub fn fingerprint_file(path: &Path) -> Result<u64, AudioError> {
    let mut f = std::fs::File::open(path)
        .map_err(|e| AudioError::Engine(format!("{}: {e}", path.display())))?;
    let mut h = Fnv::default();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = f
            .read(&mut buf)
            .map_err(|e| AudioError::Engine(format!("{}: {e}", path.display())))?;
        if n == 0 {
            return Ok(h.finish());
        }
        h.write(&buf[..n]);
    }
}

/// One take a narration used: which key, whether it was already stored,
/// and what it says (for the index).
#[derive(Debug, Clone, PartialEq)]
pub struct TakeUse {
    pub key: String,
    /// `true`: read from the store; `false`: spoken in this run.
    pub cached: bool,
    /// The ids of the lines it holds, in order.
    pub lines: Vec<String>,
    pub identity: String,
    pub text: String,
}

/// The takes of one post: `<dir>/<key>.wav`, plus an index and a
/// `.gitignore` that lets the WAVs be committed (R-0019 AC5).
#[derive(Debug, Clone)]
pub struct TakeStore {
    dir: PathBuf,
}

/// The index's name inside the store.
pub const INDEX: &str = "tomas.txt";

impl TakeStore {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        TakeStore { dir: dir.into() }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// The key of `text` spoken by a voice of `identity`: 16 hex digits.
    pub fn key(identity: &str, text: &str) -> String {
        let mut h = Fnv::default();
        h.write(identity.as_bytes());
        h.write(b"\n");
        h.write(text.as_bytes());
        format!("{:016x}", h.finish())
    }

    fn path(&self, key: &str) -> PathBuf {
        self.dir.join(format!("{key}.wav"))
    }

    /// The stored take, if there is one. A file that is there but cannot be
    /// read is an error, not a miss: it is never silently spoken again.
    pub fn load(&self, key: &str) -> Result<Option<Pcm16>, AudioError> {
        let p = self.path(key);
        if !p.is_file() {
            return Ok(None);
        }
        let pcm = read_wav_pcm16(&std::fs::read(&p)?)
            .map_err(|e| AudioError::Wav(format!("{}: {e}", p.display())))?;
        if pcm.channels != 1 {
            return Err(AudioError::Wav(format!(
                "{}: a take is mono; this one has {} channels",
                p.display(),
                pcm.channels
            )));
        }
        Ok(Some(pcm))
    }

    /// Stores a mono take (written beside and renamed: no partial file).
    pub fn save(&self, key: &str, take: &Pcm16) -> Result<(), AudioError> {
        if take.channels != 1 {
            return Err(AudioError::Wav(format!(
                "take {key}: mono expected, {} channels",
                take.channels
            )));
        }
        std::fs::create_dir_all(&self.dir)?;
        write_wav_pcm16(&self.path(key), take.sample_rate, &take.samples)
    }

    /// After a successful narration: writes the index of `used` and the
    /// `.gitignore`, and removes every stored take not in `used`. Returns
    /// the keys removed. Same `used`, same bytes: the index has no dates.
    pub fn settle(&self, used: &[TakeUse]) -> Result<Vec<String>, AudioError> {
        std::fs::create_dir_all(&self.dir)?;
        let keep: BTreeSet<&str> = used.iter().map(|u| u.key.as_str()).collect();
        let mut removed = Vec::new();
        for e in std::fs::read_dir(&self.dir)? {
            let p = e?.path();
            let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or_default();
            let is_take = p.extension().is_some_and(|x| x == "wav")
                && stem.len() == 16
                && stem.bytes().all(|b| b.is_ascii_hexdigit());
            if is_take && !keep.contains(stem) {
                std::fs::remove_file(&p)?;
                removed.push(stem.to_string());
            }
        }
        removed.sort();
        write_text(&self.dir.join(".gitignore"), GITIGNORE)?;
        write_text(&self.dir.join(INDEX), &index(used))?;
        Ok(removed)
    }
}

/// An account repo usually ignores `*.wav` (renders); takes are source.
const GITIGNORE: &str = "# Takes are source (guion R-0019): commit them.\n!*.wav\n";

fn index(used: &[TakeUse]) -> String {
    let mut s = String::from(
        "# The voice's takes (guion R-0019): one WAV per utterance, spoken once.\n\
         # Delete a WAV to speak that utterance again with `guion reel voice`.\n",
    );
    let mut seen = BTreeSet::new();
    for u in used {
        if !seen.insert(&u.key) {
            continue;
        }
        s.push_str(&format!(
            "\n{}.wav  {}\n  voice: {}\n  text: {}\n",
            u.key,
            u.lines.join(", "),
            u.identity,
            u.text
        ));
    }
    s
}

fn write_text(path: &Path, text: &str) -> Result<(), AudioError> {
    if std::fs::read_to_string(path).is_ok_and(|t| t == text) {
        return Ok(());
    }
    let tmp = path.with_extension("parcial");
    std::fs::write(&tmp, text)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_hash_is_fnv1a_64() {
        let mut h = Fnv::default();
        h.write(b"a");
        // the published FNV-1a 64 of "a" (and guion-profesor's key test)
        assert_eq!(h.finish(), 0xaf63_dc4c_8601_ec8c);
        assert_eq!(TakeStore::key("v", "hola").len(), 16);
        assert_ne!(TakeStore::key("v", "hola"), TakeStore::key("w", "hola"));
        assert_ne!(TakeStore::key("v", "hola"), TakeStore::key("v", "hola."));
    }
}
