//! The cover framing (R-0017): where the content of a vertical reel's cover
//! frame sits against Instagram's crops.
//!
//! [`measure`] takes one classified frame and returns the rows its ink spans
//! (the same ink as the safe-area check: the brand's `safe.ink`, or every
//! token that is not background), leaving out the subtitle band (the cover
//! Instagram shows is the frame the owner picks, not the line being said),
//! the profile's `except` rectangles (watermark, level chip), and up to
//! `scan.min_stray − 1` stray pixels at each end (codec noise, as in the
//! safe-area check). [`check`] compares those rows with the format's
//! [`Encuadre`].

use super::pixels::NONE;
use super::profile::Profile;
use super::report::Finding;
use crate::formato::Encuadre;

/// The check's name in the report (`check.json`).
pub const CHECK: &str = "encuadre";

/// The rows a cover's content spans: `top` is its first row, `bottom` one
/// past its last (half-open, like the report's boxes).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rows {
    pub top: usize,
    pub bottom: usize,
}

impl Rows {
    /// The box's vertical centre, px.
    pub fn center(&self) -> f64 {
        (self.top + self.bottom) as f64 / 2.0
    }
}

/// The rows the ink of a classified `w × h` frame spans, as the module
/// docs describe; `None` when fewer than `scan.min_stray` ink pixels are
/// left (nothing to frame).
pub fn measure(class: &[u8], w: usize, h: usize, ink: &[bool; 256], p: &Profile) -> Option<Rows> {
    let band = (p.subtitle.band[0], p.subtitle.band[1]);
    let bleed = p.safe.bleed;
    let except: Vec<[f64; 4]> = p
        .safe
        .except
        .iter()
        .map(|r| [r[0] - bleed, r[1] - bleed, r[2] + bleed, r[3] + bleed])
        .collect();
    let counts: Vec<usize> = (0..h)
        .map(|y| {
            let yf = y as f64;
            if yf >= band.0 && yf < band.1 {
                return 0;
            }
            let ex: Vec<&[f64; 4]> = except.iter().filter(|r| yf >= r[1] && yf < r[3]).collect();
            class[y * w..(y + 1) * w]
                .iter()
                .enumerate()
                .filter(|&(x, &c)| {
                    c != NONE
                        && ink[c as usize]
                        && !ex.iter().any(|r| (x as f64) >= r[0] && (x as f64) < r[2])
                })
                .count()
        })
        .collect();
    let need = p.scan.min_stray.max(1);
    if counts.iter().sum::<usize>() < need {
        return None;
    }
    // the first row (from either end) where the ink seen so far reaches `need`
    fn first(rows: impl Iterator<Item = (usize, usize)>, need: usize) -> Option<usize> {
        let mut sum = 0;
        rows.into_iter()
            .find(|&(_, n)| {
                sum += n;
                sum >= need
            })
            .map(|(y, _)| y)
    }
    let top = first(counts.iter().copied().enumerate(), need)?;
    let last = first(counts.iter().copied().enumerate().rev(), need)?;
    Some(Rows {
        top,
        bottom: last + 1,
    })
}

/// The finding for the content `rows` of the cover frame at `at` (a time
/// and frame, for the report) against `e`. It fails when nothing was
/// measured, when the top is above `min_top`, the bottom below
/// `max_bottom`, or the centre further than `max_offset` from `center_y`;
/// every failure names what was measured and the limit.
pub fn check(rows: Option<Rows>, e: &Encuadre, at: &str) -> Finding {
    let Some(r) = rows else {
        return Finding::fail(
            CHECK,
            format!(
                "nothing on the cover frame at {at}: no content to frame ([reel] cover: a frame with the title in)"
            ),
            vec![at.to_string()],
        );
    };
    let c = r.center();
    let off = c - e.center_y;
    let measured = format!(
        "cover at {at}: content y {}–{}, centre {c:.0} (top ≥ {:.0}, bottom ≤ {:.0}, centre {:.0} ± {:.0})",
        r.top, r.bottom, e.min_top, e.max_bottom, e.center_y, e.max_offset
    );
    let mut bad = Vec::new();
    if (r.top as f64) < e.min_top {
        bad.push(format!(
            "top y {} < {:.0} (above the 4:5 feed crop)",
            r.top, e.min_top
        ));
    }
    if (r.bottom as f64) > e.max_bottom {
        bad.push(format!(
            "bottom y {} > {:.0} (under the Reels UI)",
            r.bottom, e.max_bottom
        ));
    }
    if off.abs() > e.max_offset {
        bad.push(format!(
            "centre y {c:.0} is {off:+.0} from {:.0} (> ± {:.0}, off-centre in the 3:4 grid crop)",
            e.center_y, e.max_offset
        ));
    }
    Finding::from_misses(CHECK, measured.clone(), measured, bad)
}

/// The finding for a horizontal format: not checked (R-0017 AC8).
pub fn skipped(formato: crate::Formato) -> Finding {
    Finding::pass(
        CHECK,
        format!("not checked: {formato} is horizontal (no grid crop)"),
    )
}
