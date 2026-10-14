//! 16-bit PCM WAV, read and written exactly (R-0007 AC1, AC4).
//!
//! The cue-list path keeps samples as `i16` end to end: a round trip through
//! `f32` and back (`* 32767`) moves samples by one step and breaks the
//! byte-for-byte guarantee. The writer emits the canonical 44-byte header,
//! the same one Python's `wave` module writes, so a studio can replace a
//! Python narration tool and compare files with `md5sum`.

use std::io::Write;
use std::path::Path;

use crate::error::AudioError;

/// Interleaved 16-bit samples and their format.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pcm16 {
    pub sample_rate: u32,
    pub channels: u16,
    pub samples: Vec<i16>,
}

/// Reads a PCM 16-bit WAV, walking its chunks (`LIST` and other chunks before
/// `data` are skipped, unlike a fixed 44-byte offset).
pub fn read_wav_pcm16(bytes: &[u8]) -> Result<Pcm16, AudioError> {
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err(AudioError::Wav("no es RIFF/WAVE".into()));
    }
    let mut i = 12;
    let mut fmt: Option<(u16, u16, u32, u16)> = None;
    while i + 8 <= bytes.len() {
        let id = &bytes[i..i + 4];
        let len =
            u32::from_le_bytes([bytes[i + 4], bytes[i + 5], bytes[i + 6], bytes[i + 7]]) as usize;
        let body = i + 8;
        let end = body.saturating_add(len).min(bytes.len());
        if id == b"fmt " {
            if len < 16 || end - body < 16 {
                return Err(AudioError::Wav("chunk fmt corto".into()));
            }
            let b = &bytes[body..];
            let format = u16::from_le_bytes([b[0], b[1]]);
            let channels = u16::from_le_bytes([b[2], b[3]]);
            let rate = u32::from_le_bytes([b[4], b[5], b[6], b[7]]);
            let bits = u16::from_le_bytes([b[14], b[15]]);
            fmt = Some((format, channels, rate, bits));
        } else if id == b"data" {
            let (format, channels, sample_rate, bits) =
                fmt.ok_or_else(|| AudioError::Wav("data antes de fmt".into()))?;
            // 1 = PCM; 0xFFFE = WAVE_FORMAT_EXTENSIBLE (ffmpeg's choice for >2 channels)
            if !(format == 1 || format == 0xFFFE) || bits != 16 || channels == 0 {
                return Err(AudioError::Wav(format!(
                    "se esperaba PCM de 16 bit; viene formato {format}, {bits} bit, {channels} canales"
                )));
            }
            let samples = bytes[body..end]
                .chunks_exact(2)
                .map(|c| i16::from_le_bytes([c[0], c[1]]))
                .collect();
            return Ok(Pcm16 {
                sample_rate,
                channels,
                samples,
            });
        }
        i = body + len + (len & 1);
    }
    Err(AudioError::Wav("sin chunk data".into()))
}

/// Writes mono 16-bit PCM with the canonical 44-byte header. No partial
/// output: the file is written beside `path` and renamed on success.
pub fn write_wav_pcm16(path: &Path, sample_rate: u32, samples: &[i16]) -> Result<(), AudioError> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }
    let data_len = (samples.len() * 2) as u32;
    let mut out = Vec::with_capacity(44 + samples.len() * 2);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&1u16.to_le_bytes()); // mono
    out.extend_from_slice(&sample_rate.to_le_bytes());
    out.extend_from_slice(&(sample_rate * 2).to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for s in samples {
        out.extend_from_slice(&s.to_le_bytes());
    }
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "audio.wav".into());
    let tmp = path.with_file_name(format!(".{name}.parcial"));
    let r = std::fs::File::create(&tmp).and_then(|mut f| f.write_all(&out).and_then(|_| f.flush()));
    if let Err(e) = r {
        let _ = std::fs::remove_file(&tmp);
        return Err(e.into());
    }
    std::fs::rename(&tmp, path)?;
    Ok(())
}
