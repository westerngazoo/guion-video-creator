//! The voice sets the clock: a narrated reel reads its instants from the
//! narration's timeline (AC5).
//!
//! The narrator (R-0007's `narrate_lines`, through a studio's own tool)
//! writes a timeline file: every line with its id, what is said, its
//! subtitle, its measured start and end, and its sentences with theirs. A
//! scene loads it with [`Narration::load`] and takes **all** its instants
//! from it:
//!
//! - [`Narration::span`]: when a line starts and ends;
//! - [`Narration::at`] and [`Narration::after`]: when a phrase starts or ends
//!   being said. The phrase falls in its sentence (measured) and, inside it,
//!   in proportion to the letters, with [`COMMA_PAUSE`] extra letters at each
//!   comma, colon or semicolon. That part is an estimate: what is animated
//!   that way should last 0.4 s or more;
//! - [`Narration::active`]: the line whose subtitle is on screen at `t`, and
//!   its opacity.
//!
//! If the script changes and a phrase is gone, [`Narration::at`] fails naming
//! it, so the scene never keeps a stale time.
//!
//! ## The file (`voz.json`)
//!
//! ```json
//! {"modelo": "en-gb-alan-low", "duracion_voz": 87.464,
//!  "lineas": [{"id": "gancho", "texto": "…", "subtitulo": "…", "t0": 0.5, "t1": 5.972,
//!              "desde": 2, "frases": [{"texto": "…", "t0": 0.5, "t1": 5.972}]}]}
//! ```
//!
//! `modelo`, `desde` (default 1) and `frases` (default: the whole line is one
//! sentence) are optional; any other key is an error. The keys are Spanish
//! because the studio that first wrote the file (rotorf-sico) did.

use std::path::Path;

use serde::Deserialize;

use crate::ease::{ease_out_cubic, progress};
use crate::error::ReelError;

/// How long a comma (or colon, or semicolon) lasts inside a sentence, in
/// letters: Piper speaks about 17 letters a second and pauses ~0.15 s there.
pub const COMMA_PAUSE: f64 = 2.5;

/// A sentence of a line, with its measured times (s).
#[derive(Debug, Clone, PartialEq)]
pub struct Sentence {
    pub text: String,
    pub t0: f64,
    pub t1: f64,
}

/// One narrated line.
#[derive(Debug, Clone, PartialEq)]
pub struct NarratedLine {
    pub id: String,
    pub text: String,
    pub subtitle: String,
    pub t0: f64,
    pub t1: f64,
    pub sentences: Vec<Sentence>,
    /// The sentence (1 = the first) the subtitle enters on: a long line does
    /// not show what has not been said yet.
    pub from: usize,
}

impl NarratedLine {
    /// When the subtitle enters: at the start of sentence [`NarratedLine::from`].
    pub fn subtitle_start(&self) -> f64 {
        self.sentences[self.from - 1].t0
    }
}

