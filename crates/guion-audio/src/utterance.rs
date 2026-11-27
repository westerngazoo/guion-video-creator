//! Narration from takes: blocks of lines spoken as one utterance (R-0019).
//!
//! R-0007's [`crate::narrate_lines`] speaks every line on its own and finds
//! its sentences by energy. With prosody noise both break: a voice that
//! breathes between lines resets its intonation on each, and a pause after a
//! comma can come out longer than the one between sentences, so "the longest
//! pauses" are no longer the sentence cuts.
//!
//! Here a block of consecutive lines is **one utterance**, spoken once and
//! kept as a take ([`crate::TakeStore`]). Its sentences are cut where the
//! voice wrote exact zeros between them ([`crate::Voice`]'s contract: Piper
//! writes `int(sentence_silence × rate)` zero samples there, and nowhere else
//! is speech a run of zeros), so the cuts are exact whatever the noise. Each
//! line keeps its own start and end on the clock, and the pause between two
//! lines of a block is the voice's own, plus the line's `after` and the next
//! one's `before`. Between blocks, the pacing's `gap`, as in R-0007.

use std::ops::Range;

use crate::engine::resample_pcm16;
use crate::error::AudioError;
use crate::lines::TimedLine;
use crate::lines::{silence, split_sentences, validate, Line, Narration, Pacing, Sentence};
use crate::pcm::Pcm16;
use crate::take::{TakeStore, TakeUse, Voice};

/// Consecutive lines spoken by one voice as one utterance.
pub struct Utterance<'a> {
    /// Indices into the narration's lines: consecutive, never empty.
    pub lines: Range<usize>,
    pub voice: &'a dyn Voice,
}

/// What a narration from takes produced.
#[derive(Debug, Clone, PartialEq)]
pub struct TakeNarration {
    pub narration: Narration,
    /// One per utterance, in order.
    pub takes: Vec<TakeUse>,
}

impl TakeNarration {
    /// How many takes were spoken in this run (not read from the store).
    pub fn spoken(&self) -> usize {
        self.takes.iter().filter(|t| !t.cached).count()
    }
}

