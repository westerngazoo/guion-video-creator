//! The teacher's voice: macOS `say` per beat, cached by content, and the
//! narration track laid on the timeline. Durations are read off the
//! samples, never estimated, because they set the whole clock.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

pub const RATE: u32 = 22_050;
/// `say`'s natural pace, scaled by `narration.speed`.
const WPM: f64 = 175.0;

/// One beat spoken, as mono 16-bit samples at [`RATE`].
pub struct Clip {
    pub samples: Vec<i16>,
}

impl Clip {
    pub fn segundos(&self) -> f64 {
        self.samples.len() as f64 / f64::from(RATE)
    }
}

/// FNV-1a: a stable cache key with no dependency and no randomness.
fn huella(s: &str) -> u64 {
    s.bytes().fold(0xcbf2_9ce4_8422_2325, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// Speak `texto` with `voz`, reusing `cache/<hash>.wav` when it exists.
pub fn decir(texto: &str, voz: &str, speed: f64, cache: &Path) -> io::Result<Clip> {
    fs::create_dir_all(cache)?;
    let wpm = (WPM * speed).round() as u32;
    let path: PathBuf = cache.join(format!(
        "{:016x}.wav",
        huella(&format!("{voz}|{wpm}|{texto}"))
    ));
    if !path.exists() {
        let tmp = path.with_extension("tmp.wav");
        let st = Command::new("say")
            .args(["-v", voz, "-r", &wpm.to_string(), "--file-format=WAVE"])
            .arg(format!("--data-format=LEI16@{RATE}"))
            .arg("-o")
            .arg(&tmp)
            .arg(texto)
            .status()?;
        if !st.success() {
            return Err(io::Error::other(format!("say failed ({st}) for {texto:?}")));
        }
        fs::rename(&tmp, &path)?;
    }
    Ok(Clip {
        samples: leer_wav(&fs::read(&path)?)?,
    })
}

/// The PCM samples of a mono 16-bit WAV at [`RATE`].
pub fn leer_wav(b: &[u8]) -> io::Result<Vec<i16>> {
    let bad = |m: &str| io::Error::new(io::ErrorKind::InvalidData, format!("wav: {m}"));
    if b.len() < 12 || &b[0..4] != b"RIFF" || &b[8..12] != b"WAVE" {
        return Err(bad("not RIFF/WAVE"));
    }
    let u16_at = |i: usize| u16::from_le_bytes([b[i], b[i + 1]]);
    let u32_at = |i: usize| u32::from_le_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]]);
    let mut i = 12;
    let mut fmt_ok = false;
    while i + 8 <= b.len() {
        let id = &b[i..i + 4];
        let len = u32_at(i + 4) as usize;
        let body = i + 8;
        if body + len > b.len() && id != b"data" {
            return Err(bad("truncated chunk"));
        }
        if id == b"fmt " {
            fmt_ok = u16_at(body) == 1
                && u16_at(body + 2) == 1
                && u32_at(body + 4) == RATE
                && u16_at(body + 14) == 16;
        } else if id == b"data" {
            if !fmt_ok {
                return Err(bad("expected PCM, mono, 16-bit at 22050 Hz"));
            }
            let end = (body + len).min(b.len());
            return Ok(b[body..end]
                .as_chunks::<2>()
                .0
                .iter()
                .map(|c| i16::from_le_bytes(*c))
                .collect());
        }
        i = body + len + (len & 1);
    }
    Err(bad("no data chunk"))
}

pub fn escribir_wav(path: &Path, samples: &[i16]) -> io::Result<()> {
    let data = samples.len() as u32 * 2;
    let mut out = Vec::with_capacity(44 + data as usize);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&RATE.to_le_bytes());
    out.extend_from_slice(&(RATE * 2).to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data.to_le_bytes());
    for s in samples {
        out.extend_from_slice(&s.to_le_bytes());
    }
    fs::write(path, out)
}

/// Place each clip at its start time on a silent track `duracion` long.
pub fn mezclar(clips: &[(f64, &Clip)], duracion: f64) -> Vec<i16> {
    let n = (duracion * f64::from(RATE)).ceil() as usize;
    let mut track = vec![0i16; n];
    for (t0, c) in clips {
        let at = (t0 * f64::from(RATE)).round() as usize;
        for (dst, src) in track.iter_mut().skip(at).zip(&c.samples) {
            *dst = src.saturating_add(*dst);
        }
    }
    track
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_written_wav_reads_back() {
        let dir = std::env::temp_dir().join("guion-profesor-wav-test");
        fs::create_dir_all(&dir).unwrap();
        let p = dir.join("x.wav");
        escribir_wav(&p, &[1, -2, 3]).unwrap();
        assert_eq!(leer_wav(&fs::read(&p).unwrap()).unwrap(), vec![1, -2, 3]);
    }

    #[test]
    fn clips_land_at_their_start() {
        let c = Clip {
            samples: vec![7; 10],
        };
        let t = mezclar(&[(1.0, &c)], 2.0);
        assert_eq!(t.len(), 2 * RATE as usize);
        assert_eq!(t[RATE as usize - 1], 0);
        assert_eq!(t[RATE as usize], 7);
    }

    #[test]
    fn the_cache_key_is_stable() {
        assert_eq!(huella("a"), 0xaf63_dc4c_8601_ec8c);
    }
}
