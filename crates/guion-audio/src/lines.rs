//! Narration from a cue list: the voice sets the clock (R-0007).
//!
//! A screenplay fixes each cue's window and fits the speech into it
//! ([`crate::fit_to_window`]). A narrated reel works the other way round: the
//! lines are spoken one after another with fixed pauses, and **their measured
//! durations are the timeline** that the picture and the sound then follow.
//! [`narrate_lines`] returns the track and, for every line, when it starts and
//! ends and when each of its sentences does.
//!
//! **Sentence times are measured, not estimated.** Piper leaves
//! `sentence_silence` seconds after every sentence. Inside a line of *n*
//! sentences, the *n − 1* longest internal silences ([`SilenceRule`]) are the
//! cuts; if there are fewer, the line fails ([`AudioError::SentenceTiming`])
//! instead of guessing.
//!
//! Everything here is integer sample arithmetic on 16-bit PCM, in a fixed
//! order, so the same engine output gives the same times and the same bytes
//! (AC1–AC3). The arithmetic matches the Python tool it replaces in the
//! rotorf-sico studio (`tools/narra.py`) operation for operation, so a
//! studio can switch and compare files with `md5sum` (AC4).

use crate::engine::NarrationEngine;
use crate::error::AudioError;
use crate::pcm::write_wav_pcm16;

/// One line to speak.
#[derive(Debug, Clone, PartialEq)]
pub struct Line {
    /// A stable name the scene refers to (`"gancho"`, `"qed"`).
    pub id: String,
    /// What is said.
    pub text: String,
    /// Extra silence before the line, in seconds (on top of the pacing gap).
    pub before: f64,
    /// Extra silence after the line, in seconds.
    pub after: f64,
}

impl Line {
    pub fn new(id: impl Into<String>, text: impl Into<String>) -> Self {
        Line {
            id: id.into(),
            text: text.into(),
            before: 0.0,
            after: 0.0,
        }
    }
}

/// How the lines are laid out in time.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pacing {
    /// Output sample rate (the studio's is 48 000).
    pub sample_rate: u32,
    /// Silence before the first line, in seconds.
    pub lead_in: f64,
    /// Silence between consecutive lines, in seconds (not after the last).
    pub gap: f64,
}

/// What counts as a pause between sentences.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SilenceRule {
    /// Analysis window, in seconds.
    pub window: f64,
    /// A window is quiet when its RMS is below this, in 16-bit units.
    pub threshold: f64,
    /// A pause is at least this long, in seconds.
    pub min_gap: f64,
}

impl SilenceRule {
    /// For Piper with `sentence_silence = 0.25`: 10 ms windows, quiet below
    /// 131 (−48 dBFS), pauses of 0.25 s or more. On the studio's voice
    /// (`en-gb-alan-low`) the pauses between sentences measured 0.36–0.44 s.
    /// A pause inside a sentence (a comma, a colon) only competes when it is
    /// longer than one between sentences; the studio's three narrated reels
    /// have none.
    pub const PIPER: SilenceRule = SilenceRule {
        window: 0.01,
        threshold: 131.0,
        min_gap: 0.25,
    };
}

/// A sentence of a line, with its measured times (seconds on the track).
#[derive(Debug, Clone, PartialEq)]
pub struct Sentence {
    pub text: String,
    pub t0: f64,
    pub t1: f64,
}

/// A spoken line, placed on the track.
#[derive(Debug, Clone, PartialEq)]
pub struct TimedLine {
    pub id: String,
    pub text: String,
    pub t0: f64,
    pub t1: f64,
    pub sentences: Vec<Sentence>,
}

/// The whole narration: mono 16-bit samples and where every line fell.
#[derive(Debug, Clone, PartialEq)]
pub struct Narration {
    pub sample_rate: u32,
    pub samples: Vec<i16>,
    pub lines: Vec<TimedLine>,
}

impl Narration {
    pub fn duration(&self) -> f64 {
        self.samples.len() as f64 / self.sample_rate as f64
    }

    /// Mono 16-bit WAV, canonical header, no partial output.
    pub fn write_wav(&self, path: &std::path::Path) -> Result<(), AudioError> {
        write_wav_pcm16(path, self.sample_rate, &self.samples)
    }
}

