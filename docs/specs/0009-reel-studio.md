# SPEC-0009 — The reel studio

- **Status:** In review
- **Realizes:** R-0009
- **Created:** 2026-10-14
- **Depends on:** SPEC-0005 (`encode_ordered_with`, `ffmpeg::ffmpeg`), SPEC-0007 (the narration)
- **Module(s):** `guion-reel` (new crate)

## 1. Motivation

Realizes R-0009. Lifted from the rotorf-sico motor (`src/estudio.rs` and
`src/voz.rs`, 2026-10-06 to 2026-10-14), operation for operation, so the
studio can switch and compare with `md5sum`.

## 2. Design

### Crate layout (`crates/guion-reel`)

```
guion-reel/src/
├── lib.rs        # re-exports
├── canvas.rs     # Canvas: px size, fps, px per view unit; to_view, len, frames, video_spec
├── clock.rs      # Clock, Series; to_json (events.json schema v1)          AC1
├── json.rs       # number ({:.6}) and string (RFC 8259) for events.json     AC1
├── studio.rs     # Reel, Frame<L>, Overlay, NoOverlay, Layout, SheetGrid,
│                 # Command, times, all_cores, png_from_rgb                  AC2–AC4
├── narration.rs  # Narration, NarratedLine, Sentence, COMMA_PAUSE           AC5
├── subtitle.rs   # Piece, Unit, is_formula_word, units, word_count,
│                 # row_width, break_rows                                    AC6
├── ease.rs       # progress, ease_out_cubic (the fades)
└── error.rs      # ReelError
```

Dependencies: `guion-encode`, `motoreel`, `motoreel-typeset` (for `Fonts`),
`serde`, `serde_json` (`float_roundtrip`). Not guion-core, not guion-assemble
(AC7).

### Use

```rust
use guion_reel::{Canvas, Clock, Command, Frame, NoOverlay, Reel};

let clock = Clock::new().event("qed", 12.4)?.series("angle", 0.0, 1.0 / 30.0, angles)?;
let reel = Reel::new("my-reel", dir, 20.0, background, clock, Canvas::VERTICAL_REEL, Box::new(load_fonts))
    .with_label_face(MONO_BOLD);
let cmd = Command::from_args(&std::env::args().skip(1).collect::<Vec<_>>())?;
for path in reel.run(&cmd, |t| -> Frame<NoOverlay> { (scene.frame(t), Vec::new()) })? {
    println!("{}", path.display());
}
```

### The render (AC2, AC4)

`Reel::video` calls `encode_ordered_with(mp4, canvas.video_spec(),
frame_count, threads, init, render)`. `init(h)` gives thread `h` its own
`PpmSink` in `<mp4 stem>.frames/h<h>` (motoreel's sink writes a file per
frame, so threads cannot share a folder); `render` computes `frame(i / fps)`,
rasterizes it (`PpmSink::frame`, read back `frame_{i:05}.ppm`, delete it),
applies each overlay's `compose(&mut p6)` in order, and hands the encoder the
last `width × height × 3` bytes (the P6 header has a fixed length). The
working folder is removed whether the encode succeeds or not.

`Overlay::compose` receives the whole P6 so a compositor can parse its own
header; `NoOverlay` is an empty enum for reels without overlays.

### Review (AC3)

`frames_png` and `contact_sheet` share the raster path with a single sink.
The sheet's cell `n` goes to column `n mod C`, row `n div C`, at
`(gap + col·(w + gap), gap + row·(h + gap))`, with `w = width / k`,
`h = height / k`; each cell pixel is `(Σ block + k²/2) / k²` per channel
(integer). Before rasterizing, the frame gets the label: a black band
`1080 k × 120 k` at 0.78 opacity, without stroke, and the text at
`(36 k, 92 k)`, size `76 k`, white, in the caller's face, with
`k = width / 1080`. `label_text` picks the latest event with `t_e ≤ t + 1e-9`;
within half a frame it prints `"{t:.2} s · {name}"`, otherwise
`"{t:.2} s · {name} +{t − t_e:.2}"`, and with no event `"{t:.2} s"`.

`key_times`: the event times plus the duration, sorted (`total_cmp`),
deduplicated, and the midpoint of each consecutive pair rounded to a frame,
`round(m · fps) / fps`.