/// Speaks `lines` as `utterances`, each one take: read from `store` when it
/// holds the key, else spoken by its voice and saved there.
///
/// `spoken[i]` is what the voice says for line `i` (its text after a
/// pronunciation lexicon, R-0019 AC11); `lines[i].text` is what is written,
/// and the sentences returned carry the written text with the voice's times.
///
/// Fails, before anything is spoken, when the utterances do not cover the
/// lines in order, a line is empty or has a negative pause (R-0007 AC6), a
/// line inside a block does not end a sentence (AC7), or the written and the
/// spoken text cut into different sentences. Fails after speaking, with
/// nothing returned, when a take's zero runs are not its sentence cuts
/// (AC6). Takes spoken before a failure stay in the store.
pub fn narrate_utterances(
    lines: &[Line],
    spoken: &[String],
    utterances: &[Utterance],
    pacing: &Pacing,
    store: &TakeStore,
) -> Result<TakeNarration, AudioError> {
    validate(lines, pacing)?;
    if spoken.len() != lines.len() {
        return Err(AudioError::InvalidScript(format!(
            "{} lines and {} spoken texts",
            lines.len(),
            spoken.len()
        )));
    }
    let mut next = 0;
    for u in utterances {
        if u.lines.start != next || u.lines.is_empty() {
            return Err(AudioError::InvalidScript(format!(
                "the blocks must cover the lines in order: one starts at line {}, \
                 the next expected is {next}",
                u.lines.start
            )));
        }
        next = u.lines.end;
    }
    if next != lines.len() {
        return Err(AudioError::InvalidScript(format!(
            "the blocks cover {next} of {} lines",
            lines.len()
        )));
    }
    let mut counts = Vec::with_capacity(lines.len());
    for (l, s) in lines.iter().zip(spoken) {
        if s.trim().is_empty() {
            return Err(AudioError::InvalidLine {
                line: l.id.clone(),
                message: "la voz no dice nada (el léxico dejó el texto vacío)".into(),
            });
        }
        let (w, v) = (split_sentences(&l.text).len(), split_sentences(s).len());
        if w != v {
            return Err(AudioError::InvalidLine {
                line: l.id.clone(),
                message: format!(
                    "lo escrito tiene {w} frases y lo que dice la voz {v}: el léxico no debe \
                     mover un corte de frase"
                ),
            });
        }
        counts.push(v);
    }
    for u in utterances {
        for i in u.lines.start..u.lines.end - 1 {
            if !ends_sentence(&spoken[i]) {
                return Err(AudioError::InvalidLine {
                    line: lines[i].id.clone(),
                    message: format!(
                        "termina a media frase («{}») dentro de un bloque: una línea que sigue \
                         en el mismo bloque tiene que cerrar su frase con . ? o ! (o empieza un \
                         bloque nuevo después de ella)",
                        tail(&spoken[i])
                    ),
                });
            }
        }
    }

    let sr = pacing.sample_rate;
    let mut track: Vec<i16> = Vec::new();
    silence(&mut track, pacing.lead_in, sr);
    let mut out: Vec<TimedLine> = Vec::with_capacity(lines.len());
    let mut takes = Vec::with_capacity(utterances.len());
    for (k, u) in utterances.iter().enumerate() {
        let text = utterance_text(&spoken[u.lines.clone()]);
        let identity = u.voice.identity()?;
        let key = TakeStore::key(&identity, &text);
        let (take, cached) = match store.load(&key)? {
            Some(t) => (t, true),
            None => {
                let t = u.voice.take(&text)?;
                if t.channels != 1 || t.samples.is_empty() {
                    return Err(AudioError::Engine(format!(
                        "the voice returned {} channels and {} samples for «{}»",
                        t.channels,
                        t.samples.len(),
                        tail(&text)
                    )));
                }
                store.save(&key, &t)?;
                (t, false)
            }
        };
        let ids: Vec<String> = lines[u.lines.clone()]
            .iter()
            .map(|l| l.id.clone())
            .collect();
        let cuts = cut_take(&take, &counts[u.lines.clone()], u.voice.sentence_silence()).map_err(
            |(expected, found)| AudioError::TakeTiming {
                lines: ids.join(", "),
                expected,
                found,
                key: key.clone(),
            },
        )?;
        let native = take.sample_rate as f64;
        let last_utterance = k + 1 == utterances.len();
        for (j, (i, cut)) in u.lines.clone().zip(&cuts).enumerate() {
            let l = &lines[i];
            silence(&mut track, l.before, sr);
            let t0 = track.len() as f64 / sr as f64;
            let segment = Pcm16 {
                sample_rate: take.sample_rate,
                channels: 1,
                samples: take.samples[cut.speech.clone()].to_vec(),
            };
            track.extend_from_slice(&resample_pcm16(&segment, sr)?);
            let t1 = track.len() as f64 / sr as f64;
            let written = split_sentences(&l.text);
            let n = written.len();
            let sentences = written
                .into_iter()
                .zip(&cut.sentences)
                .enumerate()
                .map(|(s, (text, r))| Sentence {
                    text,
                    t0: t0 + (r.start - cut.speech.start) as f64 / native,
                    t1: if s + 1 == n {
                        t1
                    } else {
                        t0 + (r.end - cut.speech.start) as f64 / native
                    },
                })
                .collect();
            out.push(TimedLine {
                id: l.id.clone(),
                text: l.text.clone(),
                t0,
                t1,
                sentences,
            });
            let last_in_block = j + 1 == cuts.len();
            let gap = if !last_in_block {
                cut.pause_after as f64 / native
            } else if last_utterance {
                0.0
            } else {
                pacing.gap
            };
            silence(&mut track, l.after + gap, sr);
        }
        takes.push(TakeUse {
            key,
            cached,
            lines: ids,
            identity,
            text,
        });
    }
    Ok(TakeNarration {
        narration: Narration {
            sample_rate: sr,
            samples: track,
            lines: out,
        },
        takes,
    })
}

