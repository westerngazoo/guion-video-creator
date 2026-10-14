use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::error::AudioError;
use crate::pcm::{read_wav_pcm16, Pcm16};
use crate::wav::SAMPLE_RATE;

/// Synthesizes speech samples for one narration cue.
pub trait NarrationEngine {
    fn name(&self) -> &'static str;

    /// Return mono samples at [`SAMPLE_RATE`]. Duration may differ from the cue window.
    fn synthesize(&self, text: &str) -> Result<Vec<f32>, AudioError>;

    /// Mono 16-bit samples at `sample_rate`, exactly as the engine produced
    /// them (R-0007 AC1): no trip through `f32`. The default converts
    /// [`NarrationEngine::synthesize`] and only serves [`SAMPLE_RATE`].
    fn synthesize_pcm16(&self, text: &str, sample_rate: u32) -> Result<Vec<i16>, AudioError> {
        if sample_rate != SAMPLE_RATE {
            return Err(AudioError::Engine(format!(
                "{} solo sintetiza a {SAMPLE_RATE} Hz (se pidió {sample_rate})",
                self.name()
            )));
        }
        Ok(self
            .synthesize(text)?
            .iter()
            .map(|&s| f32_to_i16(s))
            .collect())
    }
}

/// MVP engine: short tone bursts proportional to cue length (no external deps).
pub struct ScaffoldEngine;

impl NarrationEngine for ScaffoldEngine {
    fn name(&self) -> &'static str {
        "scaffold"
    }

    fn synthesize(&self, text: &str) -> Result<Vec<f32>, AudioError> {
        Ok(tone_burst(scaffold_secs(text), 440.0, 0.12, SAMPLE_RATE))
    }

    fn synthesize_pcm16(&self, text: &str, sample_rate: u32) -> Result<Vec<i16>, AudioError> {
        Ok(tone_burst(scaffold_secs(text), 440.0, 0.12, sample_rate)
            .iter()
            .map(|&s| f32_to_i16(s))
            .collect())
    }
}

fn scaffold_secs(text: &str) -> f64 {
    let words = text.split_whitespace().count().max(1);
    (words as f64 * 0.18).clamp(0.35, 6.0)
}

/// Offline TTS via the `piper` CLI.
///
/// Works with both builds the ecosystem has used: the standalone C++ binary
/// (rhasspy/piper 2023.11.14-2, `piper` 1.2.0) and the PyPI `piper-tts`
/// package. Only the flag spellings both accept are passed (`--model`,
/// `--output_file`, `--noise_scale`, `--noise_w`, `--length_scale`,
/// `--sentence_silence`).
///
/// **Determinism (R-0007 AC1).** Piper samples prosody noise on every run;
/// [`PiperEngine::deterministic`] sets both noise scales to 0, and then the
/// same text, voice and build give the same bytes. Different builds of
/// Piper (another espeak-ng or onnxruntime) give slightly different speech:
/// pin the build.
pub struct PiperEngine {
    model: String,
    binary: Option<PathBuf>,
    noise_scale: Option<f64>,
    noise_w: Option<f64>,
    length_scale: Option<f64>,
    sentence_silence: Option<f64>,
}

impl PiperEngine {
    /// Piper with its own defaults (random prosody: not deterministic).
    pub fn new(model: impl Into<String>) -> Self {
        PiperEngine {
            model: model.into(),
            binary: None,
            noise_scale: None,
            noise_w: None,
            length_scale: None,
            sentence_silence: None,
        }
    }

    /// Piper with `noise_scale = 0` and `noise_w = 0`: same input, same bytes.
    pub fn deterministic(model: impl Into<String>) -> Self {
        let mut p = PiperEngine::new(model);
        p.noise_scale = Some(0.0);
        p.noise_w = Some(0.0);
        p
    }

    /// The `piper` executable to run. Without it: `GUION_PIPER`, then `PATH`.
    pub fn binary(mut self, path: impl Into<PathBuf>) -> Self {
        self.binary = Some(path.into());
        self
    }

    /// Phoneme length (`--length_scale`; > 1 speaks slower).
    pub fn length_scale(mut self, x: f64) -> Self {
        self.length_scale = Some(x);
        self
    }

    /// Seconds of silence Piper leaves after each sentence (`--sentence_silence`).
    pub fn sentence_silence(mut self, s: f64) -> Self {
        self.sentence_silence = Some(s);
        self
    }

