//! R-0007: narration from a cue list. One test per acceptance criterion,
//! with a fake engine whose output is known sample by sample.

use guion_audio::{
    narrate_lines, read_wav_pcm16, sentence_times, split_sentences, write_wav_pcm16, AudioError,
    Line, NarrationEngine, Pacing, SilenceRule,
};

const SR: u32 = 48_000;

/// Speaks every sentence as 0.5 s of a loud square wave per word, with a
/// 0.05 s dip after each comma and 0.4 s of silence after each sentence but
/// the last, like Piper with `sentence_silence`.
struct Fake;

impl NarrationEngine for Fake {
    fn name(&self) -> &'static str {
        "fake"
    }
    fn synthesize(&self, _: &str) -> Result<Vec<f32>, AudioError> {
        unreachable!()
    }
    fn synthesize_pcm16(&self, text: &str, sr: u32) -> Result<Vec<i16>, AudioError> {
        let mut out = Vec::new();
        let parts = split_sentences(text);
        for (i, s) in parts.iter().enumerate() {
            for w in s.split_whitespace() {
                for k in 0..(sr / 2) as usize {
                    out.push(if (k / 24) % 2 == 0 { 8000 } else { -8000 });
                }
                if w.ends_with(',') {
                    out.resize(out.len() + (sr as usize) / 20, 0);
                }
            }
            if i + 1 < parts.len() {
                out.resize(out.len() + (sr as usize) * 2 / 5, 0);
            }
        }
        Ok(out)
    }
}

fn pacing() -> Pacing {
    Pacing {
        sample_rate: SR,
        lead_in: 0.5,
        gap: 0.35,
    }
}

/// AC1: the same input gives the same samples and the same times.
#[test]
fn ac1_deterministic() {
    let lines = [
        Line::new("a", "One two. Three, four."),
        Line::new("b", "Five."),
    ];
    let x = narrate_lines(&lines, &pacing(), &SilenceRule::PIPER, &Fake).unwrap();
    let y = narrate_lines(&lines, &pacing(), &SilenceRule::PIPER, &Fake).unwrap();
    assert_eq!(x, y);
}

/// AC2: lead-in, extra silences and gaps, to the sample; no gap after the last line.
#[test]
fn ac2_layout_to_the_sample() {
    let mut a = Line::new("a", "One.");
    a.after = 1.2;
    let mut b = Line::new("b", "Two three.");
    b.before = 0.2;
    let n = narrate_lines(&[a, b], &pacing(), &SilenceRule::PIPER, &Fake).unwrap();
    let s = |secs: f64| (secs * SR as f64).round() as usize;
    let a0 = s(0.5);
    let a1 = a0 + s(0.5);
    let b0 = a1 + s(1.2 + 0.35) + s(0.2);
    let b1 = b0 + s(1.0);
    assert_eq!(n.lines[0].t0, a0 as f64 / SR as f64);
    assert_eq!(n.lines[0].t1, a1 as f64 / SR as f64);
    assert_eq!(n.lines[1].t0, b0 as f64 / SR as f64);
    assert_eq!(n.lines[1].t1, b1 as f64 / SR as f64);
    assert_eq!(
        n.samples.len(),
        b1,
        "nothing after the last line but its own `after`"
    );
}

/// AC3: n sentences → the n − 1 longest internal pauses; a comma's short
/// dip does not cut; the last sentence runs to the end of the line.
#[test]
fn ac3_sentence_times_are_measured() {
    let text = "One, two. Three four, five. Six.";
    let speech = Fake.synthesize_pcm16(text, SR).unwrap();
    let t0 = 10.0;
    let s = sentence_times(text, &speech, SR, t0, &SilenceRule::PIPER).unwrap();
    assert_eq!(
        s.iter().map(|x| x.text.as_str()).collect::<Vec<_>>(),
        ["One, two.", "Three four, five.", "Six."]
    );
    let e = 1e-9;
    // "One, two." = 0.5 + 0.05 + 0.5 s of speech, then 0.4 s of pause
    assert!(
        (s[0].t0 - 10.0).abs() < e && (s[0].t1 - 11.05).abs() < e,
        "{s:?}"
    );
    assert!(
        (s[1].t0 - 11.45).abs() < e && (s[1].t1 - 13.0).abs() < e,
        "{s:?}"
    );
    let end = t0 + speech.len() as f64 / SR as f64;
    assert!((s[2].t0 - 13.4).abs() < e && s[2].t1 == end, "{s:?}");
    assert_eq!(
        split_sentences("  3.5 cuts?! Yes. e.g. too  "),
        ["3.5 cuts?!", "Yes.", "e.g.", "too"]
    );
}

/// AC4: the WAV round-trips sample for sample, with the canonical 44-byte
/// header; chunks before `data` are skipped on read.
#[test]
fn ac4_pcm_is_exact() {
    let dir = std::env::temp_dir().join(format!("guion-audio-ac4-{}", std::process::id()));
    let path = dir.join("voz.wav");
    let samples: Vec<i16> = (0..1000)
        .map(|i| ((i * 37) % 65_536 - 32_768) as i16)
        .collect();
    write_wav_pcm16(&path, SR, &samples).unwrap();
    let bytes = std::fs::read(&path).unwrap();
    assert_eq!(bytes.len(), 44 + 2000);
    assert_eq!(&bytes[36..40], b"data");
    let back = read_wav_pcm16(&bytes).unwrap();
    assert_eq!((back.sample_rate, back.channels), (SR, 1));
    assert_eq!(back.samples, samples);
    // the same audio with a LIST chunk in front of data (as ffmpeg writes without +bitexact)
    let mut with_list = bytes[..36].to_vec();
    with_list.extend_from_slice(b"LIST\x05\x00\x00\x00abcde\x00");
    with_list.extend_from_slice(&bytes[36..]);
    assert_eq!(read_wav_pcm16(&with_list).unwrap().samples, samples);
    assert!(!dir.join(".voz.wav.parcial").exists());
    std::fs::remove_dir_all(dir).unwrap();
}

/// AC6: typed, located failures.
#[test]
fn ac6_failures_name_the_line() {
    let r = narrate_lines(
        &[Line::new("vacia", "  ")],
        &pacing(),
        &SilenceRule::PIPER,
        &Fake,
    );
    assert!(matches!(r, Err(AudioError::InvalidLine { ref line, .. }) if line == "vacia"));
    let mut l = Line::new("neg", "Hi.");
    l.after = -0.1;
    let r = narrate_lines(&[l], &pacing(), &SilenceRule::PIPER, &Fake);
    assert!(
        matches!(r, Err(AudioError::InvalidLine { ref line, ref message }) if line == "neg" && message.contains("after"))
    );
    // two sentences the engine speaks without a pause between them
    struct Run;
    impl NarrationEngine for Run {
        fn name(&self) -> &'static str {
            "run"
        }
        fn synthesize(&self, _: &str) -> Result<Vec<f32>, AudioError> {
            unreachable!()
        }
        fn synthesize_pcm16(&self, _: &str, sr: u32) -> Result<Vec<i16>, AudioError> {
            Ok(vec![9000; sr as usize])
        }
    }
    let r = narrate_lines(
        &[Line::new("junta", "One. Two.")],
        &pacing(),
        &SilenceRule::PIPER,
        &Run,
    );
    match r {
        Err(AudioError::SentenceTiming {
            line,
            sentences,
            gaps,
            ..
        }) => {
            assert_eq!((line.as_str(), sentences, gaps), ("junta", 2, 0));
        }
        other => panic!("{other:?}"),
    }
}
