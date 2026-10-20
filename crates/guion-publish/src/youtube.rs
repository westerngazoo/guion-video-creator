//! A YouTube upload's text: `youtube.md` → the title and the description
//! that are pasted in YouTube Studio, chapters included (R-0016).
//!
//! ```markdown
//! ## Título
//! ¿Por qué el kart levanta una rueda?
//!
//! ## Descripción
//! The idea, in a paragraph or two.
//!
//! ## Capítulos
//! - inicio | La pregunta
//! - gancho.fin | El modelo
//! - beat-3 | Frenar y girar
//!
//! ## Procedencia
//! What is computed, measured or reported, in words.
//! ```
//!
//! Section names are accepted in Spanish or English (`Title`, `Description`,
//! `Chapters`, `Provenance`). A chapter names an **anchor** (a voice line,
//! `<line>.fin`, `inicio`, an event of `eventos.json`, ± s): the caller
//! resolves it to seconds (this crate is std only and knows no voice) and
//! hands [`export`] the [`Capitulo`]s.
//!
//! The platform's rules (reported; see `docs/specs/0016-formats.md` for the
//! sources and how each was verified):
//!
//! - a title of at most [`MAX_TITLE_CHARS`] characters, and neither `<` nor
//!   `>` in the title or the description;
//! - a description of at most [`MAX_DESCRIPTION`] characters, and as many
//!   UTF-8 bytes (the Data API counts bytes, the Help Center characters: the
//!   stricter of the two holds);
//! - chapters: the first at `00:00`, at least [`MIN_CHAPTERS`], in ascending
//!   order, each at least [`MIN_CHAPTER_S`] s long.

use std::path::{Path, PathBuf};

use crate::error::PublishError;
use crate::instagram::{plain, sections};
use crate::text::read_text;

/// Characters a title may have.
pub const MAX_TITLE_CHARS: usize = 100;
/// Characters (and UTF-8 bytes) a description may have.
pub const MAX_DESCRIPTION: usize = 5000;
/// The fewest chapters YouTube shows.
pub const MIN_CHAPTERS: usize = 3;
/// The shortest chapter, s.
pub const MIN_CHAPTER_S: u64 = 10;
/// Characters the title and the description may not carry.
pub const FORBIDDEN: [char; 2] = ['<', '>'];

/// `youtube.md`, parsed.
#[derive(Debug, Clone, PartialEq)]
pub struct Fuente {
    pub titulo: String,
    pub descripcion: String,
    /// `(anchor, title)`, in the file's order.
    pub capitulos: Vec<(String, String)>,
    pub procedencia: String,
}

/// A chapter resolved to its start, s.
#[derive(Debug, Clone, PartialEq)]
pub struct Capitulo {
    pub inicio: f64,
    pub titulo: String,
}

/// The two texts, as they are pasted (no trailing newline).
#[derive(Debug, Clone, PartialEq)]
pub struct YoutubeExport {
    pub title: String,
    pub description: String,
}

impl YoutubeExport {
    /// Writes `titulo.txt` and `descripcion.txt` (each with a final newline) into `dir`.
    pub fn write_to(&self, dir: &Path) -> Result<(PathBuf, PathBuf), PublishError> {
        let (t, d) = (dir.join("titulo.txt"), dir.join("descripcion.txt"));
        for (p, s) in [(&t, &self.title), (&d, &self.description)] {
            std::fs::write(p, format!("{s}\n")).map_err(|e| PublishError::Io {
                path: p.display().to_string(),
                source: e,
            })?;
        }
        Ok((t, d))
    }
}

fn section<'a>(
    s: &'a std::collections::HashMap<String, String>,
    names: &[&str],
) -> Option<&'a String> {
    names.iter().find_map(|n| s.get(*n))
}

/// Parses `youtube.md`; `name` is how errors refer to it.
pub fn parse(md: &str, name: &str) -> Result<Fuente, PublishError> {
    let s = sections(md);
    let need = |names: &[&str]| {
        section(&s, names)
            .cloned()
            .ok_or_else(|| PublishError::MissingSection {
                path: name.to_string(),
                section: names[0].to_string(),
            })
    };
    let titulo = plain(&need(&["título", "titulo", "title"])?);
    let descripcion = need(&["descripción", "descripcion", "description"])?;
    let procedencia = need(&["procedencia", "provenance"])?;
    let mut capitulos = Vec::new();
    let mut bad = Vec::new();
    if let Some(c) = section(&s, &["capítulos", "capitulos", "chapters"]) {
        for line in c.lines().map(str::trim).filter(|l| !l.is_empty()) {
            let item = line.strip_prefix("- ").or_else(|| line.strip_prefix("* "));
            match item.and_then(|i| i.split_once('|')) {
                Some((a, t)) if !a.trim().is_empty() && !t.trim().is_empty() => {
                    capitulos.push((a.trim().to_string(), t.trim().to_string()))
                }
                _ => bad.push(format!("chapter «{line}»: write «- <anchor> | <title>»")),
            }
        }
    }
    if !bad.is_empty() {
        return Err(PublishError::Youtube {
            path: name.to_string(),
            problems: bad,
        });
    }
    Ok(Fuente {
        titulo,
        descripcion,
        capitulos,
        procedencia,
    })
}

/// Reads and parses a `youtube.md` ([`parse`]).
pub fn parse_file(path: &Path) -> Result<Fuente, PublishError> {
    parse(&read_text(path)?, &path.display().to_string())
}

