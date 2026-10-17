# R-0007 — Narration from a cue list: the voice sets the clock

- **Status:** In review (implemented locally, awaiting the owner's sign-off)
- **Milestone:** M2
- **Owner:** físico buen físico (see project-specifics.md)
- **Created:** 2026-10-14
- **Depends on:** R-0005 (ffmpeg discovery, `GUION_FFMPEG`)
- **Realized by:** SPEC-0007
- **QA:** `qa` agent run scoped to this requirement

> Numbering: `R-0004` stays earmarked for M4 source resolution (R-0006
> numbering note); `R-0005` and `R-0006` are taken, so this is the next free
> integer.

## 1. Statement

`guion-audio` must turn an ordered list of lines into a narration track
**without a screenplay**: each line spoken in turn by a deterministic TTS
engine, separated by fixed pauses, with the measured start and end of every
line *and of every sentence inside it*. The same engine output must give the
same samples and the same times, byte for byte.

## 2. Rationale

The screenplay path (`narrate_screenplay`) fixes each cue's window first and
fits the speech into it (`fit_to_window` pads or resamples). A narrated reel
works the other way round: the voice is recorded first and **its measured
durations are the timeline** the picture and the sound follow. The sibling
`rotorf-sico` studio built that locally (`tools/narra.py`, 2026-10-14, Python)
for its narrated reels, with three findings that belong here:

- **Determinism.** Piper samples prosody noise on every run. With
  `noise_scale = 0` and `noise_w = 0` the same text and voice give the same
  bytes (measured: three runs of the C++ build, one md5; two of the PyPI
  build, one md5). The default flags do not.
- **Sentence times can be measured.** Piper leaves `sentence_silence` after
  each sentence. In a line of *n* sentences, the *n − 1* longest internal
  silences (10 ms windows below −48 dBFS, 0.25 s or longer) are the cuts. A
  scene anchors mid-line events there.
- **The two Piper builds differ.** The C++ binary (rhasspy/piper
  2023.11.14-2, `piper` 1.2.0) silently **ignores** the PyPI spellings
  (`--noise-scale`): a tool written for PyPI runs it with noise and loses
  determinism without an error. It also pads the *last* sentence with
  exactly `sentence_silence × rate` zero samples, which the PyPI build
  (1.8.0) does not. And it phonemizes with an older espeak-ng
  (`ðɪ` for `ðɪʲ`), so the speech itself is slightly different.

The engine (`PiperEngine`) and the cue model already lived here, but the
engine shelled out without determinism flags, mislabelled the voice's native
rate as 44.1 kHz, and every entry point required a `Screenplay`.

## 3. Acceptance criteria

- **AC1.** Determinism: `narrate_lines` over the same lines with the same
  engine output returns identical samples and times. `PiperEngine::deterministic`
  passes `noise_scale = 0` and `noise_w = 0`. Samples stay 16-bit from the
  engine to the file (no `f32` round trip); resampling is ffmpeg in bit-exact
  mode.
- **AC2.** Layout to the sample: the first line starts after `lead_in`; each
  line after the previous one's speech, its `after`, the `gap` and its own
  `before`, each silence rounded to the nearest sample (ties to even); no gap
  after the last line.
- **AC3.** Sentence times: a line's text is cut after `.`, `?` or `!`
  followed by whitespace; its *n − 1* longest internal pauses
  (`SilenceRule`) are the cuts; each sentence runs from the end of the pause
  before it to the start of the pause after it, and the last one to the end
  of the line. A comma's short pause does not cut when the sentence pauses
  are longer.
- **AC4.** WAV exactness: the written WAV is mono 16-bit PCM with the
  canonical 44-byte header and round-trips sample for sample; reading walks
  the chunks (a `LIST` before `data` is skipped).
- **AC5.** No screenplay: with `default-features = false` the crate builds
  and the cue-list API works without `guion-assemble` (and so without
  `motoreel`, `garust`, `guion-motion`). The default features keep the
  screenplay API unchanged for `guion-cli` and the studio app.
- **AC6.** Failures are typed and located (CLAUDE.md §6): an empty line, a
  negative or non-finite pause (naming the line and the field), and a line
  with fewer measured pauses than sentences (naming the line, the counts and
  the minimum pause). A failure returns before anything is written.
- **AC7.** Both Piper builds behave alike: only flag spellings both accept
  are passed (`--model`, `--output_file`, `--noise_scale`, `--noise_w`,
  `--length_scale`, `--sentence_silence`); the text goes on stdin without a
  shell; `GUION_PIPER` selects a binary outside `PATH`; an utterance ends
  where its speech ends (the C++ build's trailing padding, exactly
  `sentence_silence × rate` zeros, is removed; any other ending is kept).

## 4. Constraints & non-goals

- Piper and ffmpeg stay external processes; no FFI, no new crates.io
  dependency (`guion-encode` is a workspace crate).
- The cue list is plain Rust types. Parsing a studio's own file format (the
  rotorf-sico `voz.toml`, with Spanish keys, subtitles and `desde`) and
  writing its timing file (`voz.json`) stay in that studio.
- Not here: audio beds, ducking under the voice, the final mix (R-0005 owns
  loudness), choosing a voice.

## 5. Open questions

- Q1. Should a Piper-backed test run in CI (download the 2023.11.14-2 binary
  and one voice, ~70 MB) the way ffmpeg tests do (R-0005: fail, not skip)?
  Recommend yes, pinned by md5, once the owner accepts the download in CI.
- Q2. Should `narrate_screenplay` move onto `narrate_lines` (speak first,
  then check that each cue fits its window) instead of `fit_to_window`?
  Recommend yes: stretching speech by sample picking changes its pitch.
- Q3. Should the resampling move from ffmpeg to a Rust polyphase filter
  (16 → 48 kHz is an integer ratio)? It would drop one process per line but
  change every sample, so only together with a deliberate voice re-take.

## 6. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-10-14 | The screenplay API goes behind a default feature `screenplay` | A studio with its own scene format should not compile the assembler twice; existing consumers keep the default (AC5) |
| 2026-10-14 | `PiperEngine::synthesize` now resamples to `SAMPLE_RATE` | It returned the voice's native rate (16 kHz for `*-low` voices) labelled as 44.1 kHz |
| 2026-10-14 | ffmpeg is found through `guion_encode::ffmpeg::ffmpeg` | One rule for `GUION_FFMPEG` across the crates |
| 2026-10-14 | Sentence-time arithmetic mirrors the studio's Python tool operation for operation (integer windows, `f64` division, stable sort) | So the studio can switch and compare with `md5sum`: on its narrated reel *imaginarios* (12 lines, 87.464 s) the Rust path and `tools/narra.py` give the same `voz.wav` and `voz.json` bytes with the same Piper |
| 2026-10-14 | The C++ build's padding after the last sentence is trimmed only when it is exactly `sentence_silence × rate` zeros | It is a pause *between* sentences; trimming anything less exact could cut speech (AC7) |

## Changelog

- 2026-10-14 — created; realized by SPEC-0007 in the same change.
