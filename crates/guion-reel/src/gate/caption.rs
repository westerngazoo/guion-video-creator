//! The caption checks (R-0012 AC7), on the texts that are pasted (the
//! caption with its hashtags, and the alt text) as guion-publish exports
//! them: the platform's limits, U+202F between a number and its unit, and
//! nothing internal.

use std::path::Path;

use guion_publish::instagram::hashtags;
use guion_publish::text::is_word;

use super::profile::Profile;
use super::report::{Finding, MAX_AT};

/// A narrow no-break space, the only space allowed between a number and its unit.
pub const NNBSP: char = '\u{202F}';

/// Every number followed by a unit without U+202F in between: `(context, what)`.
pub fn unit_spacing(text: &str, units: &[String]) -> Vec<String> {
    let cs: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    for i in 0..cs.len() {
        if !cs[i].is_ascii_digit()
            || cs.get(i + 1).is_some_and(|c| {
                c.is_ascii_digit() || *c == '.' && cs.get(i + 2).is_some_and(|d| d.is_ascii_digit())
            })
        {
            continue;
        }
        // the run of spaces after the number's last digit
        let mut j = i + 1;
        while j < cs.len() && cs[j].is_whitespace() {
            j += 1;
        }
        let space: String = cs[i + 1..j].iter().collect();
        for u in units {
            let uc: Vec<char> = u.chars().collect();
            if cs.len() < j + uc.len() || cs[j..j + uc.len()] != uc[..] {
                continue;
            }
            // a unit is a whole word: «5 gatos» is not grams
            let ends = cs
                .get(j + uc.len())
                .is_none_or(|c| !is_word(*c) || !uc.last().is_some_and(|l| is_word(*l)));
            if ends && space != NNBSP.to_string() {
                let a = i.saturating_sub(12);
                let ctx: String = cs[a..(j + uc.len()).min(cs.len())].iter().collect();
                let how = match space.as_str() {
                    "" => "no space".to_string(),
                    s => s
                        .chars()
                        .map(|c| format!("U+{:04X}", c as u32))
                        .collect::<Vec<_>>()
                        .join(" "),
                };
                out.push(format!(
                    "«…{}» ({how} before «{u}»)",
                    ctx.replace('\n', " ")
                ));
            }
        }
    }
    out
}

/// Whether `pat` (literal, `#` = one ASCII digit) occurs in `text`; the first match.
pub fn find_pattern(text: &str, pat: &str) -> Option<String> {
    let t: Vec<char> = text.chars().collect();
    let p: Vec<char> = pat.chars().collect();
    if p.is_empty() || t.len() < p.len() {
        return None;
    }
    (0..=t.len() - p.len())
        .find(|&i| {
            p.iter().zip(&t[i..]).all(|(pc, tc)| {
                if *pc == '#' {
                    tc.is_ascii_digit()
                } else {
                    pc.to_lowercase().eq(tc.to_lowercase())
                }
            })
        })
        .map(|i| t[i..i + p.len()].iter().collect())
}

/// The caption findings for `caption.md`.
pub fn check(caption_md: &Path, p: &Profile) -> Vec<Finding> {
    let c = &p.caption;
    let md = match guion_publish::text::read_text(caption_md) {
        Ok(m) => m,
        Err(e) => return vec![Finding::fail("caption", e.to_string(), Vec::new())],
    };
    let name = caption_md.display().to_string();
    let ex = match guion_publish::export(&md, &name) {
        Ok(e) => e,
        Err(e) => return vec![Finding::fail("caption", e.to_string(), Vec::new())],
    };
    let tags = hashtags(&ex.caption).len();
    let chars = ex.caption.chars().count();
    let mut out = vec![if tags <= c.max_hashtags && chars <= c.max_chars {
        Finding::pass(
            "caption",
            format!(
                "{chars} characters ≤ {}, {tags} hashtags ≤ {}",
                c.max_chars, c.max_hashtags
            ),
        )
    } else {
        Finding::fail(
            "caption",
            format!(
                "{chars} characters (≤ {}), {tags} hashtags (≤ {})",
                c.max_chars, c.max_hashtags
            ),
            Vec::new(),
        )
    }];
    let mut spacing: Vec<String> = unit_spacing(&ex.caption, &c.units)
        .into_iter()
        .map(|s| format!("caption {s}"))
        .collect();
    spacing.extend(
        unit_spacing(&ex.alt_text, &c.units)
            .into_iter()
            .map(|s| format!("alt {s}")),
    );
    let n = spacing.len();
    spacing.truncate(MAX_AT);
    out.push(Finding::from_misses(
        "units",
        "U+202F before every unit",
        format!("{n} numbers without U+202F before the unit"),
        spacing,
    ));
    let mut internal = Vec::new();
    for pat in &c.internal {
        for (which, t) in [("caption", &ex.caption), ("alt", &ex.alt_text)] {
            if let Some(m) = find_pattern(t, pat) {
                internal.push(format!("{which}: «{m}»"));
            }
        }
    }
    out.push(Finding::from_misses(
        "internal",
        "nothing internal",
        format!("{} internal references", internal.len()),
        internal,
    ));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn units() -> Vec<String> {
        ["%", "g", "km/h"].map(String::from).to_vec()
    }

    #[test]
    fn a_unit_needs_a_narrow_no_break_space() {
        assert!(unit_spacing("te queda 87\u{202F}% y 0.6\u{202F}g.", &units()).is_empty());
        assert_eq!(unit_spacing("87 % y 0.6g", &units()).len(), 2);
        assert_eq!(unit_spacing("a 20\u{00A0}km/h", &units()).len(), 1);
        // not units: a word that starts like one, a number that goes on
        assert!(unit_spacing("5 gatos, 1.2 de ejemplo, 2 200", &units()).is_empty());
    }

    #[test]
    fn internal_patterns_use_hash_for_a_digit() {
        assert_eq!(
            find_pattern("ver R-0009 aquí", "R-####").as_deref(),
            Some("R-0009")
        );
        assert_eq!(find_pattern("R-00x9", "R-####"), None);
        assert_eq!(
            find_pattern("en Guion-Video-Creator", "guion-video-creator").as_deref(),
            Some("Guion-Video-Creator")
        );
    }
}