/// A reel's narration.
#[derive(Debug, Clone, PartialEq)]
pub struct Narration {
    /// The voice model, if the file names it.
    pub model: Option<String>,
    /// The voice track's length, s.
    pub duration: f64,
    pub lines: Vec<NarratedLine>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FileSentence {
    texto: String,
    t0: f64,
    t1: f64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FileLine {
    id: String,
    texto: String,
    subtitulo: String,
    t0: f64,
    t1: f64,
    #[serde(default)]
    desde: Option<usize>,
    #[serde(default)]
    frases: Option<Vec<FileSentence>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    #[serde(default)]
    modelo: Option<String>,
    duracion_voz: f64,
    lineas: Vec<FileLine>,
}

impl Narration {
    /// Reads a timeline file.
    pub fn load(path: &Path) -> Result<Narration, ReelError> {
        let text = std::fs::read_to_string(path)?;
        Narration::from_json(&text).map_err(|e| match e {
            ReelError::Narration { reason, .. } => ReelError::Narration {
                file: path.display().to_string(),
                reason,
            },
            other => other,
        })
    }

    /// Parses a timeline and checks that it holds together: `desde` names a
    /// sentence of its line, and no line starts before the previous one ends.
    pub fn from_json(text: &str) -> Result<Narration, ReelError> {
        let bad = |reason: String| ReelError::Narration {
            file: "voz.json".into(),
            reason,
        };
        let file: File = serde_json::from_str(text).map_err(|e| bad(e.to_string()))?;
        let mut lines = Vec::with_capacity(file.lineas.len());
        for l in file.lineas {
            let sentences = match l.frases {
                Some(fs) => fs
                    .into_iter()
                    .map(|f| Sentence {
                        text: f.texto,
                        t0: f.t0,
                        t1: f.t1,
                    })
                    .collect(),
                None => vec![Sentence {
                    text: l.texto.clone(),
                    t0: l.t0,
                    t1: l.t1,
                }],
            };
            let from = l.desde.unwrap_or(1);
            if from < 1 || from > sentences.len() {
                return Err(bad(format!(
                    "line «{}»: desde = {from} with {} sentences",
                    l.id,
                    sentences.len()
                )));
            }
            lines.push(NarratedLine {
                id: l.id,
                text: l.texto,
                subtitle: l.subtitulo,
                t0: l.t0,
                t1: l.t1,
                sentences,
                from,
            });
        }
        for w in lines.windows(2) {
            if w[1].t0 < w[0].t1 {
                return Err(bad(format!(
                    "«{}» starts before «{}» ends",
                    w[1].id, w[0].id
                )));
            }
        }
        Ok(Narration {
            model: file.modelo,
            duration: file.duracion_voz,
            lines,
        })
    }

    /// The line `id`.
    pub fn line(&self, id: &str) -> Result<&NarratedLine, ReelError> {
        self.lines
            .iter()
            .find(|l| l.id == id)
            .ok_or_else(|| ReelError::NoSuchLine { id: id.to_string() })
    }

    /// When line `id` starts and ends, s.
    pub fn span(&self, id: &str) -> Result<(f64, f64), ReelError> {
        let l = self.line(id)?;
        Ok((l.t0, l.t1))
    }

    /// When sentence `k` (0 = the first) of line `id` starts and ends, s (measured).
    pub fn sentence(&self, id: &str, k: usize) -> Result<(f64, f64), ReelError> {
        let l = self.line(id)?;
        let s = l
            .sentences
            .get(k)
            .ok_or_else(|| ReelError::NoSuchSentence {
                id: id.to_string(),
                index: k,
                count: l.sentences.len(),
            })?;
        Ok((s.t0, s.t1))
    }

    /// When `phrase` starts being said in line `id` (estimated inside its sentence).
    pub fn at(&self, id: &str, phrase: &str) -> Result<f64, ReelError> {
        self.locate(id, phrase, false)
    }

    /// When `phrase` has been said in line `id` (estimated inside its sentence).
    pub fn after(&self, id: &str, phrase: &str) -> Result<f64, ReelError> {
        self.locate(id, phrase, true)
    }

    fn locate(&self, id: &str, phrase: &str, end: bool) -> Result<f64, ReelError> {
        let l = self.line(id)?;
        for s in &l.sentences {
            if let Some(b) = s.text.find(phrase) {
                let b = if end { b + phrase.len() } else { b };
                let weight = |x: &str| {
                    x.chars()
                        .map(|c| {
                            if matches!(c, ',' | ':' | ';') {
                                1.0 + COMMA_PAUSE
                            } else {
                                1.0
                            }
                        })
                        .sum::<f64>()
                };
                let k = weight(&s.text[..b]) / weight(&s.text);
                return Ok(s.t0 + (s.t1 - s.t0) * k);
            }
        }
        Err(ReelError::NoSuchPhrase {
            id: id.to_string(),
            phrase: phrase.to_string(),
            text: l.text.clone(),
        })
    }

    /// The line whose subtitle shows (or is fading) at `t`, and its opacity:
    /// a cubic ease-out over `fade` s from [`NarratedLine::subtitle_start`]
    /// in, and over `fade` s from the line's end out.
    pub fn active(&self, t: f64, fade: f64) -> Option<(&NarratedLine, f64)> {
        self.lines.iter().find_map(|l| {
            let t0 = l.subtitle_start();
            let a = ease_out_cubic(progress(t, t0, t0 + fade))
                * (1.0 - ease_out_cubic(progress(t, l.t1, l.t1 + fade)));
            (a > 0.0).then_some((l, a))
        })
    }

    /// Fails if a subtitle would enter before the previous one has faded out.
    pub fn check_no_overlap(&self, fade: f64) -> Result<(), ReelError> {
        for w in self.lines.windows(2) {
            if w[1].subtitle_start() < w[0].t1 + fade {
                return Err(ReelError::Subtitle(format!(
                    "«{}» enters before «{}» leaves",
                    w[1].id, w[0].id
                )));
            }
        }
        Ok(())
    }
}