    fn args(&self, out: &Path) -> Vec<String> {
        let mut a = vec!["--model".to_string(), self.model.clone()];
        for (flag, v) in [
            ("--noise_scale", self.noise_scale),
            ("--noise_w", self.noise_w),
            ("--length_scale", self.length_scale),
            ("--sentence_silence", self.sentence_silence),
        ] {
            if let Some(v) = v {
                a.push(flag.into());
                a.push(format!("{v}"));
            }
        }
        a.push("--output_file".into());
        a.push(out.display().to_string());
        a
    }

    /// One utterance at the voice's own sample rate (16 or 22.05 kHz), as
    /// Piper wrote it, minus the padding after the last sentence (below).
    pub fn synthesize_native(&self, text: &str) -> Result<Pcm16, AudioError> {
        let tmp = TempDir::new()?;
        let wav = tmp.0.join("piper.wav");
        self.run(text, &wav)?;
        let mut pcm = read_wav_pcm16(&std::fs::read(&wav)?)?;
        self.trim_last_sentence_silence(&mut pcm);
        Ok(pcm)
    }

    /// `sentence_silence` is a pause *between* sentences: an utterance ends
    /// where its speech ends (R-0007 AC7). The C++ build (1.2.0) also pads
    /// the last sentence with exactly `sentence_silence × rate` zero samples;
    /// the PyPI build (1.8) does not. When the output ends in exactly that
    /// run of zeros, it is removed, so both builds keep the same convention.
    /// Returns whether it trimmed.
    fn trim_last_sentence_silence(&self, pcm: &mut Pcm16) -> bool {
        let Some(s) = self.sentence_silence else {
            return false;
        };
        let n = (s * pcm.sample_rate as f64).round() as usize * pcm.channels as usize;
        let len = pcm.samples.len();
        if n == 0 || n >= len || pcm.samples[len - n..].iter().any(|&x| x != 0) {
            return false;
        }
        pcm.samples.truncate(len - n);
        true
    }

    fn run(&self, text: &str, wav: &Path) -> Result<(), AudioError> {
        use std::io::Write;
        let piper = match &self.binary {
            Some(b) => b.display().to_string(),
            None => which_piper()?,
        };
        // The text goes on stdin, not through a shell: no quoting to get wrong.
        let mut child = Command::new(&piper)
            .args(self.args(wav))
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| AudioError::Engine(format!("{piper}: {e}")))?;
        child
            .stdin
            .take()
            .expect("stdin is piped")
            .write_all(text.as_bytes())?;
        let out = child.wait_with_output()?;
        if !out.status.success() {
            return Err(AudioError::Engine(
                String::from_utf8_lossy(&out.stderr).trim().to_string(),
            ));
        }
        Ok(())
    }
}

impl NarrationEngine for PiperEngine {
    fn name(&self) -> &'static str {
        "piper"
    }

    /// Resampled to [`SAMPLE_RATE`]. (Before R-0007 the voice's native rate
    /// came back labelled as 44.1 kHz.)
    fn synthesize(&self, text: &str) -> Result<Vec<f32>, AudioError> {
        Ok(self
            .synthesize_pcm16(text, SAMPLE_RATE)?
            .iter()
            .map(|&s| s as f32 / 32_768.0)
            .collect())
    }

    /// Piper's WAV, resampled by ffmpeg in bit-exact mode (R-0007 AC1).
    fn synthesize_pcm16(&self, text: &str, sample_rate: u32) -> Result<Vec<i16>, AudioError> {
        let tmp = TempDir::new()?;
        let raw = tmp.0.join("piper.wav");
        self.run(text, &raw)?;
        let mut native = read_wav_pcm16(&std::fs::read(&raw)?)?;
        let src = if self.trim_last_sentence_silence(&mut native) {
            let trimmed = tmp.0.join("trimmed.wav");
            crate::pcm::write_wav_pcm16(&trimmed, native.sample_rate, &native.samples)?;
            trimmed
        } else {
            raw
        };
        if native.sample_rate == sample_rate && native.channels == 1 {
            return Ok(native.samples);
        }
        let fine = tmp.0.join("resampled.wav");
        resample_file(&src, &fine, sample_rate)?;
        Ok(read_wav_pcm16(&std::fs::read(&fine)?)?.samples)
    }
}

