//! `reel.toml`: what a narrated reel's post declares so one command can make
//! it (R-0012 AC9). Paths are relative to the post's folder; `${VAR}` takes
//! a variable from the environment (a studio's setup script exports where
//! its voices and its Piper are).

use std::path::{Path, PathBuf};

use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReelToml {
    pub reel: ReelSpec,
    pub voice: VoiceSpec,
    pub scene: SceneSpec,
    pub sound: SoundSpec,
    #[serde(default)]
    pub publish: Option<PublishSpec>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReelSpec {
    /// The reel's name: `<name>.mp4`, `<name>-audio.mp4`. Default: the folder's name.
    #[serde(default)]
    pub name: Option<String>,
    /// The brand profile (R-0012 AC1).
    pub brand: String,
    /// The longest the reel may last, s.
    pub max_duration: f64,
    /// The cover's time, s: a review frame is written there.
    #[serde(default)]
    pub cover: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VoiceSpec {
    /// The voice model `voz.toml` must name.
    pub model: String,
    /// The folder with `<model>/<model>.onnx`.
    pub voices: String,
    /// The Piper executable (default: `GUION_PIPER`, then `PATH`).
    #[serde(default)]
    pub piper: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneSpec {
    /// The Cargo manifest the scene's binary is built from.
    pub manifest: String,
    pub bin: String,
    #[serde(default)]
    pub features: Vec<String>,
    /// The flag the binary takes for loose review frames (`--frames 2.0`
    /// in guion-reel's `Command`; a studio may map its own).
    #[serde(default = "frames_flag")]
    pub frames_flag: String,
}

fn frames_flag() -> String {
    "--frames".into()
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SoundSpec {
    /// The command that writes the reel's sound (run in the post's folder;
    /// `{post}` is the folder's absolute path).
    pub command: Vec<String>,
    /// What it writes, relative to the post.
    #[serde(default = "sound_output")]
    pub output: String,
}

fn sound_output() -> String {
    "audio.wav".into()
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublishSpec {
    /// Where `caption.txt` and `alt-text.txt` go (default: the post).
    pub dir: String,
}

/// `${VAR}` replaced by the environment's value; an unset variable is an error.
pub fn expand(s: &str) -> Result<String, String> {
    let mut out = String::new();
    let mut rest = s;
    while let Some(i) = rest.find("${") {
        out.push_str(&rest[..i]);
        let tail = &rest[i + 2..];
        let j = tail
            .find('}')
            .ok_or_else(|| format!("«{s}»: «${{» without «}}»"))?;
        let var = &tail[..j];
        let v =
            std::env::var(var).map_err(|_| format!("«{s}» needs ${{{var}}}, which is not set"))?;
        out.push_str(&v);
        rest = &tail[j + 1..];
    }
    out.push_str(rest);
    Ok(out)
}

impl ReelToml {
    pub fn load(post: &Path) -> Result<ReelToml, String> {
        let p = post.join("reel.toml");
        let text = std::fs::read_to_string(&p).map_err(|e| format!("{}: {e}", p.display()))?;
        toml::from_str(&text).map_err(|e| format!("{}: {e}", p.display()))
    }

    pub fn name(&self, post: &Path) -> String {
        self.reel.name.clone().unwrap_or_else(|| {
            post.file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default()
        })
    }
}

/// A path from `reel.toml`, expanded and resolved against the post.
pub fn path(post: &Path, s: &str) -> Result<PathBuf, String> {
    let e = expand(s)?;
    let p = PathBuf::from(e);
    Ok(if p.is_absolute() { p } else { post.join(p) })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands_the_environment_and_names_what_is_missing() {
        std::env::set_var("GUION_TEST_VOICES", "/v");
        assert_eq!(expand("${GUION_TEST_VOICES}/es").unwrap(), "/v/es");
        assert!(expand("${GUION_TEST_NOT_SET_ANYWHERE}")
            .unwrap_err()
            .contains("GUION_TEST_NOT_SET_ANYWHERE"));
    }

    #[test]
    fn unknown_keys_fail() {
        let t = "[reel]\nbrand = \"b.toml\"\nmax_duration = 90\ncovr = 2\n[voice]\nmodel = \"m\"\nvoices = \"v\"\n\
                 [scene]\nmanifest = \"C.toml\"\nbin = \"x\"\n[sound]\ncommand = [\"true\"]\n";
        let e = toml::from_str::<ReelToml>(t).unwrap_err().to_string();
        assert!(e.contains("covr"), "{e}");
    }
}
