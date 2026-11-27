//! R-0019: the natural voice, spoken once and kept. A fake voice whose takes
//! change on every call (as Piper's do with noise) shows that the narration
//! only depends on the takes kept.

use std::cell::Cell;
use std::path::PathBuf;

use guion_audio::{
    cut_take, narrate_utterances, sentence_times, split_sentences, AudioError, Line, Pacing, Pcm16,
    SilenceRule, TakeStore, Utterance, Voice, TAKE_INDEX,
};

const SR: u32 = 48_000;
const SILENCE: f64 = 0.35;

/// Like Piper with noise: every word 0.2 s of a square wave whose level
/// changes on every call, a comma leaves 1.0 s of quiet *noise* (not zeros:
/// a long, low breath, as `es-carlfm-x-low` did at «C»), and between two
/// sentences exactly `SILENCE × rate` zero samples.
struct Noisy {
    calls: Cell<usize>,
    identity: String,
}

impl Noisy {
    fn new(identity: &str) -> Self {
        Noisy {
            calls: Cell::new(0),
            identity: identity.into(),
        }
    }
}

impl Voice for Noisy {
    fn identity(&self) -> Result<String, AudioError> {
        Ok(self.identity.clone())
    }
    fn take(&self, text: &str) -> Result<Pcm16, AudioError> {
        let call = self.calls.get() + 1;
        self.calls.set(call);
        let level = 8000 + 37 * call as i16;
        let mut out = Vec::new();
        let parts = split_sentences(text);
        for (i, s) in parts.iter().enumerate() {
            for w in s.split_whitespace() {
                for k in 0..(SR / 5) as usize {
                    out.push(if (k / 24) % 2 == 0 { level } else { -level });
                }
                if w.ends_with(',') {
                    for k in 0..SR as usize {
                        out.push(if k % 2 == 0 { 3 } else { -3 });
                    }
                }
            }
            if i + 1 < parts.len() {
                out.resize(out.len() + (SILENCE * SR as f64) as usize, 0);
            }
        }
        Ok(Pcm16 {
            sample_rate: SR,
            channels: 1,
            samples: out,
        })
    }
    fn sentence_silence(&self) -> f64 {
        SILENCE
    }
}

fn pacing() -> Pacing {
    Pacing {
        sample_rate: SR,
        lead_in: 0.5,
        gap: 0.4,
    }
}

fn store(name: &str) -> TakeStore {
    let d: PathBuf =
        std::env::temp_dir().join(format!("guion-r0019-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    TakeStore::new(d)
}

fn spoken(lines: &[Line]) -> Vec<String> {
    lines.iter().map(|l| l.text.clone()).collect()
}

fn one_per_line<'a>(lines: &[Line], v: &'a dyn Voice) -> Vec<Utterance<'a>> {
    (0..lines.len())
        .map(|i| Utterance {
            lines: i..i + 1,
            voice: v,
        })
        .collect()
}

/// AC4: the first run speaks every take; the second speaks none and gives
/// the same samples and times, although the voice would now say it
/// differently. A new identity (another model, build or prosody) speaks
/// again.
#[test]
fn ac4_spoken_once_then_read_back() {
    let lines = [
        Line::new("a", "Uno dos. Tres, cuatro."),
        Line::new("b", "Cinco."),
    ];
    let s = store("ac4");
    let v = Noisy::new("voz-1");
    let first = narrate_utterances(
        &lines,
        &spoken(&lines),
        &one_per_line(&lines, &v),
        &pacing(),
        &s,
    )
    .unwrap();
    assert_eq!((first.spoken(), v.calls.get()), (2, 2));
    let again = narrate_utterances(
        &lines,
        &spoken(&lines),
        &one_per_line(&lines, &v),
        &pacing(),
        &s,
    )
    .unwrap();
    assert_eq!(
        (again.spoken(), v.calls.get()),
        (0, 2),
        "nothing spoken twice"
    );
    assert_eq!(again.narration, first.narration);
    // what the voice would say now is different: only the kept take decides
    assert_ne!(
        v.take("Cinco.").unwrap().samples[0],
        first.narration.samples[24_000]
    );

    let other = Noisy::new("voz-2");
    let r = narrate_utterances(
        &lines,
        &spoken(&lines),
        &one_per_line(&lines, &other),
        &pacing(),
        &s,
    )
    .unwrap();
    assert_eq!(r.spoken(), 2, "another identity, other takes");
    // the pauses are not part of a take: changing them speaks nothing
    let mut paused = lines.clone();
    paused[1].before = 0.7;
    let p = narrate_utterances(
        &paused,
        &spoken(&paused),
        &one_per_line(&paused, &v),
        &pacing(),
        &s,
    )
    .unwrap();
    assert_eq!(p.spoken(), 0);
    let _ = std::fs::remove_dir_all(s.dir());
}

