use std::cell::OnceCell;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::compose::compose;
use crate::error::AudioError;
use crate::pcm::{read_wav_pcm16, Pcm16};
use crate::take::{fingerprint_file, Fnv, Voice};
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

/// Piper's prosody (R-0019 AC1): the four numbers that decide how a voice
/// breathes. `noise_scale` varies the pitch and timbre, `noise_w` the length
/// of each phoneme; `length_scale` > 1 speaks slower; `sentence_silence` is
/// the pause Piper writes between two sentences, in seconds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Prosody {
    pub noise_scale: f64,
    pub noise_w: f64,
    pub length_scale: f64,
    pub sentence_silence: f64,
}

impl Prosody {
    /// Preset «C», the owner's choice (2026-11-26, heard on
    /// `es_MX-claude-high`): fluid, and different on every run, so its takes
    /// are kept ([`crate::TakeStore`]). The default for new voices.
    pub const NATURAL: Prosody = Prosody {
        noise_scale: 0.75,
        noise_w: 0.9,
        length_scale: 1.0,
        sentence_silence: 0.35,
    };

    /// R-0007's voice: no noise, so the same text, voice and build give the
    /// same bytes. Flat; explicit only.
    pub const DETERMINISTIC: Prosody = Prosody {
        noise_scale: 0.0,
        noise_w: 0.0,
        length_scale: 1.0,
        sentence_silence: 0.25,
    };

    /// Every number finite, the noises and the silence ≥ 0, `length_scale`
    /// > 0. `Err` names the first parameter out of range.
    pub fn check(&self) -> Result<(), String> {
        for (name, v, ok) in [
            ("noise_scale", self.noise_scale, self.noise_scale >= 0.0),
            ("noise_w", self.noise_w, self.noise_w >= 0.0),
            ("length_scale", self.length_scale, self.length_scale > 0.0),
            (
                "sentence_silence",
                self.sentence_silence,
                self.sentence_silence >= 0.0,
            ),
        ] {
            if !(v.is_finite() && ok) {
                return Err(format!("{name} = {v} is out of range"));
            }
        }
        Ok(())
    }
}

impl Default for Prosody {
    fn default() -> Self {
        Prosody::NATURAL
    }
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
///
/// **Natural voice (R-0019).** [`PiperEngine::with_prosody`] with
/// [`Prosody::NATURAL`] turns the noise on; as a [`Voice`] it then makes
/// takes that a [`crate::TakeStore`] keeps, keyed by the model's bytes, the
/// build and the prosody.
pub struct PiperEngine {
    model: String,
    binary: Option<PathBuf>,
    noise_scale: Option<f64>,
    noise_w: Option<f64>,
    length_scale: Option<f64>,
    sentence_silence: Option<f64>,
    identity: OnceCell<String>,
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
            identity: OnceCell::new(),
        }
    }

    /// Piper with every prosody number set (R-0019 AC1).
    pub fn with_prosody(model: impl Into<String>, p: Prosody) -> Self {
        let mut e = PiperEngine::new(model);
        e.noise_scale = Some(p.noise_scale);
        e.noise_w = Some(p.noise_w);
        e.length_scale = Some(p.length_scale);
        e.sentence_silence = Some(p.sentence_silence);
        e
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
        self.run_with(text, wav, &[]).map(|_| ())
    }

    /// Runs Piper on `text` with `extra` flags; returns what it logged.
    ///
    /// The text is composed first (R-0019 AC10: espeak-ng reads «u» + a
    /// combining acute as a plain «u»), and the PyPI build is told to read
    /// its stdin as UTF-8 whatever the locale (with a Latin-1 locale, «ú»
    /// would arrive as two letters). Composed text is unchanged, so the
    /// legacy voice keeps its bytes.
    fn run_with(&self, text: &str, wav: &Path, extra: &[&str]) -> Result<String, AudioError> {
        use std::io::Write;
        let piper = match &self.binary {
            Some(b) => b.display().to_string(),
            None => which_piper()?,
        };
        // The text goes on stdin, not through a shell: no quoting to get wrong.
        let mut child = Command::new(&piper)
            .args(self.args(wav))
            .args(extra)
            .env("PYTHONIOENCODING", "utf-8")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| AudioError::Engine(format!("{piper}: {e}")))?;
        child
            .stdin
            .take()
            .expect("stdin is piped")
            .write_all(compose(text).as_bytes())?;
        let out = child.wait_with_output()?;
        let log = String::from_utf8_lossy(&out.stderr).into_owned();
        if !out.status.success() {
            return Err(AudioError::Engine(log.trim().to_string()));
        }
        Ok(log)
    }

    /// The phonemes Piper speaks for `text`, one string per sentence, read
    /// from its own `--debug` log: the exact path of synthesis (espeak-ng
    /// with the voice's language, R-0019 AC9). Only the PyPI build logs them;
    /// with another, or a text on several lines, it fails saying so.
    pub fn phonemes(&self, text: &str) -> Result<Vec<String>, AudioError> {
        if text.contains('\n') {
            return Err(AudioError::Engine(
                "phonemes: one line at a time (Piper reads each line apart)".into(),
            ));
        }
        let tmp = TempDir::new()?;
        let log = self.run_with(text, &tmp.0.join("piper.wav"), &["--debug"])?;
        log.lines()
            .find_map(|l| l.rfind("phonemes=[").map(|i| &l[i + "phonemes=".len()..]))
            .map(parse_phoneme_log)
            .ok_or_else(|| {
                AudioError::Engine(
                    "phonemes: this Piper logs no `phonemes=` with --debug (the PyPI build does)"
                        .into(),
                )
            })
    }
}

