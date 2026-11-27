//! An external voice: a program that speaks a take (R-0020).
//!
//! The account's voice can be one Piper cannot be (a neural voice such as
//! Qwen3-TTS, cloned from a reference clip, in Python). guion does not link
//! it: it runs it, like it runs Piper, and keeps what it said as a take
//! ([`crate::TakeStore`]), so the reel stays byte-deterministic even though
//! the voice samples at random.
//!
//! The program's contract, [`Voice`]'s:
//! - **stdin:** the utterance's sentences, one per line, UTF-8, as
//!   [`split_sentences`] cuts them (the program never guesses where one ends);
//! - **arguments:** the configured ones, with `{salida}` replaced by the WAV
//!   to write and `{silencio}` by [`Voice::sentence_silence`] in seconds;
//! - **working directory:** the folder of the file that describes the voice,
//!   so its relative paths resolve there;
//! - **output:** a mono 16-bit PCM WAV in which every two sentences are
//!   separated by a run of exact zeros at least `silencio` s long and no
//!   other run that long is zero; exit status 0. Anything else is an error
//!   carrying the end of its stderr.

use std::cell::OnceCell;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::error::AudioError;
use crate::lines::split_sentences;
use crate::pcm::{read_wav_pcm16, Pcm16};
use crate::take::{fingerprint_file, Voice};

/// A voice spoken by an external program (see the module's contract).
#[derive(Debug)]
pub struct CommandVoice {
    program: String,
    args: Vec<String>,
    dir: PathBuf,
    sentence_silence: f64,
    /// What the configuration says, as written (no machine's paths in it).
    label: String,
    /// Files whose bytes decide the voice: the reference clip, its transcript,
    /// the model's weights.
    identity_files: Vec<PathBuf>,
    identity: OnceCell<String>,
}

static NEXT: AtomicU64 = AtomicU64::new(0);

impl CommandVoice {
    /// `program args…` run in `dir`. `label` names the configuration as
    /// written; it and the bytes of `identity_files` are the voice's
    /// identity (two machines with the same files share takes).
    ///
    /// Fails when `args` has no `{salida}` (the program would have nowhere
    /// to write) or `sentence_silence` is not positive (guion could not cut).
    pub fn new(
        program: impl Into<String>,
        args: Vec<String>,
        dir: impl Into<PathBuf>,
        sentence_silence: f64,
        label: impl Into<String>,
        identity_files: Vec<PathBuf>,
    ) -> Result<Self, AudioError> {
        if !args.iter().any(|a| a.contains("{salida}")) {
            return Err(AudioError::Engine(
                "the voice's arguments must name {salida}, the WAV it writes".into(),
            ));
        }
        if !(sentence_silence > 0.0 && sentence_silence.is_finite()) {
            return Err(AudioError::Engine(format!(
                "the voice's silence between sentences must be positive, not {sentence_silence}"
            )));
        }
        Ok(CommandVoice {
            program: program.into(),
            args,
            dir: dir.into(),
            sentence_silence,
            label: label.into(),
            identity_files,
            identity: OnceCell::new(),
        })
    }
}

impl CommandVoice {
    /// Runs the program once, writing to `out`, and reads its take.
    fn speak(&self, text: &str, out: &std::path::Path) -> Result<Pcm16, AudioError> {
        let salida = out.display().to_string();
        let silencio = self.sentence_silence.to_string();
        let args: Vec<String> = self
            .args
            .iter()
            .map(|a| {
                a.replace("{salida}", &salida)
                    .replace("{silencio}", &silencio)
            })
            .collect();
        let mut child = Command::new(&self.program)
            .args(&args)
            .current_dir(&self.dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| AudioError::Engine(format!("{}: {e}", self.program)))?;
        let frases = split_sentences(text).join("\n") + "\n";
        // a program that exits without reading its stdin closes the pipe: its
        // exit status and stderr say why, not the broken pipe; any other write
        // error is reported after the program was waited for
        let escrito = match child.stdin.take() {
            Some(mut stdin) => match stdin.write_all(frases.as_bytes()) {
                Err(e) if e.kind() != std::io::ErrorKind::BrokenPipe => Err(e),
                _ => Ok(()),
            },
            None => Ok(()),
        };
        let done = child.wait_with_output()?;
        if !done.status.success() {
            let err = String::from_utf8_lossy(&done.stderr);
            let tail: Vec<&str> = err.lines().rev().take(5).collect();
            return Err(AudioError::Engine(format!(
                "{} failed ({}): {}",
                self.program,
                done.status,
                tail.into_iter().rev().collect::<Vec<_>>().join(" | ")
            )));
        }
        escrito?;
        let bytes = std::fs::read(out).map_err(|e| {
            AudioError::Engine(format!("{} exited 0 but wrote no take: {e}", self.program))
        })?;
        let pcm = read_wav_pcm16(&bytes).map_err(|e| AudioError::Wav(format!("{salida}: {e}")))?;
        if pcm.channels != 1 {
            return Err(AudioError::Engine(format!(
                "{}: {} channels, mono expected",
                self.program, pcm.channels
            )));
        }
        Ok(pcm)
    }
}

impl Voice for CommandVoice {
    /// `label`, the silence, and each identity file's FNV-1a 64 (its name
    /// is not part of it: only its bytes). Fails when a file cannot be read.
    fn identity(&self) -> Result<String, AudioError> {
        if let Some(id) = self.identity.get() {
            return Ok(id.clone());
        }
        let mut id = format!("command {} silence {}", self.label, self.sentence_silence);
        for f in &self.identity_files {
            let p = if f.is_absolute() {
                f.clone()
            } else {
                self.dir.join(f)
            };
            id.push_str(&format!(" {:016x}", fingerprint_file(&p)?));
        }
        Ok(self.identity.get_or_init(|| id).clone())
    }