PNGs are written by the ffmpeg guion-encode finds (`GUION_FFMPEG` or PATH),
from raw RGB24 on stdin.

## 3. `events.json`, schema v1 (AC1)

```text
{"duracion":D,"fps":F,"eventos":{"<name>":T,...},"series":{"<name>":{"t0":T0,"dt":DT,"v":[V,...]},...}}
```

No whitespace; numbers `format!("{x:.6}")`; events and series in
declaration order; names escaped (`"`, `\`, controls as `\u00XX`). The
Spanish keys are the contract the first studio and its sound designs use.

## 4. `voz.json` (AC5)

```text
{"modelo"?: S, "duracion_voz": D,
 "lineas": [{"id": S, "texto": S, "subtitulo": S, "t0": T, "t1": T,
             "desde"?: N, "frases"?: [{"texto": S, "t0": T, "t1": T}]}]}
```

`deny_unknown_fields` at every level (CLAUDE.md §6). Without `frases`, the
line is one sentence; without `desde`, it is 1. Checks: `1 ≤ desde ≤
#frases`; for consecutive lines, `t0[i+1] ≥ t1[i]`.

Phrase time (`at` / `after`): the first sentence containing the phrase
(`str::find`, bytes); `b` is its start (or end); with weight `w(s) = Σ (1 +
2.5 if c ∈ {',', ':', ';'} else 1)` over chars, the time is
`t0 + (t1 − t0) · w(text[..b]) / w(text)`.

Active subtitle at `t` with fade `f`: the first line whose
`a = eo(prog(t, s, s + f)) · (1 − eo(prog(t, t1, t1 + f)))` is positive, where
`s` is the start of sentence `desde`, `prog` is the clamped progress and `eo`
is `1 − (1 − p)³`.

## 5. Subtitles (AC6)

`units(s, is_formula)`: split on spaces (skipping empties); the core of a word
is it without trailing `.,:;`; a formula core joins the previous unit's
formula if that unit is still open (a formula with no trailing punctuation),
otherwise starts a unit; trailing punctuation goes as prose and closes the
unit. `is_formula_word`: the constant `i`, a word with one of `₁₂₃²³⁴−=×√()+·`,
or all ASCII digits. `row_width = Σ unit widths + space · (n − 1)`.
`break_rows`: one row if it fits or has one unit; else, with at least two rows
allowed, the split `k` minimizing `max(width(..k), width(k..))` (the first on
a tie, `Iterator::min_by`); an error if even that is too wide.

## 6. Acceptance-criteria checklist

| AC | Test (`tests/r0009_reel.rs`) |
|----|------|
| AC1 | `ac1_events_json_has_the_schema_bytes`, `ac1_an_empty_clock_is_valid_json`, `ac1_bad_entries_fail_naming_the_entry`; unit tests in `json.rs` |
| AC2 | `ac2_parallel_video_is_byte_identical_to_one_thread`, `ac2_run_writes_the_events_and_the_mp4` |
| AC3 | `ac3_frames_png_are_the_frames`, `ac3_contact_sheet_is_the_grid_of_reduced_frames`, `ac3_key_times_are_the_middle_of_each_stretch`, `ac3_labels_name_the_time_and_the_last_event`, `ac3_command_line` |
| AC4 | `ac4_overlays_compose_after_the_raster_in_order` |
| AC5 | `ac5_the_clock_comes_from_the_sentences`, `ac5_failures_name_the_line_or_the_phrase`, `ac5_a_timeline_that_does_not_hold_together_fails` |
| AC6 | `ac6_units_keep_formulas_whole`, `ac6_rows_break_where_the_wider_row_is_narrowest`, `ac6_one_subtitle_at_a_time_with_its_fade` |
| AC7 | By construction: `Cargo.toml` (no guion-core, no guion-assemble) and the public API (every style input is a parameter) |
| AC8 | Out of tree: rotorf-sico, *imaginarios* rendered with the motor's modules and with `--features guion-reel`; the md5s are in rotorf-sico `docs/infra.md` §4 |

The ffmpeg tests fail, not skip, without ffmpeg (as R-0005's).

## 7. Decision log

See R-0009 §6.

## Changelog

- 2026-10-14 — created with R-0009.
