//! The YouTube checks (R-0016): on what is uploaded, as it was written.
//!
//! - `srt`: the `.srt`'s cues are `voz.json`'s sentences, same times to the
//!   millisecond, same count (long YouTube video);
//! - `chapters`: the timestamps in the description, by YouTube's rules
//!   (guion-publish's [`validate_chapters`]) against the video's length;
//! - `yt-title` and `yt-description`: the platform's limits and the brand's
//!   internal patterns (a Short and a long video);
//! - `thumbnail`: 1280 × 720, at most 2 MB (long YouTube video).

use std::path::Path;

use guion_publish::youtube::{chapters_in, check_description, check_title, validate_chapters};

use super::caption::find_pattern;
use super::profile::Profile;
use super::report::{Finding, MAX_AT};
use crate::narration::Narration;
use crate::youtube::{check_thumbnail, cues, parse_srt, srt_time};

fn read(p: &Path) -> Result<String, String> {
    std::fs::read_to_string(p).map_err(|e| format!("{}: {e}", p.display()))
}

/// The `.srt` against `voz.json`.
pub fn srt(path: &Path, n: &Narration) -> Finding {
    let text = match read(path) {
        Ok(t) => t,
        Err(e) => return Finding::fail("srt", "no subtitles file", vec![e]),
    };
    let got = match parse_srt(&text) {
        Ok(c) => c,
        Err(e) => return Finding::fail("srt", "the .srt does not parse", vec![e]),
    };
    let want = cues(n);
    let mut bad = Vec::new();
    if got.len() != want.len() {
        bad.push(format!(
            "{} cues; voz.json has {} sentences",
            got.len(),
            want.len()
        ));
    }
    for (k, (g, w)) in got.iter().zip(&want).enumerate() {
        if (g.t0, g.t1) != (w.t0, w.t1) {
            bad.push(format!(
                "cue {}: {} --> {}, voz.json {} --> {}",
                k + 1,
                srt_time(g.t0),
                srt_time(g.t1),
                srt_time(w.t0),
                srt_time(w.t1)
            ));
        }
    }
    bad.truncate(MAX_AT);
    Finding::from_misses(
        "srt",
        format!("{} cues, timed as voz.json", got.len()),
        "the subtitles do not follow voz.json",
        bad,
    )
}

/// The chapters written in the description, against the video's length.
pub fn chapters(description: &Path, duration: f64) -> Finding {
    let text = match read(description) {
        Ok(t) => t,
        Err(e) => return Finding::fail("chapters", "no description", vec![e]),
    };
    let caps = chapters_in(&text);
    let starts: Vec<u64> = caps.iter().map(|(s, _)| *s).collect();
    Finding::from_misses(
        "chapters",
        format!(
            "{} chapters from 00:00, each ≥ 10 s of {duration:.1} s",
            caps.len()
        ),
        "YouTube would not show these chapters",
        validate_chapters(&starts, duration),
    )
}

fn internal(text: &str, p: &Profile) -> Vec<String> {
    p.caption
        .internal
        .iter()
        .filter_map(|pat| find_pattern(text, pat).map(|m| format!("«{m}» ({pat})")))
        .collect()
}

/// The title and the description, as written.
pub fn texts(title: &Path, description: &Path, p: &Profile) -> Vec<Finding> {
    let mut out = Vec::new();
    for (check, path, rules) in [
        ("yt-title", title, check_title as fn(&str) -> Vec<String>),
        ("yt-description", description, check_description),
    ] {
        out.push(match read(path) {
            Err(e) => Finding::fail(check, "missing", vec![e]),
            Ok(t) => {
                let t = t.strip_suffix('\n').unwrap_or(&t);
                let mut bad = rules(t);
                bad.extend(internal(t, p));
                Finding::from_misses(
                    check,
                    format!(
                        "{} characters, {} bytes, nothing internal",
                        t.chars().count(),
                        t.len()
                    ),
                    "YouTube would refuse it, or it shows something internal",
                    bad,
                )
            }
        });
    }
    out
}

/// The thumbnail's size and weight.
pub fn thumbnail(path: &Path) -> Finding {
    match check_thumbnail(path) {
        Ok(n) => Finding::pass("thumbnail", format!("1280×720, {n} bytes ≤ 2000000")),
        Err(e) => Finding::fail("thumbnail", "the thumbnail does not fit YouTube", vec![e]),
    }
}
