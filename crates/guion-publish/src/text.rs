//! Text as Python's `re` and `str` see it.
//!
//! The export replaces a reviewed Python tool and must give **the same bytes**
//! for every caption written for it, so its character classes are Python's,
//! from generated tables ([`crate::py_unicode`], `tools/gen_tables.py`), not
//! Rust's: Python's `\w` is not `char::is_alphanumeric` (they differ on marks
//! with Other_Alphabetic, for example), and `str.isspace` counts U+001C to
//! U+001F, which Rust does not.

use std::path::Path;

use crate::error::PublishError;
use crate::py_unicode;

fn within(c: char, table: &[(u32, u32)]) -> bool {
    let c = c as u32;
    table
        .binary_search_by(|&(a, b)| {
            if b < c {
                std::cmp::Ordering::Less
            } else if a > c {
                std::cmp::Ordering::Greater
            } else {
                std::cmp::Ordering::Equal
            }
        })
        .is_ok()
}

/// `\w` of `re` on `str`: letters, numbers and the underscore.
pub fn is_word(c: char) -> bool {
    within(c, py_unicode::WORD)
}

/// `str.isspace` (and `\s` of `re`).
pub fn is_space(c: char) -> bool {
    within(c, py_unicode::SPACE)
}

/// `str.strip()` with no argument.
pub fn strip(s: &str) -> &str {
    s.trim_matches(is_space)
}

/// `str.splitlines()`: splits on `\n`, `\r`, `\r\n`, `\v`, `\f`, `\x1c`–`\x1e`,
/// `\x85`, U+2028 and U+2029, with no empty line at the end.
pub fn splitlines(s: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut it = s.char_indices().peekable();
    while let Some((i, c)) = it.next() {
        if within(c, py_unicode::LINEBREAK) {
            out.push(&s[start..i]);
            let mut end = i + c.len_utf8();
            if c == '\r' {
                if let Some(&(_, '\n')) = it.peek() {
                    it.next();
                    end += 1;
                }
            }
            start = end;
        }
    }
    if start < s.len() {
        out.push(&s[start..]);
    }
    out
}

/// `Path.read_text()`: UTF-8, with universal newlines (`\r\n` and `\r` become `\n`).
pub fn read_text(path: &Path) -> Result<String, PublishError> {
    let bytes = std::fs::read(path).map_err(|e| PublishError::Io {
        path: path.display().to_string(),
        source: e,
    })?;
    let text = String::from_utf8(bytes).map_err(|e| PublishError::NotUtf8 {
        path: path.display().to_string(),
        at: e.utf8_error().valid_up_to(),
    })?;
    if !text.contains('\r') {
        return Ok(text);
    }
    Ok(text.replace("\r\n", "\n").replace('\r', "\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classes_as_python() {
        assert!(is_word('á') && is_word('_') && is_word('٣') && !is_word('-'));
        assert!(!is_word('\u{0303}')); // the combining tilde of R̃ is not \w
        assert!(is_space('\u{202F}') && is_space('\u{1C}') && !is_space('\u{200B}'));
    }

    #[test]
    fn splitlines_as_python() {
        assert_eq!(splitlines("a\r\nb\rc\n"), vec!["a", "b", "c"]);
        assert_eq!(splitlines("\n"), vec![""]);
        assert_eq!(splitlines(""), Vec::<&str>::new());
        assert_eq!(splitlines("a\u{2028}b\x0bc"), vec!["a", "b", "c"]);
    }
}
