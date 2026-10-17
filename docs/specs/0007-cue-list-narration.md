# SPEC-0007 — Narration from a cue list

- **Status:** In review
- **Realizes:** R-0007
- **Created:** 2026-10-14
- **Depends on:** SPEC-0005 (ffmpeg discovery)
- **Module(s):** `guion-audio`

## 1. Motivation

Realizes R-0007. Proven first in the `rotorf-sico` studio
(`tools/narra.py`); this lifts the reusable part into the framework so the
studio's Rust `narra` binary (`mathematicus/motor/src/bin/narra.rs`) and any
other creator share one path.

## 2. Design

### Module layout (`guion-audio`)

```
guion-audio/src/
├── lib.rs      # re-exports; screenplay modules behind `screenplay`
├── engine.rs   # NarrationEngine (+ synthesize_pcm16), ScaffoldEngine, PiperEngine
├── lines.rs    # Line, Pacing, SilenceRule, narrate_lines, sentence_times, split_sentences
├── pcm.rs      # Pcm16, read_wav_pcm16 (chunk walk), write_wav_pcm16 (44-byte header)
├── error.rs    # + InvalidLine, SentenceTiming, Wav, Resample
├── wav.rs      # (unchanged) f32 buffers at 44.1 kHz
├── bed.rs      # [screenplay]
├── narrate.rs  # [screenplay]
└── script.rs   # [screenplay]
```

### Use

```rust
let piper = PiperEngine::deterministic("/voces/en-gb-alan-low.onnx")
    .binary("/deps/bin/piper-cpp")      // or GUION_PIPER, or PATH
    .length_scale(1.0)
    .sentence_silence(0.25);
let lines = vec![Line::new("gancho", "For almost four hundred years, ...")];
let pacing = Pacing { sample_rate: 48_000, lead_in: 0.5, gap: 0.35 };
let n = narrate_lines(&lines, &pacing, &SilenceRule::PIPER, &piper)?;
n.write_wav(Path::new("voz.wav"))?;     // and n.lines[i].{t0, t1, sentences}
```

### Engine (AC1, AC7)

`NarrationEngine::synthesize_pcm16(text, rate)` is a provided method (the
default converts `synthesize` and serves only `SAMPLE_RATE`), so existing
engines keep compiling. `PiperEngine` overrides it: Piper writes a WAV at the
voice's rate; the trailing padding is trimmed (AC7); ffmpeg resamples with
`-ar <rate> -ac 1 -c:a pcm_s16le -fflags +bitexact -flags:a +bitexact`. When
nothing was trimmed, ffmpeg reads Piper's own file (the studio's byte
equivalence depends on it).

### Timing (AC2, AC3)

Integer sample counts; a time is `samples / rate` as `f64`. Silences are
`round_ties_even(secs × rate)` samples. Sentence pauses: windows of
`round(rate × window)` samples; quiet when `Σs² / window < threshold²`;
runs touching neither end of the line and at least `round(min_gap / window)`
windows long; the *n − 1* longest by a stable sort, then put back in order.

## 3. Acceptance-criteria checklist

| AC | Test |
|----|------|
| AC1 | `tests/lines.rs::ac1_deterministic`; `engine::tests::piper_flags_are_shared_by_both_builds_and_deterministic` |
| AC2 | `tests/lines.rs::ac2_layout_to_the_sample` |
| AC3 | `tests/lines.rs::ac3_sentence_times_are_measured` |
| AC4 | `tests/lines.rs::ac4_pcm_is_exact` |
| AC5 | `cargo test -p guion-audio --no-default-features` (the `lines` tests run; the screenplay tests are compiled out) |
| AC6 | `tests/lines.rs::ac6_failures_name_the_line` |
| AC7 | `engine::tests::piper_flags_are_shared_by_both_builds_and_deterministic`; `engine::tests::trailing_sentence_padding_is_trimmed_only_when_exact` |

End to end, outside CI (needs Piper and a voice): in `rotorf-sico`, the Rust
`narra` and `tools/narra.py` with the PyPI build give the same `voz.wav` and
`voz.json` bytes for its three narrated reels (*imaginarios*: `voz.wav` md5
`1ddf72f4…`); with the C++ build, two runs give the same md5 (`voz.wav`
`d01ce77d…`).
