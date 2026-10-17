//! An Instagram post's text: `caption.md` → what is pasted in the app (R-0010).
//!
//! A creator writes one Markdown file per post with three sections:
//!
//! ```markdown
//! ## Caption
//! The question, the idea, the formula.
//!
//! ## Hashtags
//! #physics #karting
//!
//! ## Alt text
//! What the video shows, for a screen reader.
//! ```
//!
//! [`export`] turns it into the two texts the app takes: the caption (the
//! caption, a blank line, the hashtags) and the alt text. Each paragraph is
//! flattened to one line, and `**bold**` and `_italic_` lose their marks (the
//! app shows them literally). It fails, before anything is written, when the
//! platform would refuse the post:
//!
//! - more than [`MAX_HASHTAGS`] hashtags, counted in the caption and the
//!   hashtags together (Instagram's limit since 2025-12-19);
//! - a pasted caption longer than [`MAX_CAPTION_CHARS`] characters, counted
//!   in code points, as Instagram counts them.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::error::PublishError;
use crate::text::{is_space, is_word, read_text, splitlines, strip};

/// Hashtags Instagram accepts on one post.
pub const MAX_HASHTAGS: usize = 5;
/// Characters Instagram accepts in a caption.
pub const MAX_CAPTION_CHARS: usize = 2200;

/// The two texts of a post, as they are pasted (no trailing newline).
#[derive(Debug, Clone, PartialEq)]
pub struct Export {
    pub caption: String,
    pub alt_text: String,
}

impl Export {
    /// Writes `caption.txt` and `alt-text.txt` (each with a final newline) into `dir`.
    pub fn write_to(&self, dir: &Path) -> Result<(PathBuf, PathBuf), PublishError> {
        let (caption, alt) = (dir.join("caption.txt"), dir.join("alt-text.txt"));
        let write = |p: &Path, s: &str| {
            std::fs::write(p, format!("{s}\n")).map_err(|e| PublishError::Io {
                path: p.display().to_string(),
                source: e,
            })
        };
        write(&caption, &self.caption)?;
        write(&alt, &self.alt_text)?;
        Ok((caption, alt))
    }
}

/// Reads `caption_md` and exports it ([`export`]).
pub fn export_file(caption_md: &Path) -> Result<Export, PublishError> {
    export(&read_text(caption_md)?, &caption_md.display().to_string())
}

/// Exports the text of a `caption.md`; `name` is how errors refer to it.
pub fn export(md: &str, name: &str) -> Result<Export, PublishError> {
    let sections = sections(md);
    let section = |k: &str| {
        sections.get(k).ok_or_else(|| PublishError::MissingSection {
            path: name.to_string(),
            section: k.to_string(),
        })
    };
    let (caption, tags, alt) = (
        section("caption")?,
        section("hashtags")?,
        section("alt text")?,
    );
    let count = hashtags(&format!("{tags} {caption}")).len();
    if count > MAX_HASHTAGS {
        return Err(PublishError::TooManyHashtags {
            path: name.to_string(),
            count,
            max: MAX_HASHTAGS,
        });
    }
    let pasted = format!("{}\n\n{}", plain(caption), plain(tags));
    let chars = pasted.chars().count();
    if chars > MAX_CAPTION_CHARS {
        return Err(PublishError::CaptionTooLong {
            path: name.to_string(),
            chars,
            max: MAX_CAPTION_CHARS,
        });
    }
    Ok(Export {
        caption: pasted,
        alt_text: plain(alt),
    })
}

/// The `## Name` sections (lower-cased names) with their text, stripped. A
/// repeated name starts over, as a later key does in a dict.
pub fn sections(md: &str) -> HashMap<String, String> {
    let mut out: HashMap<String, Vec<&str>> = HashMap::new();
    let mut current: Option<String> = None;
    for line in splitlines(md) {
        // ^## (.+)$
        if let Some(rest) = line.strip_prefix("## ").filter(|r| !r.is_empty()) {
            let n = strip(rest).to_lowercase();
            out.insert(n.clone(), Vec::new());
            current = Some(n);
        } else if let Some(n) = &current {
            out.get_mut(n)
                .expect("the current section was inserted")
                .push(line);
        }
    }
    out.into_iter()
        .map(|(k, v)| (k, strip(&v.join("\n")).to_string()))
        .collect()
}