/// AC5: settling writes the index (no dates: the same takes, the same bytes)
/// and the `.gitignore` that lets an account commit its takes, and removes
/// the takes no line uses.
#[test]
fn ac5_the_store_is_kept_with_the_post() {
    let s = store("ac5");
    let v = Noisy::new("voz");
    let lines = [Line::new("a", "Uno."), Line::new("b", "Dos.")];
    let r = narrate_utterances(
        &lines,
        &spoken(&lines),
        &one_per_line(&lines, &v),
        &pacing(),
        &s,
    )
    .unwrap();
    assert!(s.settle(&r.takes).unwrap().is_empty());
    let index = std::fs::read_to_string(s.dir().join(TAKE_INDEX)).unwrap();
    assert!(
        index.contains(&format!("{}.wav  a", r.takes[0].key)),
        "{index}"
    );
    assert!(index.contains("text: Dos."), "{index}");
    let ignore = std::fs::read_to_string(s.dir().join(".gitignore")).unwrap();
    assert!(ignore.lines().any(|l| l == "!*.wav"), "{ignore}");
    // line b now says something else: its old take goes, a's stays
    let edited = [Line::new("a", "Uno."), Line::new("b", "Dos y tres.")];
    let r2 = narrate_utterances(
        &edited,
        &spoken(&edited),
        &one_per_line(&edited, &v),
        &pacing(),
        &s,
    )
    .unwrap();
    assert_eq!(r2.spoken(), 1);
    assert_eq!(s.settle(&r2.takes).unwrap(), vec![r.takes[1].key.clone()]);
    assert!(s.dir().join(format!("{}.wav", r.takes[0].key)).is_file());
    assert_eq!(
        std::fs::read_to_string(s.dir().join(TAKE_INDEX)).unwrap(),
        std::fs::read_to_string(s.dir().join(TAKE_INDEX)).unwrap()
    );
    let _ = std::fs::remove_dir_all(s.dir());
}

/// AC6: the sentences are cut where the voice wrote zeros, not at the
/// longest quiet stretch: here a comma's 1 s breath is longer than the
/// 0.35 s between sentences, and R-0007's energy rule cuts at the comma.
#[test]
fn ac6_sentences_are_cut_at_the_zeros() {
    let lines = [Line::new("a", "Uno, dos. Tres.")];
    let s = store("ac6");
    let v = Noisy::new("voz");
    let r = narrate_utterances(
        &lines,
        &spoken(&lines),
        &one_per_line(&lines, &v),
        &pacing(),
        &s,
    )
    .unwrap();
    let l = &r.narration.lines[0];
    let e = 1e-9;
    // «Uno,» 0.2 s + 1.0 s of breath + «dos.» 0.2 s; then 0.35 s of zeros
    assert!((l.t0 - 0.5).abs() < e, "{l:?}");
    assert!((l.sentences[0].t1 - (0.5 + 1.4)).abs() < e, "{l:?}");
    assert!((l.sentences[1].t0 - (0.5 + 1.75)).abs() < e, "{l:?}");
    assert_eq!(l.sentences[1].t1, l.t1);
    assert_eq!(l.sentences[0].text, "Uno, dos.");

    // the energy rule of R-0007 would cut inside the first sentence
    let take = Noisy::new("x").take("Uno, dos. Tres.").unwrap();
    let energy = sentence_times(
        "Uno, dos. Tres.",
        &take.samples,
        SR,
        0.0,
        &SilenceRule::PIPER,
    )
    .unwrap();
    assert!(energy[0].t1 < 0.3, "the comma wins by energy: {energy:?}");
    let _ = std::fs::remove_dir_all(s.dir());
}