/// `input` → mono 16-bit at `sample_rate`, with ffmpeg's bit-exact flags
/// (no encoder tag, the same bytes on every run).
fn resample_file(input: &Path, output: &Path, sample_rate: u32) -> Result<(), AudioError> {
    let ff = guion_encode::ffmpeg::ffmpeg().map_err(|e| AudioError::Resample(e.to_string()))?;
    let out = Command::new(&ff)
        .args(["-y", "-loglevel", "error", "-i"])
        .arg(input)
        .args([
            "-ar",
            &sample_rate.to_string(),
            "-ac",
            "1",
            "-c:a",
            "pcm_s16le",
        ])
        .args(["-fflags", "+bitexact", "-flags:a", "+bitexact"])
        .arg(output)
        .output()
        .map_err(|e| AudioError::Resample(format!("{ff}: {e}")))?;
    if !out.status.success() {
        return Err(AudioError::Resample(
            String::from_utf8_lossy(&out.stderr).trim().to_string(),
        ));
    }
    Ok(())
}

pub fn engine_by_name(
    name: &str,
    piper_model: Option<&str>,
) -> Result<Box<dyn NarrationEngine>, AudioError> {
    match name {
        "scaffold" => Ok(Box::new(ScaffoldEngine)),
        "piper" => Ok(Box::new(PiperEngine::new(
            piper_model.unwrap_or("es_ES-sharvina-medium.onnx"),
        ))),
        other => Err(AudioError::Engine(format!(
            "motor desconocido: {other} (usa scaffold o piper)"
        ))),
    }
}

/// `GUION_PIPER` if set (a binary outside `PATH`), else `piper` on `PATH`.
fn which_piper() -> Result<String, AudioError> {
    if let Some(p) = std::env::var_os("GUION_PIPER").filter(|p| !p.is_empty()) {
        return Ok(p.to_string_lossy().into_owned());
    }
    let out = Command::new("which")
        .arg("piper")
        .output()
        .map_err(|e| AudioError::Engine(e.to_string()))?;
    if !out.status.success() {
        return Err(AudioError::PiperMissing);
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// A scratch directory removed on drop (no partial files left behind).
struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Result<Self, AudioError> {
        static N: AtomicUsize = AtomicUsize::new(0);
        let d = std::env::temp_dir().join(format!(
            "guion-audio-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&d)?;
        Ok(TempDir(d))
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn f32_to_i16(s: f32) -> i16 {
    (s.clamp(-1.0, 1.0) * 32_767.0).round() as i16
}

fn tone_burst(dur: f64, freq: f64, vol: f32, sample_rate: u32) -> Vec<f32> {
    let n = (dur * sample_rate as f64).ceil() as usize;
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let t = i as f64 / sample_rate as f64;
        let env = ((i as f64 / n as f64) * std::f64::consts::PI).sin();
        let s = vol * (2.0 * std::f64::consts::PI * freq * t).sin() as f32 * env as f32;
        out.push(s);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// R-0007 AC7: only the spellings both Piper builds accept, and the
    /// deterministic engine sets both noise scales to 0.
    #[test]
    fn piper_flags_are_shared_by_both_builds_and_deterministic() {
        let p = PiperEngine::deterministic("/voces/alan.onnx")
            .length_scale(1.0)
            .sentence_silence(0.25);
        let a = p.args(Path::new("/tmp/x.wav"));
        assert_eq!(
            a,
            [
                "--model",
                "/voces/alan.onnx",
                "--noise_scale",
                "0",
                "--noise_w",
                "0",
                "--length_scale",
                "1",
                "--sentence_silence",
                "0.25",
                "--output_file",
                "/tmp/x.wav"
            ]
        );
        // the default engine leaves Piper's own (random) prosody
        let a = PiperEngine::new("m.onnx").args(Path::new("o.wav"));
        assert!(!a.iter().any(|x| x.starts_with("--noise")));
    }

    /// R-0007 AC7: the C++ build's padding after the last sentence goes;
    /// an ending that is not exactly that run of zeros stays.
    #[test]
    fn trailing_sentence_padding_is_trimmed_only_when_exact() {
        let p = PiperEngine::deterministic("m").sentence_silence(0.25);
        let mut cpp = Pcm16 {
            sample_rate: 16_000,
            channels: 1,
            samples: [vec![300; 100], vec![0; 4000]].concat(),
        };
        assert!(p.trim_last_sentence_silence(&mut cpp));
        assert_eq!(cpp.samples, vec![300; 100]);
        let mut pypi = Pcm16 {
            sample_rate: 16_000,
            channels: 1,
            samples: [vec![300; 4100], vec![0; 1]].concat(),
        };
        let before = pypi.clone();
        assert!(!p.trim_last_sentence_silence(&mut pypi));
        assert_eq!(pypi, before);
    }
}