/// A timestamp as YouTube reads it: `MM:SS`, or `H:MM:SS` when `hours`.
pub fn timestamp(secs: u64, hours: bool) -> String {
    if hours {
        format!("{}:{:02}:{:02}", secs / 3600, secs / 60 % 60, secs % 60)
    } else {
        format!("{:02}:{:02}", secs / 60, secs % 60)
    }
}

/// The chapters' lines (`00:00 Title`), each start floored to the second
/// (YouTube jumps to the second written; flooring never starts a chapter
/// after what it names).
pub fn chapter_lines(caps: &[Capitulo], duration: f64) -> String {
    let hours = duration >= 3600.0;
    caps.iter()
        .map(|c| {
            format!(
                "{} {}",
                timestamp(c.inicio.max(0.0).floor() as u64, hours),
                c.titulo
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn parse_stamp(s: &str) -> Option<u64> {
    let p: Vec<&str> = s.split(':').collect();
    if !(2..=3).contains(&p.len())
        || p.iter()
            .any(|x| x.is_empty() || !x.bytes().all(|b| b.is_ascii_digit()))
    {
        return None;
    }
    let n: Vec<u64> = p.iter().map(|x| x.parse().unwrap_or(0)).collect();
    if n[1..].iter().any(|&v| v >= 60) {
        return None;
    }
    Some(n.iter().fold(0, |acc, v| acc * 60 + v))
}

/// The chapters a description declares: every line that starts with a
/// timestamp (`00:00`, `1:05`, `1:02:03`) and a title, in order.
pub fn chapters_in(description: &str) -> Vec<(u64, String)> {
    description
        .lines()
        .filter_map(|l| {
            let (stamp, rest) = l.trim().split_once(char::is_whitespace)?;
            Some((parse_stamp(stamp)?, rest.trim().to_string()))
        })
        .collect()
}

/// What YouTube would refuse in these chapters (empty: they show). `starts`
/// in whole seconds, as written; `duration` is the video's, s.
pub fn validate_chapters(starts: &[u64], duration: f64) -> Vec<String> {
    let mut bad = Vec::new();
    if starts.len() < MIN_CHAPTERS {
        bad.push(format!(
            "{} chapters; YouTube shows them from {MIN_CHAPTERS}",
            starts.len()
        ));
    }
    if let Some(&first) = starts.first() {
        if first != 0 {
            bad.push(format!(
                "the first chapter starts at {}, not 00:00",
                timestamp(first, false)
            ));
        }
    }
    for (k, w) in starts.windows(2).enumerate() {
        match w[1].checked_sub(w[0]) {
            None | Some(0) => bad.push(format!(
                "chapter {} ({}) does not come after chapter {} ({})",
                k + 2,
                timestamp(w[1], false),
                k + 1,
                timestamp(w[0], false)
            )),
            Some(d) if d < MIN_CHAPTER_S => {
                bad.push(format!("chapter {} lasts {d} s < {MIN_CHAPTER_S} s", k + 1))
            }
            Some(_) => {}
        }
    }
    if let Some(&last) = starts.last() {
        let tail = duration - last as f64;
        if tail < MIN_CHAPTER_S as f64 {
            bad.push(format!(
                "the last chapter ({}) lasts {tail:.1} s < {MIN_CHAPTER_S} s",
                timestamp(last, false)
            ));
        }
    }
    bad
}

/// What YouTube would refuse in a title.
pub fn check_title(t: &str) -> Vec<String> {
    let mut bad = Vec::new();
    let n = t.chars().count();
    if t.trim().is_empty() {
        bad.push("the title is empty".into());
    }
    if n > MAX_TITLE_CHARS {
        bad.push(format!("the title is {n} characters > {MAX_TITLE_CHARS}"));
    }
    if t.contains(FORBIDDEN) {
        bad.push("the title has «<» or «>»".into());
    }
    bad
}

/// What YouTube would refuse in a description.
pub fn check_description(d: &str) -> Vec<String> {
    let mut bad = Vec::new();
    let (n, b) = (d.chars().count(), d.len());
    if n > MAX_DESCRIPTION || b > MAX_DESCRIPTION {
        bad.push(format!(
            "the description is {n} characters, {b} bytes > {MAX_DESCRIPTION}"
        ));
    }
    if d.contains(FORBIDDEN) {
        bad.push("the description has «<» or «>»".into());
    }
    bad
}

/// The title and the description: the description's text, a blank line,
/// the chapters (if any), a blank line, the provenance. Fails, writing
/// nothing, on anything the platform would refuse (chapters only when there
/// are some: a Short has none).
pub fn export(
    f: &Fuente,
    caps: &[Capitulo],
    duration: f64,
    name: &str,
) -> Result<YoutubeExport, PublishError> {
    let mut parts = vec![plain(&f.descripcion)];
    if !caps.is_empty() {
        parts.push(chapter_lines(caps, duration));
    }
    parts.push(plain(&f.procedencia));
    let description = parts
        .into_iter()
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n");
    let mut bad = check_title(&f.titulo);
    bad.extend(check_description(&description));
    if !caps.is_empty() {
        let starts: Vec<u64> = caps
            .iter()
            .map(|c| c.inicio.max(0.0).floor() as u64)
            .collect();
        bad.extend(validate_chapters(&starts, duration));
    }
    if !bad.is_empty() {
        return Err(PublishError::Youtube {
            path: name.to_string(),
            problems: bad,
        });
    }
    Ok(YoutubeExport {
        title: f.titulo.clone(),
        description,
    })
}
