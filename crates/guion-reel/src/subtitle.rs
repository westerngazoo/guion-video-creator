//! Subtitle layout, without a face (AC6).
//!
//! A subtitle mixes prose and short formulas ("i = e₁e₂: the plane's
//! square."). This module decides what is prose and what is formula, which
//! pieces never break apart, how many words there are to read, and where a
//! too-long subtitle breaks into two rows. It measures nothing itself: the
//! caller passes the width of a unit in its own faces and sizes, so the same
//! layout serves any studio's typography.

use crate::error::ReelError;

/// A piece of a subtitle: prose or a formula.
#[derive(Debug, Clone, PartialEq)]
pub enum Piece {
    Text(String),
    Formula(String),
}

/// What never breaks: a word, or a formula of several words, with the
/// punctuation that follows it.
pub type Unit = Vec<Piece>;

/// The default rule for "this word is a formula": it carries an operator, an
/// index or an exponent, or it is all digits, or it is the constant i alone.
pub fn is_formula_word(w: &str) -> bool {
    w == "i"
        || w.chars().any(|c| "₁₂₃²³⁴−=×√()+·".contains(c))
        || (!w.is_empty() && w.chars().all(|c| c.is_ascii_digit()))
}

/// Splits a subtitle into units. Consecutive formula words join into one
/// formula; the punctuation after a formula is prose.
pub fn units(s: &str, is_formula: impl Fn(&str) -> bool) -> Vec<Unit> {
    let mut out: Vec<Unit> = Vec::new();
    let mut open = false; // the last unit is a formula that may go on
    for w in s.split(' ').filter(|w| !w.is_empty()) {
        let core = w.trim_end_matches(['.', ',', ':', ';']);
        let tail = &w[core.len()..];
        if is_formula(core) {
            if open {
                if let Some(Piece::Formula(f)) = out.last_mut().and_then(|u| u.last_mut()) {
                    f.push(' ');
                    f.push_str(core);
                }
            } else {
                out.push(vec![Piece::Formula(core.to_string())]);
            }
            if !tail.is_empty() {
                if let Some(u) = out.last_mut() {
                    u.push(Piece::Text(tail.to_string()));
                }
            }
            open = tail.is_empty();
        } else {
            out.push(vec![Piece::Text(w.to_string())]);
            open = false;
        }
    }
    out
}

/// The words to read: units with a letter in their prose (formulas do not count).
pub fn word_count(units: &[Unit]) -> usize {
    units
        .iter()
        .filter(|u| {
            u.iter()
                .any(|p| matches!(p, Piece::Text(x) if x.chars().any(char::is_alphabetic)))
        })
        .count()
}

/// The width of a row: its units plus one `space` between each two.
pub fn row_width(row: &[Unit], unit_width: &impl Fn(&Unit) -> f64, space: f64) -> f64 {
    row.iter().map(unit_width).sum::<f64>() + space * row.len().saturating_sub(1) as f64
}

/// Breaks `units` into rows no wider than `max_width`: one row if it fits (or
/// if there is a single unit); otherwise, with `max_rows` ≥ 2, the two-row
/// break whose wider row is narrowest (the first such break on a tie). More
/// than two rows is not a subtitle; it fails.
pub fn break_rows(
    units: Vec<Unit>,
    unit_width: impl Fn(&Unit) -> f64,
    space: f64,
    max_width: f64,
    max_rows: usize,
    text: &str,
) -> Result<Vec<Vec<Unit>>, ReelError> {
    if row_width(&units, &unit_width, space) <= max_width || units.len() < 2 {
        return Ok(vec![units]);
    }
    if max_rows < 2 {
        return Err(ReelError::Subtitle(format!(
            "the subtitle «{text}» does not fit on one row"
        )));
    }
    let (k, w) = (1..units.len())
        .map(|k| {
            (
                k,
                row_width(&units[..k], &unit_width, space).max(row_width(
                    &units[k..],
                    &unit_width,
                    space,
                )),
            )
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .expect("two units or more give at least one break");
    if w > max_width {
        return Err(ReelError::Subtitle(format!(
            "the subtitle «{text}» does not fit on two rows of {max_width} px"
        )));
    }
    Ok(vec![units[..k].to_vec(), units[k..].to_vec()])
}
