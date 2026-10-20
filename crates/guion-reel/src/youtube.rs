//! What a YouTube upload needs from a post besides its MP4 (R-0016): the
//! subtitles as `.srt`, timed by `voz.json`, and the thumbnail at 1280 × 720.
//!
//! - [`cues`] and [`srt`]: one cue per measured sentence of the voice, with
//!   what is said (captions are what the viewer hears; the short on-screen
//!   subtitle is a design element, not a transcript). The times are
//!   `voz.json`'s, rounded to the millisecond, so a re-synthesised voice
//!   moves the captions with it;
//! - [`parse_srt`]: reads an `.srt` back (the gate compares it with `voz.json`);
//! - [`resample`]: an area-weighted resize, deterministic (the same bytes in,
//!   the same bytes out), used for the thumbnail;
//! - [`png_size`] and [`check_thumbnail`]: the thumbnail's size and weight.
//!
//! The thumbnail's numbers are reported: YouTube Help, «Add video thumbnails»
//! (support.google.com/youtube/answer/72431): a resolution of 1280 × 720
//! (minimum width 640) and under 2 MB for videos.

use std::path::Path;

use crate::narration::Narration;

/// The thumbnail's width, px.
pub const THUMB_W: usize = 1280;
/// The thumbnail's height, px.
pub const THUMB_H: usize = 720;
/// The heaviest thumbnail, bytes: «under 2MB», read as 2 000 000 (the
/// strictest reading of a megabyte).
pub const THUMB_MAX_BYTES: u64 = 2_000_000;
/// The longest caption row, characters (a common broadcast convention; the
/// platform wraps longer rows itself, so this is a readability choice).
pub const CUE_ROW: usize = 42;

/// One subtitle cue: from `t0` to `t1`, in milliseconds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cue {
    pub t0: u64,
    pub t1: u64,
    pub text: String,
}

fn ms(t: f64) -> u64 {
    (t.max(0.0) * 1000.0).round() as u64
}

/// Words wrapped into rows of at most [`CUE_ROW`] characters (a longer word
/// keeps its own row).
fn wrap(s: &str) -> String {
    let mut rows: Vec<String> = Vec::new();
    for w in s.split_whitespace() {
        match rows.last_mut() {
            Some(r) if r.chars().count() + 1 + w.chars().count() <= CUE_ROW => {
                r.push(' ');
                r.push_str(w);
            }
            _ => rows.push(w.to_string()),
        }
    }
    rows.join("\n")
}

/// The cues of a narration: one per sentence, in order, with what is said.
pub fn cues(n: &Narration) -> Vec<Cue> {
    n.lines
        .iter()
        .flat_map(|l| l.sentences.iter())
        .filter(|s| !s.text.trim().is_empty())
        .map(|s| Cue {
            t0: ms(s.t0),
            t1: ms(s.t1),
            text: wrap(&s.text),
        })
        .collect()
}

/// `HH:MM:SS,mmm`.
pub fn srt_time(ms: u64) -> String {
    format!(
        "{:02}:{:02}:{:02},{:03}",
        ms / 3_600_000,
        ms / 60_000 % 60,
        ms / 1000 % 60,
        ms % 1000
    )
}

/// The `.srt` of a narration (UTF-8, `\n` line ends, a blank line after
/// each cue).
pub fn srt(n: &Narration) -> String {
    let mut s = String::new();
    for (i, c) in cues(n).iter().enumerate() {
        s.push_str(&format!(
            "{}\n{} --> {}\n{}\n\n",
            i + 1,
            srt_time(c.t0),
            srt_time(c.t1),
            c.text
        ));
    }
    s
}

fn parse_time(s: &str) -> Option<u64> {
    let (hms, milli) = s.trim().split_once([',', '.'])?;
    let p: Vec<&str> = hms.split(':').collect();
    let [h, m, sec] = p.as_slice() else {
        return None;
    };
    let n = |x: &str| x.parse::<u64>().ok();
    Some(n(h)? * 3_600_000 + n(m)? * 60_000 + n(sec)? * 1000 + n(milli)?)
}

/// Reads an `.srt`; the error names the cue that does not parse.
pub fn parse_srt(text: &str) -> Result<Vec<Cue>, String> {
    let text = text.trim_start_matches('\u{feff}').replace("\r\n", "\n");
    let mut out = Vec::new();
    for (k, block) in text
        .split("\n\n")
        .filter(|b| !b.trim().is_empty())
        .enumerate()
    {
        let mut lines = block.trim_matches('\n').lines();
        let _index = lines.next();
        let times = lines
            .next()
            .ok_or_else(|| format!("cue {}: no times", k + 1))?;
        let (a, b) = times
            .split_once("-->")
            .ok_or_else(|| format!("cue {}: «{times}» is not «start --> end»", k + 1))?;
        let (t0, t1) = match (parse_time(a), parse_time(b)) {
            (Some(x), Some(y)) => (x, y),
            _ => return Err(format!("cue {}: «{times}» does not parse", k + 1)),
        };
        out.push(Cue {
            t0,
            t1,
            text: lines.collect::<Vec<_>>().join("\n"),
        });
    }
    Ok(out)
}