/// `re.findall(r"#\w+", text)`.
pub fn hashtags(text: &str) -> Vec<String> {
    let c: Vec<char> = text.chars().collect();
    let (mut out, mut i) = (Vec::new(), 0);
    while i < c.len() {
        if c[i] == '#' {
            let mut j = i + 1;
            while j < c.len() && is_word(c[j]) {
                j += 1;
            }
            if j > i + 1 {
                out.push(c[i..j].iter().collect());
                i = j;
                continue;
            }
        }
        i += 1;
    }
    out
}

/// `re.split(r"\n[ \t]*\n", text)`.
fn paragraphs(c: &[char]) -> Vec<&[char]> {
    let (mut out, mut start, mut i) = (Vec::new(), 0, 0);
    while i < c.len() {
        if c[i] == '\n' {
            let mut j = i + 1;
            while j < c.len() && (c[j] == ' ' || c[j] == '\t') {
                j += 1;
            }
            if j < c.len() && c[j] == '\n' {
                out.push(&c[start..i]);
                start = j + 1;
                i = j + 1;
                continue;
            }
        }
        i += 1;
    }
    out.push(&c[start..]);
    out
}

/// `re.sub(r"[ \t\r\n]+", " ", p).strip()`.
fn one_line(p: &[char]) -> String {
    let mut s = String::new();
    let mut gap = false;
    for &ch in p {
        if matches!(ch, ' ' | '\t' | '\r' | '\n') {
            if !gap {
                s.push(' ');
            }
            gap = true;
        } else {
            s.push(ch);
            gap = false;
        }
    }
    strip(&s).to_string()
}

/// `re.sub(r"\*\*(.+?)\*\*|(?<!\w)_(.+?)_(?!\w)", <the inside>, p)`.
fn without_emphasis(p: &str) -> String {
    let c: Vec<char> = p.chars().collect();
    let n = c.len();
    let mut s = String::new();
    let mut i = 0;
    while i < n {
        // \*\*(.+?)\*\*: the first ** that leaves at least one character (not \n) inside
        if i + 1 < n && c[i] == '*' && c[i + 1] == '*' {
            let mut j = i + 3;
            let mut close = None;
            while j + 1 < n && c[j - 1] != '\n' {
                if c[j] == '*' && c[j + 1] == '*' {
                    close = Some(j);
                    break;
                }
                j += 1;
            }
            if let Some(j) = close {
                s.extend(&c[i + 2..j]);
                i = j + 2;
                continue;
            }
        }
        // (?<!\w)_(.+?)_(?!\w): the first _ not followed by a word character
        if c[i] == '_' && (i == 0 || !is_word(c[i - 1])) {
            let mut j = i + 2;
            let mut close = None;
            while j < n && c[j - 1] != '\n' {
                if c[j] == '_' && (j + 1 == n || !is_word(c[j + 1])) {
                    close = Some(j);
                    break;
                }
                j += 1;
            }
            if let Some(j) = close {
                s.extend(&c[i + 1..j]);
                i = j + 1;
                continue;
            }
        }
        s.push(c[i]);
        i += 1;
    }
    s
}

/// Text to paste in the app: each non-blank paragraph on one line, without
/// emphasis marks, paragraphs separated by a blank line.
pub fn plain(text: &str) -> String {
    let c: Vec<char> = text.chars().collect();
    paragraphs(&c)
        .into_iter()
        .filter(|p| p.iter().any(|&ch| !is_space(ch)))
        .map(|p| without_emphasis(&one_line(p)))
        .collect::<Vec<_>>()
        .join("\n\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emphasis_as_re() {
        assert_eq!(without_emphasis("a **b** c _d_ e"), "a b c d e");
        assert_eq!(without_emphasis("snake_case_name"), "snake_case_name");
        assert_eq!(without_emphasis("****"), "****"); // .+? needs a character: the first ** does not close on the second
        assert_eq!(without_emphasis("*****"), "*");
        assert_eq!(without_emphasis("_a_b_ c"), "a_b c");
        assert_eq!(without_emphasis("**a** **b**"), "a b");
    }

    #[test]
    fn paragraphs_as_re() {
        assert_eq!(plain("a\nb\n \t\nc\n\n\n\nd"), "a b\n\nc\n\nd");
        assert_eq!(plain("  \n\n x  y \n"), "x y");
    }
}