/// `[['p', 'ˈ', 'u'], ['l', 'a']]` (Python's repr of Piper's phonemes, one
/// list per sentence, one code point per item) → `["pˈu", "la"]`, composed.
fn parse_phoneme_log(repr: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut sentence = String::new();
    let mut depth = 0;
    let mut chars = repr.chars();
    while let Some(c) = chars.next() {
        match c {
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth == 1 {
                    out.push(compose(&std::mem::take(&mut sentence)));
                }
                if depth == 0 {
                    break;
                }
            }
            q @ ('\'' | '"') if depth == 2 => {
                while let Some(c) = chars.next() {
                    match c {
                        '\\' => sentence.extend(chars.next()),
                        c if c == q => break,
                        c => sentence.push(c),
                    }
                }
            }
            _ => {}
        }
    }
    out
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

/// Mono 16-bit `pcm` at `sample_rate`: the samples as they are when the rate
/// already matches, else resampled by ffmpeg in bit-exact mode (R-0007 AC1).
pub fn resample_pcm16(pcm: &Pcm16, sample_rate: u32) -> Result<Vec<i16>, AudioError> {
    if pcm.sample_rate == sample_rate && pcm.channels == 1 {
        return Ok(pcm.samples.clone());
    }
    let tmp = TempDir::new()?;
    let src = tmp.0.join("in.wav");
    crate::pcm::write_wav(&src, pcm)?;
    let fine = tmp.0.join("resampled.wav");
    resample_file(&src, &fine, sample_rate)?;
    Ok(read_wav_pcm16(&std::fs::read(&fine)?)?.samples)
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

impl PiperEngine {
    /// The `piper` this engine runs: [`PiperEngine::binary`], else
    /// `GUION_PIPER`, else `PATH`.
    fn executable(&self) -> Result<PathBuf, AudioError> {
        Ok(match &self.binary {
            Some(b) => b.clone(),
            None => PathBuf::from(which_piper()?),
        })
    }

    fn compute_identity(&self) -> Result<String, AudioError> {
        let model = Path::new(&self.model);
        let mut h = Fnv::default();
        h.write(&fingerprint_file(model)?.to_le_bytes());
        let config = PathBuf::from(format!("{}.json", self.model));
        h.write(&fingerprint_file(&config)?.to_le_bytes());
        let name = model
            .file_stem()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let num = |x: Option<f64>| x.map_or("default".to_string(), |v| format!("{v}"));
        Ok(format!(
            "piper {name} model={:016x} build=«{}» noise_scale={} noise_w={} length_scale={} \
             sentence_silence={}",
            h.finish(),
            piper_build(&self.executable()?)?,
            num(self.noise_scale),
            num(self.noise_w),
            num(self.length_scale),
            num(self.sentence_silence),
        ))
    }
}

/// Piper as a maker of takes (R-0019). The identity hashes the model and its
/// config (`<model>.json`, Piper's own convention) once per engine.
impl Voice for PiperEngine {
    fn identity(&self) -> Result<String, AudioError> {
        if let Some(id) = self.identity.get() {
            return Ok(id.clone());
        }
        let id = self.compute_identity()?;
        Ok(self.identity.get_or_init(|| id).clone())
    }

    fn take(&self, text: &str) -> Result<Pcm16, AudioError> {
        let pcm = self.synthesize_native(text)?;
        if pcm.channels != 1 {
            // Piper voices are mono; a stereo WAV would not cut into lines
            return Err(AudioError::Engine(format!(
                "{}: {} channels, mono expected",
                self.model, pcm.channels
            )));
        }
        Ok(pcm)
    }

    fn sentence_silence(&self) -> f64 {
        self.sentence_silence.unwrap_or(0.0)
    }
}

/// The Piper build, for the take key (R-0019 AC4): `piper-tts <v>` when
/// `piper` is a Python launcher whose package can be found (the PyPI build:
/// its `dist-info` folder in the launcher's or the interpreter's
/// environment, or in a `PYTHONPATH=` the launcher sets, as rotorf-sico's
/// `tools/setup.sh` wrapper does); else the executable's bytes. A launcher
/// holds its machine's paths, so hashing it would tie a committed take to
/// one machine. The release pins the phonemizer and its espeak-ng data, what
/// decides how a text is said; onnxruntime, resolved by pip per machine,
/// only moves the numerics, so it stays out of the key.
fn piper_build(exe: &Path) -> Result<String, AudioError> {
    let bytes =
        std::fs::read(exe).map_err(|e| AudioError::Engine(format!("{}: {e}", exe.display())))?;
    if bytes.starts_with(b"#!") {
        if let Some(v) = python_release(exe, &String::from_utf8_lossy(&bytes)) {
            return Ok(v);
        }
    }
    let mut h = Fnv::default();
    h.write(&bytes);
    Ok(format!("bin:{:016x}", h.finish()))
}

/// The `piper-tts` release a Python launcher runs.
fn python_release(exe: &Path, script: &str) -> Option<String> {
    let exe = exe.canonicalize().unwrap_or_else(|_| exe.to_path_buf());
    let mut envs: Vec<PathBuf> = Vec::new();
    // <env>/bin/piper → <env>
    if let Some(v) = exe.parent().and_then(Path::parent) {
        envs.push(v.to_path_buf());
    }
    // #!<env>/bin/python3.12 → <env>
    let shebang = script.lines().next().unwrap_or_default();
    if let Some(interp) = shebang.trim_start_matches("#!").split_whitespace().next() {
        if let Some(v) = Path::new(interp).parent().and_then(Path::parent) {
            envs.push(v.to_path_buf());
        }
    }
    let mut sites: Vec<PathBuf> = Vec::new();
    for env in envs {
        for lib in ["lib", "lib64"] {
            if let Ok(rd) = std::fs::read_dir(env.join(lib)) {
                let mut pythons: Vec<PathBuf> = rd.flatten().map(|e| e.path()).collect();
                pythons.sort();
                sites.extend(pythons.into_iter().map(|p| p.join("site-packages")));
            }
        }
    }
    // `PYTHONPATH=<dir> exec …`: a wrapper around a `pip install --target`
    for (i, _) in script.match_indices("PYTHONPATH=") {
        let value = script[i + "PYTHONPATH=".len()..]
            .split_whitespace()
            .next()
            .unwrap_or_default();
        sites.extend(
            value
                .trim_matches(['"', '\''])
                .split(':')
                .map(PathBuf::from),
        );
    }
    sites.iter().find_map(|site| {
        let mut found: Vec<String> = std::fs::read_dir(site)
            .ok()?
            .flatten()
            .filter_map(|e| {
                let n = e.file_name().to_string_lossy().into_owned();
                n.strip_prefix("piper_tts-")?
                    .strip_suffix(".dist-info")
                    .map(str::to_string)
            })
            .collect();
        found.sort();
        found.pop().map(|v| format!("piper-tts {v}"))
    })
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

    /// R-0019 AC1: the natural preset is the owner's «C», passed with the
    /// spellings both builds accept; a number out of range is named.
    #[test]
    fn the_natural_preset_is_c() {
        let a = PiperEngine::with_prosody("m.onnx", Prosody::NATURAL).args(Path::new("o.wav"));
        assert_eq!(
            a[2..10],
            [
                "--noise_scale",
                "0.75",
                "--noise_w",
                "0.9",
                "--length_scale",
                "1",
                "--sentence_silence",
                "0.35"
            ]
        );
        assert_eq!(Prosody::default(), Prosody::NATURAL);
        let d = Prosody::DETERMINISTIC;
        assert_eq!(
            (d.noise_scale, d.noise_w, d.sentence_silence),
            (0.0, 0.0, 0.25)
        );
        let bad = Prosody {
            length_scale: 0.0,
            ..Prosody::NATURAL
        };
        assert!(bad.check().unwrap_err().contains("length_scale"));
        let bad = Prosody {
            noise_w: f64::NAN,
            ..Prosody::NATURAL
        };
        assert!(bad.check().unwrap_err().contains("noise_w"));
        assert!(Prosody::NATURAL.check().is_ok());
    }

    /// R-0019 AC4: a Python launcher is known by its release, wherever it
    /// lives (a venv, or rotorf-sico's `PYTHONPATH` wrapper); a binary by its
    /// bytes.
    #[test]
    fn the_build_is_the_release_not_the_launcher() {
        let root = std::env::temp_dir().join(format!("guion-build-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        // a venv: <venv>/bin/piper, <venv>/lib/python3.12/site-packages/…
        let site = root.join("venv/lib/python3.12/site-packages");
        std::fs::create_dir_all(site.join("piper_tts-1.8.0.dist-info")).unwrap();
        std::fs::create_dir_all(site.join("onnxruntime-1.31.1.dist-info")).unwrap();
        std::fs::create_dir_all(root.join("venv/bin")).unwrap();
        let exe = root.join("venv/bin/piper");
        std::fs::write(
            &exe,
            format!(
                "#!{}/venv/bin/python3.12\nfrom piper.__main__ import main\n",
                root.display()
            ),
        )
        .unwrap();
        let venv = piper_build(&exe).unwrap();
        assert_eq!(venv, "piper-tts 1.8.0");
        // a wrapper elsewhere, pointing at a `pip install --target` folder
        // (with another onnxruntime: pip picks it per machine)
        let target = root.join("py");
        std::fs::create_dir_all(target.join("piper_tts-1.8.0.dist-info")).unwrap();
        std::fs::create_dir_all(target.join("onnxruntime-1.24.0.dist-info")).unwrap();
        let wrapper = root.join("piper-wrapper");
        std::fs::write(
            &wrapper,
            format!(
                "#!/bin/sh\nPYTHONPATH={0} exec {0}/bin/piper \"$@\"\n",
                target.display()
            ),
        )
        .unwrap();
        assert_eq!(
            piper_build(&wrapper).unwrap(),
            venv,
            "same release, same build"
        );
        // a binary: its bytes
        let bin = root.join("piper-cpp");
        std::fs::write(&bin, b"\x7fELF...").unwrap();
        assert!(piper_build(&bin).unwrap().starts_with("bin:"));
        let _ = std::fs::remove_dir_all(&root);
    }

    /// R-0019 AC9: Piper's `--debug` line (as piper-tts 1.8.0 logs it) reads
    /// back as one phoneme string per sentence.
    #[test]
    fn the_phoneme_log_reads_back() {
        let line = "DEBUG:piper.voice:text=¿Cómo? Lo hago como tú., phonemes=[['k', 'ˈ', 'o', \
                    'm', 'o', '?'], ['l', 'o', ' ', 'ˈ', 'a', 'ɣ', 'o', ' ', 'k', 'ˌ', 'o', 'm', \
                    'o', ' ', 't', 'ˈ', 'u', '.', \"'\"]]";
        let i = line.rfind("phonemes=[").unwrap();
        assert_eq!(
            parse_phoneme_log(&line[i + "phonemes=".len()..]),
            ["kˈomo?", "lo ˈaɣo kˌomo tˈu.'"]
        );
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
