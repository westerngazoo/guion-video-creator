//! The subtitle rules, read from `voz.json` and measured in the brand's face
//! (R-0012 AC2): one row no wider than the box, at most N words, no digits or
//! formula signs, a long line entering on a later sentence, never two at once.

use std::path::Path;

use motoreel_typeset::{measure, Face, FaceId, Fonts};

use super::profile::{Profile, SizeUnit};
use super::report::Finding;
use super::GateError;
use crate::narration::Narration;
use crate::subtitle::{is_formula_word, units, word_count, Piece, Unit};

/// The brand's subtitle face(s), loaded once, and how to measure with them.
pub struct SubtitleMeter {
    fonts: Fonts,
    text: FaceId,
    formula: FaceId,
    /// The size motoreel receives for the text face and for the formula face.
    text_size: f64,
    formula_size: f64,
    pad: f64,
    formula_words: bool,
}

/// `size` px in `unit`, as the scale motoreel (ab_glyph) takes: its size is
/// the face's line height, so an em size is multiplied by the face's line
/// height in em, `(ascent − descent) / unitsPerEm`.
fn motoreel_size(bytes: &[u8], size: f64, unit: SizeUnit) -> Result<f64, GateError> {
    match unit {
        SizeUnit::Line => Ok(size),
        SizeUnit::Em => {
            use ab_glyph::Font as _;
            let f = ab_glyph::FontRef::try_from_slice(bytes)
                .map_err(|_| GateError::Profile("subtitle.font is not a font".into()))?;
            let upem = f
                .units_per_em()
                .ok_or_else(|| GateError::Profile("subtitle.font has no unitsPerEm".into()))?;
            Ok(size * f64::from(f.ascent_unscaled() - f.descent_unscaled()) / f64::from(upem))
        }
    }
}

fn read_font(p: &Path) -> Result<Vec<u8>, GateError> {
    std::fs::read(p).map_err(|e| GateError::Input(format!("{}: {e}", p.display())))
}

impl SubtitleMeter {
    /// Loads the faces the profile names.
    pub fn new(profile: &Profile) -> Result<SubtitleMeter, GateError> {
        let s = &profile.subtitle;
        let text_bytes = read_font(&profile.resolve(&s.font))?;
        let text_size = motoreel_size(&text_bytes, s.size, s.size_unit)?;
        let mut fonts = Fonts::new();
        let face = |b: Vec<u8>, name: &str| {
            Face::load(b, name).map_err(|e| GateError::Profile(format!("{name}: {e}")))
        };
        let text = fonts.add(face(text_bytes, &s.font)?);
        let (formula, formula_size) = match &s.formula_font {
            Some(ff) => {
                let b = read_font(&profile.resolve(ff))?;
                let size = motoreel_size(&b, s.size * s.formula_scale, s.size_unit)?;
                (fonts.add(face(b, ff)?), size)
            }
            None => (text, text_size * s.formula_scale),
        };
        Ok(SubtitleMeter {
            fonts,
            text,
            formula,
            text_size,
            formula_size,
            pad: s.formula_pad * s.size,
            formula_words: s.formula_words,
        })
    }

    fn run(&self, s: &str, face: FaceId, size: f64) -> Result<f64, GateError> {
        measure(s, face, size, &self.fonts)
            .map(|n| n.metrics().width)
            .map_err(|e| GateError::Input(format!("«{s}»: {e}")))
    }

    fn units(&self, s: &str) -> Vec<Unit> {
        if self.formula_words {
            units(s, is_formula_word)
        } else {
            units(s, |_| false)
        }
    }

    /// The width of a subtitle set on one row, px.
    pub fn width(&self, s: &str) -> Result<f64, GateError> {
        let us = self.units(s);
        let mut widths = Vec::with_capacity(us.len());
        for u in &us {
            let mut w = 0.0;
            for p in u {
                w += match p {
                    Piece::Text(t) => self.run(t, self.text, self.text_size)?,
                    Piece::Formula(f) => self.run(f, self.formula, self.formula_size)? + self.pad,
                };
            }
            widths.push(w);
        }
        let space = self.run(" ", self.text, self.text_size)?;
        // as guion-reel lays a row out (`row_width`): units plus one space between each two
        Ok(widths.iter().sum::<f64>() + space * widths.len().saturating_sub(1) as f64)
    }

    /// The words to read (formulas do not count).
    pub fn words(&self, s: &str) -> usize {
        word_count(&self.units(s))
    }
}

/// The subtitle checks, one finding each.
pub fn check(
    narration: &Narration,
    profile: &Profile,
    meter: &SubtitleMeter,
) -> Result<Vec<Finding>, GateError> {
    let s = &profile.subtitle;
    let mut wide = Vec::new();
    let mut wordy = Vec::new();
    let mut signs = Vec::new();
    let mut long = Vec::new();
    let mut widest: f64 = 0.0;
    for l in &narration.lines {
        let w = meter.width(&l.subtitle)?;
        widest = widest.max(w);
        if w > s.max_width {
            wide.push(format!("{:.2} s «{}» {w:.0} px", l.t0, l.subtitle));
        }
        let n = meter.words(&l.subtitle);
        if n > s.max_words {
            wordy.push(format!("{:.2} s «{}» {n} words", l.t0, l.subtitle));
        }
        let bad: String = l
            .subtitle
            .chars()
            .filter(|c| (s.forbid_digits && c.is_ascii_digit()) || s.forbidden.contains(*c))
            .collect();
        if !bad.is_empty() {
            signs.push(format!("{:.2} s «{}» carries «{bad}»", l.t0, l.subtitle));
        }
        if l.t1 - l.t0 > s.long_line && l.sentences.len() > 1 && l.from == 1 {
            long.push(format!(
                "{:.2} s «{}» {:.1} s, {} sentences, no desde",
                l.t0,
                l.id,
                l.t1 - l.t0,
                l.sentences.len()
            ));
        }
    }
    let mut overlap = Vec::new();
    for w in narration.lines.windows(2) {
        if w[1].subtitle_start() < w[0].t1 + s.fade {
            overlap.push(format!(
                "{:.2} s «{}» enters before «{}» leaves",
                w[1].subtitle_start(),
                w[1].id,
                w[0].id
            ));
        }
    }
    let n = narration.lines.len();
    Ok(vec![
        Finding::from_misses(
            "sub-width",
            format!("{n} lines, widest {widest:.0} px ≤ {:.0}", s.max_width),
            format!("{} wider than {:.0} px", wide.len(), s.max_width),
            wide,
        ),
        Finding::from_misses(
            "sub-words",
            format!("all ≤ {} words", s.max_words),
            format!("{} over {} words", wordy.len(), s.max_words),
            wordy,
        ),
        Finding::from_misses(
            "sub-signs",
            "words only",
            format!("{} with digits or signs", signs.len()),
            signs,
        ),
        Finding::from_misses(
            "sub-desde",
            format!("lines over {:.0} s enter on their sentence", s.long_line),
            format!("{} long lines without desde", long.len()),
            long,
        ),
        Finding::from_misses(
            "sub-overlap",
            "one at a time",
            format!("{} overlaps", overlap.len()),
            overlap,
        ),
    ])
}