/// AC6: a take whose zero runs are not its sentences fails, naming the line
/// and both counts, and returns nothing.
#[test]
fn ac6_a_take_that_does_not_cut_fails_by_name() {
    // «3.5 kilos.» is one sentence for guion; a voice that cut after «3.»
    // would leave one silence too many
    let take = Noisy::new("x").take("Tres. Cinco kilos.").unwrap();
    assert_eq!(cut_take(&take, &[1], SILENCE), Err((0, 1)));
    assert!(cut_take(&take, &[2], SILENCE).is_ok());

    struct Cuts(Noisy);
    impl Voice for Cuts {
        fn identity(&self) -> Result<String, AudioError> {
            self.0.identity()
        }
        fn take(&self, _: &str) -> Result<Pcm16, AudioError> {
            self.0.take("Tres. Cinco kilos.")
        }
        fn sentence_silence(&self) -> f64 {
            SILENCE
        }
    }
    let lines = [Line::new("peso", "Tres punto cinco kilos.")];
    let s = store("ac6b");
    let v = Cuts(Noisy::new("voz"));
    let e = narrate_utterances(
        &lines,
        &spoken(&lines),
        &one_per_line(&lines, &v),
        &pacing(),
        &s,
    )
    .unwrap_err();
    let m = e.to_string();
    assert!(
        m.contains("«peso»") && m.contains("0 silencios") && m.contains("dejó 1"),
        "{m}"
    );
    let _ = std::fs::remove_dir_all(s.dir());
}

/// AC7, AC8: a block is one take; each line keeps its start and end; inside
/// the block the pause is the voice's own plus the line's `after` and the
/// next line's `before`; the global gap only separates blocks.
#[test]
fn ac7_a_block_is_one_take_with_each_line_on_the_clock() {
    let mut lines = vec![
        Line::new("a", "Uno dos."),
        Line::new("b", "Tres."),
        Line::new("c", "Cuatro."),
    ];
    lines[1].before = 0.1;
    lines[0].after = 0.2;
    let s = store("ac7");
    let v = Noisy::new("voz");
    let blocks = [
        Utterance {
            lines: 0..2,
            voice: &v,
        },
        Utterance {
            lines: 2..3,
            voice: &v,
        },
    ];
    let r = narrate_utterances(&lines, &spoken(&lines), &blocks, &pacing(), &s).unwrap();
    assert_eq!(v.calls.get(), 2, "two blocks, two takes");
    assert_eq!(r.takes[0].text, "Uno dos. Tres.");
    assert_eq!(r.takes[0].lines, ["a", "b"]);
    let smp = |secs: f64| (secs * SR as f64).round_ties_even() as usize;
    let n = &r.narration.lines;
    let a0 = smp(0.5);
    let a1 = a0 + smp(0.4);
    // inside the block: after (0.2) + the voice's 0.35 s, then before (0.1)
    let b0 = a1 + smp(0.2 + SILENCE) + smp(0.1);
    let b1 = b0 + smp(0.2);
    // between blocks: the pacing's gap
    let c0 = b1 + smp(0.4);
    let sec = |x: usize| x as f64 / SR as f64;
    assert_eq!((n[0].t0, n[0].t1), (sec(a0), sec(a1)));
    assert_eq!((n[1].t0, n[1].t1), (sec(b0), sec(b1)));
    assert_eq!(n[2].t0, sec(c0));
    let _ = std::fs::remove_dir_all(s.dir());
}

/// AC7: a line inside a block must end its sentence; it fails before
/// anything is spoken, naming the line.
#[test]
fn ac7_a_line_mid_sentence_inside_a_block_fails_before_speaking() {
    let lines = [
        Line::new("duro", "En su punto más duro, lo mismo:"),
        Line::new("palanca", "fuerza por palanca."),
    ];
    let s = store("ac7b");
    let v = Noisy::new("voz");
    let e = narrate_utterances(
        &lines,
        &spoken(&lines),
        &[Utterance {
            lines: 0..2,
            voice: &v,
        }],
        &pacing(),
        &s,
    )
    .unwrap_err();
    assert!(e.to_string().contains("«duro»"), "{e}");
    assert_eq!(v.calls.get(), 0);
    // as the last line of its own block it is fine
    let ok = narrate_utterances(
        &lines,
        &spoken(&lines),
        &one_per_line(&lines, &v),
        &pacing(),
        &s,
    );
    assert!(ok.is_ok(), "{ok:?}");
    let _ = std::fs::remove_dir_all(s.dir());
}
