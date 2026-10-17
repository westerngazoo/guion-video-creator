# R-0009 — The reel studio: scenes written as code, one clock, one contact sheet

- **Status:** In review (implemented locally, awaiting the owner's sign-off)
- **Milestone:** M2
- **Owner:** físico buen físico (see project-specifics.md)
- **Created:** 2026-10-14
- **Depends on:** R-0005 (`encode_ordered_with`, ffmpeg discovery), R-0007 (the narration whose timeline AC5 reads)
- **Realized by:** SPEC-0009
- **QA:** `qa` agent run scoped to this requirement

> Numbering: `R-0008` is the sound synthesis in `guion-audio` (in progress
> in parallel), so this is the next free integer.

## 1. Statement

`guion-video-creator` must offer a studio for reels whose scene is **code**:
a pure function of time that returns motoreel primitives (and raster
overlays). Around that function the studio provides the MP4 (in parallel,
byte-identical to one thread), loose review frames, a contact sheet of the key
frames labelled with their events, and one clock file (`events.json`) that the
sound design reads instead of writing times by hand. For a narrated reel, the
studio reads the narration's timeline (`voz.json`) so every instant of the
scene comes from the measured voice, and lays out its subtitles without
knowing the faces they are set in.

## 2. Rationale

The screenplay path (guion-core → guion-assemble) declares a scene. Some
scenes do not fit a declarative schema, and their studios write them in
Rust. The sibling `rotorf-sico` studio did exactly that (its
`mathematicus/motor`, 2026-10-06 to 2026-10-14) and grew, around each scene's
`cuadro(t)`, everything this requirement lists: the parallel render on
`encode_ordered_with`, `--cuadros`, `--hoja`, `eventos.json`, and the
voice clock of its narrated reels (`voz.rs`: `t(id)`, `en(id, phrase)`, the
subtitle that fades in with its sentence). None of it is about that studio's
brands; all of it is what any creator writing scenes in code needs. The owner
decided (rotorf-sico `docs/decisions.md`, 2026-10-06 and 2026-10-14) that what
is reusable lives in its own repo with that repo's process; this is that move.

Findings that belong here, with the move:

- **Determinism.** A frame is a pure function of `t = i / fps` (never an
  accumulated sum) and motoreel's sink clears its canvas every frame, so the
  parallel MP4 is the one-thread MP4, byte for byte (the studio measured it on
  a 1 704-frame reel: 42.3 s with one thread, 25.4 s with four, one md5).
- **Key frames are mid-stretch.** At the start of a beat the frame is often
  still empty; the middle of each stretch between two events is when what came
  in has settled. Those are the contact sheet's default frames.
- **The voice is the clock.** A phrase's time is estimated inside its
  *measured* sentence (R-0007 AC3), in proportion to the letters with a comma
  worth 2.5 more; a missing phrase fails by name, so a script change never
  leaves a stale time in the scene.

## 3. Acceptance criteria

- **AC1. One clock file.** A `Clock` holds named events and sampled series
  in declaration order and writes `events.json` in schema v1 (SPEC-0009 §3):
  keys in a fixed order, every number with six decimals, strings escaped per
  RFC 8259. An event at a non-finite or negative time, or a series with a
  non-finite value or `dt ≤ 0`, is a typed error naming the entry.
- **AC2. Parallel equals serial.** The MP4 written with *n* threads is
  byte-identical to the one written with one thread, has
  `round(duration × fps)` frames, and leaves no working folders behind.
- **AC3. Review.** `frames_png` writes `<review>/t-<t>.png` per requested
  time; `contact_sheet` writes one PNG: cells of the frame reduced by an
  integer factor (each cell pixel the rounded mean of its block), in rows of a
  fixed number of columns with a gap, each labelled (when the caller gives a
  face) with its time and the last event at or before it ("20.65 s · pasa
  +2.55"). The default times are the middle of each stretch between events
  (and the end), rounded to a frame. A command line (`--frames`, `--sheet`,
  `--serial`, `--events`, or nothing) selects among these; a bad time is a
  typed error naming it.
- **AC4. Overlays.** A frame may carry raster overlays, composed onto the
  rasterized frame in order, before it is encoded or saved; they reach the MP4,
  the review frames and the contact sheet alike.
- **AC5. The narration timeline.** `Narration` reads `voz.json` (SPEC-0009
  §4) and gives each line's and sentence's measured times, the estimated time
  a phrase starts or ends inside its sentence, and the subtitle on screen at
  `t` with its fade. It fails, typed and located, on an unknown key, a `desde`
  outside its line's sentences, a line that starts before the previous one
  ends, an unknown line, sentence or phrase. Times parse to the correctly
  rounded `f64`.
- **AC6. Subtitles without a face.** A subtitle splits into units that never
  break (a word, or a run of formula words, with its trailing punctuation);
  the words to read exclude formulas; rows break at the two-row split whose
  wider row is narrowest, with widths measured by the caller; a subtitle that
  does not fit in the allowed rows, or that enters before the previous one has
  faded out, is a typed error.
- **AC7. No brand.** The crate holds no faces, colours, palette, sizes or
  folder names of any studio; all of them are inputs (`Canvas`, background,
  `FontLoader`, label face, `Layout`, `SheetGrid`, the formula rule). It does
  not depend on guion-core's schema or on guion-assemble.
- **AC8. A drop-in for the studio it came from.** Behind thin wrappers that
  keep its scenes' API, the rotorf-sico motor renders its narrated reel
  *imaginarios* to the same bytes (MP4, `eventos.json`, `audio.wav`, the
  final MP4 with audio, the contact sheet) as with its own modules.

## 4. Constraints & non-goals

- No new crates.io dependency beyond the workspace's (`serde`,
  `serde_json`); `guion-encode` and motoreel are workspace crates.
- Not here: the scene itself (low-poly kits, glyph outlines, formulas — those
  belong to motoreel and to each studio), writing `voz.json` (the studio's
  narrator, on R-0007), the sound design that reads `events.json` (R-0008),
  the mix (R-0005).
- The `events.json` and `voz.json` key names are Spanish, as the first studio
  wrote them; renaming them would change every file's bytes (SPEC-0009 §3).

## 5. Open questions

- Q1. Should duplicate event names be an error (AC1)? Today they are kept in
  order and a JSON reader keeps the last. Recommend: an error, once the
  studio's reels have been checked for duplicates.
- Q2. Should the screenplay path (guion-assemble) render through `Reel` too,
  so both paths share the contact sheet and `events.json`? Recommend yes, as
  its own requirement.
- Q3. Should `voz.json` be written by guion-audio itself (a
  `Narration::write_timeline` next to R-0007's `narrate_lines`), so the file
  this crate reads has its writer in the same repo? Recommend yes, after R-0008.

## 6. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-10-14 | A new crate, `guion-reel`, not a module of guion-encode or guion-audio | guion-encode is frames → MP4 and knows nothing of motoreel; guion-audio is sound. The studio sits on both and on motoreel, and its consumers need none of the screenplay crates (CLAUDE.md §2: one crate per coupling) |
| 2026-10-14 | The library exposes operations and a `Command`; flag names are English (`--frames`, `--sheet`, `--serial`, `--events`) | A studio keeps its own flags by mapping them to `Command` (rotorf-sico keeps `--cuadros`, `--hoja`, `--serie`, `--eventos`) |
| 2026-10-14 | `serde_json` with `float_roundtrip` for `voz.json` | The studio's reader parsed with `str::parse::<f64>`, which is correctly rounded; serde_json's default fast path is not always, and a time one ulp off can move a frame (AC8). Feature unification turns it on for the workspace's other serde_json users, which only makes their parsing exact |
| 2026-10-14 | The contact-sheet label is sized for a 1080 px wide canvas and scaled by `width / 1080` | At 1080 the factor is exactly 1, so the studio's sheets keep their bytes; other canvases get the same proportions |
| 2026-10-14 | Byte equivalence verified out of tree on *imaginarios* (AC8) | The md5 of every output, before and after, is recorded in rotorf-sico `docs/infra.md` |

## Changelog

- 2026-10-14 — created; realized by SPEC-0009 in the same change.