/// Speaks `lines` in order with `engine` and lays them out with `pacing`.
///
/// Fails, writing nothing, on a line with empty text or a negative pause
/// (AC6), or whose sentences cannot be timed (AC6).
pub fn narrate_lines(
    lines: &[Line],
    pacing: &Pacing,
    rule: &SilenceRule,
    engine: &dyn NarrationEngine,
) -> Result<Narration, AudioError> {
    for l in lines {
        if l.text.trim().is_empty() {
            return Err(invalid(l, "el texto está vacío"));
        }
        for (name, v) in [("before", l.before), ("after", l.after)] {
            if !(v.is_finite() && v >= 0.0) {
                return Err(invalid(l, &format!("{name} = {v}: debe ser ≥ 0")));
            }
        }
    }
    if !(pacing.lead_in.is_finite()
        && pacing.lead_in >= 0.0
        && pacing.gap.is_finite()
        && pacing.gap >= 0.0)
    {
        return Err(AudioError::InvalidScript(format!(
            "pacing: lead_in = {}, gap = {}: deben ser ≥ 0",
            pacing.lead_in, pacing.gap
        )));
    }
    let sr = pacing.sample_rate;
    let mut track: Vec<i16> = Vec::new();
    silence(&mut track, pacing.lead_in, sr);
    let mut out = Vec::with_capacity(lines.len());
    for (i, l) in lines.iter().enumerate() {
        silence(&mut track, l.before, sr);
        let t0 = track.len() as f64 / sr as f64;
        let speech = engine.synthesize_pcm16(&l.text, sr)?;
        track.extend_from_slice(&speech);
        let t1 = track.len() as f64 / sr as f64;
        let sentences = sentence_times(&l.text, &speech, sr, t0, rule).map_err(|e| match e {
            AudioError::SentenceTiming {
                sentences,
                gaps,
                min_gap,
                ..
            } => AudioError::SentenceTiming {
                line: l.id.clone(),
                sentences,
                gaps,
                min_gap,
            },
            other => other,
        })?;
        out.push(TimedLine {
            id: l.id.clone(),
            text: l.text.clone(),
            t0,
            t1,
            sentences,
        });
        let last = i + 1 == lines.len();
        silence(
            &mut track,
            l.after + if last { 0.0 } else { pacing.gap },
            sr,
        );
    }
    Ok(Narration {
        sample_rate: sr,
        samples: track,
        lines: out,
    })
}

fn invalid(l: &Line, message: &str) -> AudioError {
    AudioError::InvalidLine {
        line: l.id.clone(),
        message: message.into(),
    }
}

/// `secs` of silence, rounded to the nearest sample (ties to even).
fn silence(track: &mut Vec<i16>, secs: f64, sr: u32) {
    let n = (secs * sr as f64).round_ties_even() as usize;
    track.resize(track.len() + n, 0);
}

/// The text cut after `.`, `?` or `!` followed by whitespace; the whitespace
/// goes, the punctuation stays. An abbreviation followed by a space (“e.g. x”)
/// also cuts, as in the tool this replaces: spell it out in spoken lines.
pub fn split_sentences(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut chars = text.trim().chars().peekable();
    while let Some(c) = chars.next() {
        cur.push(c);
        if matches!(c, '.' | '?' | '!') && chars.peek().is_some_and(|n| n.is_whitespace()) {
            while chars.peek().is_some_and(|n| n.is_whitespace()) {
                chars.next();
            }
            out.push(std::mem::take(&mut cur));
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// The sentences of one spoken line, with times on the track (`t0` is where
/// the line starts). The *n − 1* longest internal pauses are the cuts; each
/// sentence runs from the end of the pause before it to the start of the
/// pause after it, and the last one to the end of the line.
pub fn sentence_times(
    text: &str,
    speech: &[i16],
    sample_rate: u32,
    t0: f64,
    rule: &SilenceRule,
) -> Result<Vec<Sentence>, AudioError> {
    let parts = split_sentences(text);
    let hop = (sample_rate as f64 * rule.window).round() as usize;
    let min_windows = (rule.min_gap / rule.window).round() as usize;
    let thr2 = rule.threshold * rule.threshold;
    let windows = speech.len() / hop.max(1);
    let quiet: Vec<bool> = (0..windows)
        .map(|k| {
            let sum: i64 = speech[k * hop..(k + 1) * hop]
                .iter()
                .map(|&s| s as i64 * s as i64)
                .sum();
            (sum as f64) / (hop as f64) < thr2
        })
        .collect();
    // internal pauses: not touching the start or the end of the line
    let mut gaps: Vec<(usize, usize)> = Vec::new();
    let mut k = 0;
    while k < quiet.len() {
        if quiet[k] {
            let mut j = k;
            while j < quiet.len() && quiet[j] {
                j += 1;
            }
            if k > 0 && j < quiet.len() && j - k >= min_windows {
                gaps.push((k, j));
            }
            k = j;
        } else {
            k += 1;
        }
    }
    let n = parts.len();
    if gaps.len() + 1 < n {
        return Err(AudioError::SentenceTiming {
            line: String::new(),
            sentences: n,
            gaps: gaps.len(),
            min_gap: rule.min_gap,
        });
    }
    // the n − 1 longest (stable: on a tie, the earlier one), back in order
    gaps.sort_by_key(|&(a, b)| a as i64 - b as i64);
    let mut cuts: Vec<(usize, usize)> = gaps.into_iter().take(n.saturating_sub(1)).collect();
    cuts.sort();
    let mut edges = vec![0usize];
    for (a, b) in &cuts {
        edges.push(*a);
        edges.push(*b);
    }
    edges.push(windows);
    let sr = sample_rate as f64;
    let dur = speech.len() as f64 / sr;
    Ok(parts
        .into_iter()
        .enumerate()
        .map(|(i, text)| {
            let a = (edges[2 * i] * hop) as f64 / sr;
            let b = dur.min((edges[2 * i + 1] * hop) as f64 / sr);
            Sentence {
                text,
                t0: t0 + a,
                t1: t0 + if i + 1 == n { dur } else { b },
            }
        })
        .collect())
}