/// The lines of a block, said as one text: Piper reads one input line as one
/// utterance, so no newline may survive inside it.
fn utterance_text(spoken: &[String]) -> String {
    spoken
        .iter()
        .flat_map(|s| s.split_whitespace())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Whether a line closes its sentence: its last mark, before any closing
/// quote or bracket, is `.`, `?` or `!`.
fn ends_sentence(text: &str) -> bool {
    text.trim_end()
        .trim_end_matches(['»', '"', '”', '’', '\'', ')', ']'])
        .ends_with(['.', '?', '!'])
}

fn tail(text: &str) -> String {
    let t = text.trim();
    let n = t.chars().count();
    if n <= 40 {
        t.to_string()
    } else {
        format!("…{}", t.chars().skip(n - 40).collect::<String>())
    }
}

/// Where a line falls inside its take, in the take's samples.
#[derive(Debug, Clone, PartialEq)]
pub struct LineCut {
    /// The line's speech.
    pub speech: Range<usize>,
    /// Each of its sentences.
    pub sentences: Vec<Range<usize>>,
    /// The zero samples between this line and the next of the block (0 for
    /// the last).
    pub pause_after: usize,
}

/// Cuts a take into its lines: `counts[i]` sentences each. The cuts are the
/// take's internal runs of exact zeros at least `0.9 × sentence_silence ×
/// rate` long; there must be exactly `Σ counts − 1` of them. `Err((expected,
/// found))` otherwise.
pub fn cut_take(
    take: &Pcm16,
    counts: &[usize],
    sentence_silence: f64,
) -> Result<Vec<LineCut>, (usize, usize)> {
    let s = &take.samples;
    let expected = counts.iter().sum::<usize>().saturating_sub(1);
    // 90 %: the C++ build and the PyPI one round the silence differently
    let min = ((0.9 * sentence_silence * take.sample_rate as f64) as usize).max(1);
    let mut runs: Vec<Range<usize>> = Vec::new();
    let mut k = 0;
    while k < s.len() {
        if s[k] == 0 {
            let mut j = k;
            while j < s.len() && s[j] == 0 {
                j += 1;
            }
            if k > 0 && j < s.len() && j - k >= min {
                runs.push(k..j);
            }
            k = j;
        } else {
            k += 1;
        }
    }
    if runs.len() != expected || (expected > 0 && sentence_silence <= 0.0) {
        return Err((expected, runs.len()));
    }
    // sentence ranges over the whole take, then grouped by line
    let mut edges = vec![0usize];
    for r in &runs {
        edges.push(r.start);
        edges.push(r.end);
    }
    edges.push(s.len());
    let all: Vec<Range<usize>> = edges.chunks(2).map(|e| e[0]..e[1]).collect();
    let mut out = Vec::with_capacity(counts.len());
    let mut at = 0;
    for (i, &n) in counts.iter().enumerate() {
        let sentences = all[at..at + n].to_vec();
        let pause_after = if i + 1 < counts.len() {
            runs[at + n - 1].len()
        } else {
            0
        };
        out.push(LineCut {
            speech: sentences[0].start..sentences[n - 1].end,
            sentences,
            pause_after,
        });
        at += n;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_sentence_ends_before_its_closing_quote() {
        assert!(ends_sentence("Dijo «sí.»"));
        assert!(ends_sentence("¿Y la palabra? "));
        assert!(!ends_sentence("lo mismo en los dos:"));
        assert!(!ends_sentence("Número…"));
    }

    #[test]
    fn newlines_never_reach_the_voice() {
        assert_eq!(
            utterance_text(&["Uno.\nDos.".into(), " Tres. ".into()]),
            "Uno. Dos. Tres."
        );
    }
}