/// For each output index along an axis, the source indices it covers and
/// their weights (the overlap of the pixel intervals, normalised).
fn taps(src: usize, dst: usize) -> Vec<Vec<(usize, f64)>> {
    let k = src as f64 / dst as f64;
    (0..dst)
        .map(|o| {
            let (a, b) = (o as f64 * k, (o + 1) as f64 * k);
            let mut v = Vec::new();
            let mut i = a.floor() as usize;
            while (i as f64) < b && i < src {
                let w = (b.min((i + 1) as f64) - a.max(i as f64)).max(0.0);
                if w > 0.0 {
                    v.push((i, w / (b - a)));
                }
                i += 1;
            }
            v
        })
        .collect()
}

/// RGB24 `w × h` resized to `ow × oh` by area weighting (a box filter that
/// also enlarges). Pure: the same pixels give the same bytes.
pub fn resample(rgb: &[u8], w: usize, h: usize, ow: usize, oh: usize) -> Vec<u8> {
    let (tx, ty) = (taps(w, ow), taps(h, oh));
    let mut mid = vec![0f64; ow * h * 3];
    for y in 0..h {
        for (x, t) in tx.iter().enumerate() {
            for c in 0..3 {
                mid[(y * ow + x) * 3 + c] = t
                    .iter()
                    .map(|(i, wt)| f64::from(rgb[(y * w + i) * 3 + c]) * wt)
                    .sum();
            }
        }
    }
    let mut out = vec![0u8; ow * oh * 3];
    for (y, t) in ty.iter().enumerate() {
        for x in 0..ow {
            for c in 0..3 {
                let v: f64 = t.iter().map(|(j, wt)| mid[(j * ow + x) * 3 + c] * wt).sum();
                out[(y * ow + x) * 3 + c] = v.round().clamp(0.0, 255.0) as u8;
            }
        }
    }
    out
}

/// A PNG's width and height, from its IHDR; `None` if it is not a PNG.
pub fn png_size(bytes: &[u8]) -> Option<(usize, usize)> {
    if bytes.len() < 24 || &bytes[..8] != b"\x89PNG\r\n\x1a\n" || &bytes[12..16] != b"IHDR" {
        return None;
    }
    let be = |i: usize| u32::from_be_bytes([bytes[i], bytes[i + 1], bytes[i + 2], bytes[i + 3]]);
    Some((be(16) as usize, be(20) as usize))
}

/// The thumbnail at `path`: a PNG of [`THUMB_W`] × [`THUMB_H`] and at most
/// [`THUMB_MAX_BYTES`]. `Ok(bytes)`, or what is wrong with it.
pub fn check_thumbnail(path: &Path) -> Result<u64, String> {
    let b = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let (w, h) = png_size(&b).ok_or_else(|| format!("{}: not a PNG", path.display()))?;
    let mut bad = Vec::new();
    if (w, h) != (THUMB_W, THUMB_H) {
        bad.push(format!("{w}×{h}, not {THUMB_W}×{THUMB_H}"));
    }
    let n = b.len() as u64;
    if n > THUMB_MAX_BYTES {
        bad.push(format!("{n} bytes > {THUMB_MAX_BYTES}"));
    }
    if bad.is_empty() {
        Ok(n)
    } else {
        Err(format!("{}: {}", path.display(), bad.join("; ")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn times_and_wrapping() {
        assert_eq!(srt_time(3_723_045), "01:02:03,045");
        assert_eq!(parse_time("01:02:03,045"), Some(3_723_045));
        let w = wrap("one two three four five six seven eight nine ten eleven twelve");
        assert!(w.lines().all(|r| r.chars().count() <= CUE_ROW), "{w}");
        assert_eq!(
            w.replace('\n', " "),
            "one two three four five six seven eight nine ten eleven twelve"
        );
    }

    #[test]
    fn resampling_keeps_flat_colour_and_area() {
        let flat = vec![200u8; 30 * 18 * 3];
        assert!(resample(&flat, 30, 18, 20, 12).iter().all(|&v| v == 200));
        // a 2×1 black-white pair averages to grey at 1×1, and enlarges to two
        let pair = [0, 0, 0, 255, 255, 255];
        assert_eq!(resample(&pair, 2, 1, 1, 1), vec![128, 128, 128]);
        assert_eq!(resample(&pair, 2, 1, 4, 1)[..3], [0, 0, 0]);
    }
}