    fn take(&self, text: &str) -> Result<Pcm16, AudioError> {
        // a private folder of its own (create_dir fails if it exists): no stale
        // WAV of an earlier process, no file planted under a guessable name
        let dir = std::env::temp_dir().join(format!(
            "guion-voz-{}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos())
        ));
        std::fs::create_dir(&dir)
            .map_err(|e| AudioError::Engine(format!("{}: {e}", dir.display())))?;
        let r = self.speak(text, &dir.join("toma.wav"));
        let _ = std::fs::remove_dir_all(&dir);
        r
    }

    fn sentence_silence(&self) -> f64 {
        self.sentence_silence
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pcm::write_wav_pcm16;

    fn dir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("guion-command-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// A fake voice: `sh` saves stdin next to the fixture and copies the
    /// fixture to `{salida}`.
    fn fake(d: &std::path::Path) -> CommandVoice {
        write_wav_pcm16(&d.join("toma.wav"), 24_000, &[5, 7, 0, 0, 0, 9]).unwrap();
        CommandVoice::new(
            "sh",
            vec![
                "-c".into(),
                "cat > dicho.txt; echo \"$1\" > silencio.txt; cp toma.wav \"$0\"".into(),
                "{salida}".into(),
                "{silencio}".into(),
            ],
            d,
            0.3,
            "fake",
            vec!["toma.wav".into()],
        )
        .unwrap()
    }

    #[test]
    fn it_speaks_through_the_program_one_sentence_per_line() {
        let d = dir("ok");
        let v = fake(&d);
        let pcm = v.take("Le pides una cita. ¿Y ahora? Sí.").unwrap();
        assert_eq!((pcm.sample_rate, pcm.channels), (24_000, 1));
        assert_eq!(pcm.samples, vec![5, 7, 0, 0, 0, 9]);
        let dicho = std::fs::read_to_string(d.join("dicho.txt")).unwrap();
        assert_eq!(dicho, "Le pides una cita.\n¿Y ahora?\nSí.\n");
        let s = std::fs::read_to_string(d.join("silencio.txt")).unwrap();
        assert_eq!(s.trim(), "0.3");
        assert_eq!(v.sentence_silence(), 0.3);
    }

    #[test]
    fn the_identity_follows_the_files_bytes() {
        let d = dir("id");
        let a = fake(&d).identity().unwrap();
        assert!(a.starts_with("command fake silence 0.3 "), "{a}");
        assert_eq!(a, fake(&d).identity().unwrap());
        // a different reference clip: another voice, other takes
        let v = fake(&d);
        write_wav_pcm16(&d.join("toma.wav"), 24_000, &[1, 2, 3]).unwrap();
        assert_ne!(a, v.identity().unwrap());
    }

    #[test]
    fn a_failing_program_is_an_error_with_its_stderr() {
        let d = dir("falla");
        let v = CommandVoice::new(
            "sh",
            vec![
                "-c".into(),
                "echo 'cuatro tomas rechazadas' >&2; exit 1".into(),
                "{salida}".into(),
            ],
            &d,
            0.3,
            "falla",
            vec![],
        )
        .unwrap();
        // a long text: the write races the program's exit (CI saw EPIPE)
        let e = v.take(&"Hola. ".repeat(20_000)).unwrap_err().to_string();
        assert!(e.contains("cuatro tomas rechazadas"), "{e}");
    }

    fn voz(d: &std::path::Path, script: &str) -> CommandVoice {
        CommandVoice::new(
            "sh",
            vec!["-c".into(), script.into(), "{salida}".into()],
            d,
            0.3,
            "t",
            vec![],
        )
        .unwrap()
    }

    #[test]
    fn a_take_that_is_missing_stereo_or_not_a_wav_is_an_error() {
        let d = dir("rechazos");
        write_wav_pcm16(&d.join("estereo.wav"), 24_000, &[1, 2, 3, 4]).unwrap();
        // the stereo fixture: rewrite the header's channel count (offset 22) to 2
        let mut b = std::fs::read(d.join("estereo.wav")).unwrap();
        b[22] = 2;
        b[28..32].copy_from_slice(&(24_000u32 * 4).to_le_bytes());
        b[32] = 4;
        std::fs::write(d.join("estereo.wav"), b).unwrap();
        std::fs::write(d.join("texto.txt"), "no soy un wav").unwrap();
        let casos = [
            ("cat > /dev/null; exit 0", "wrote no take"),
            ("cat > /dev/null; cp estereo.wav \"$0\"", "mono expected"),
            ("cat > /dev/null; cp texto.txt \"$0\"", "toma.wav"),
        ];
        for (script, espera) in casos {
            let e = voz(&d, script).take("Hola.").unwrap_err().to_string();
            assert!(e.contains(espera), "«{script}»: «{e}» no dice «{espera}»");
        }
    }

    #[test]
    fn it_refuses_a_voice_with_nowhere_to_write_or_no_silence() {
        let d = dir("mal");
        assert!(CommandVoice::new("sh", vec!["-c".into()], &d, 0.3, "x", vec![]).is_err());
        assert!(CommandVoice::new("sh", vec!["{salida}".into()], &d, 0.0, "x", vec![]).is_err());
        let v = CommandVoice::new(
            "sh",
            vec!["{salida}".into()],
            &d,
            0.3,
            "x",
            vec!["no-esta.wav".into()],
        )
        .unwrap();
        assert!(
            v.identity().is_err(),
            "a missing identity file must fail, not hash nothing"
        );
    }
}
