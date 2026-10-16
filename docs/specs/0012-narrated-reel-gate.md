# SPEC-0012 — The narrated reel: one command and an automatic gate

- **Status:** In review
- **Realizes:** R-0012
- **Created:** 2026-10-16
- **Module(s):** `guion-reel::gate` (new module), `guion-cli` (`guion reel`)

## 1. Design

```
guion-reel/src/gate/
├── mod.rs        # Inputs, run (one decode: motion every frame, pixels every stride-th), determinism, the clock
├── profile.rs    # Profile (serde, deny_unknown_fields) and its validation (AC1)
├── subtitles.rs  # SubtitleMeter (motoreel-typeset in the brand's face) and the five subtitle checks (AC2)
├── pixels.rs     # Classifier (16 M LUT, nearest token within tolerance), regions, outside_safe (AC3, AC4)
├── motion.rs     # block_means, moved, still stretches and closing hold (AC5)
├── audio.rs      # loudness (guion-encode measure), gap levels (AC6)
├── caption.rs    # unit spacing, internal patterns, on guion-publish's export (AC7)
├── video.rs      # ffmpeg decode (BT.709, TV range → RGB24), size, mono audio
└── report.rs     # Finding, Report, JSON and the ≤ 40-line summary (AC8)
guion-cli/src/reel/
├── mod.rs        # guion reel <post> | voice <post> | check <post>
├── config.rs     # reel.toml (deny_unknown_fields, ${VAR} expansion)
└── voz.rs        # voz.toml → voz.wav + voz.json (the studio's narrator, on narrate_lines)
```

### `reel.toml`

```toml
[reel]
brand = "../../../brand/reel-perfil.toml"   # the profile (AC1)
max_duration = 90.0
cover = 2.0                                 # optional: a review frame at that time
name = "x"                                  # optional: default, the folder's name

[voice]
model = "es-carlfm-x-low"                   # voz.toml must name the same
voices = "${ROTOR_VOCES}"                   # <voices>/<model>/<model>.onnx
piper = "${ROTOR_PIPER}"                    # optional

[scene]
manifest = "../../../motor/Cargo.toml"      # cargo build --release --bin <bin>
bin = "x"
features = []
frames_flag = "--cuadros"                   # default --frames (R-0009 Command)

[sound]
command = ["cargo", "run", "...", "{post}"] # run in the post's folder
output = "audio.wav"

[publish]
dir = "../../../publicar/x"                 # optional: caption.txt, alt-text.txt
```

### Use

```bash
guion reel <post> [--twice] [--no-check]
guion reel voice <post>
guion reel check <post> [--brand P.toml] [--mp4 F] [--compare A.mp4 B.mp4] [--json OUT]
```

The pipeline: voice (if `voz.wav` is missing) → `cargo build` + the scene's
binary (`<name>.mp4`, `eventos.json`) → the sound command →
`mux_at_loudness` (`<name>-audio.mp4`) → `export_file` + `write_to` → the
cover (`<bin> <frames_flag> <cover>`) → the gate on `<name>-audio.mp4`.

### The scan (AC3–AC5)

One ffmpeg decode to RGB24 (`scale=in_color_matrix=bt709:in_range=tv`).
Every frame: 8×8 block means of luma outside the subtitle band, compared
with the last moving frame (moved: one block over `strong`, or
`min_blocks` over `delta`). Every `stride`-th frame: each pixel classified
by a LUT (nearest token within `tolerance`, else none); role regions on a
4-px grid merged within `merge` px; ink outside the safe area; anything
non-background in the band outside the subtitle's measured box (centered
on `center_x`, width + 16 px each side); formula regions crossing their
band. Consecutive scanned frames that fail alike are merged into one
stretch, the worst detail kept.

## 2. Acceptance-criteria checklist

| AC | Test |
|----|------|
| AC1 | `tests/r0012_gate.rs::ac1_the_profile_is_the_brands_and_fails_by_name` |
| AC2 | `ac2_to_ac8_a_synthetic_reel_fails_where_it_should` (width, words, signs, overlap) |
| AC3 | same (two red regions at 1.0 s); unit `pixels::tests::two_glyphs_close_are_one_region_and_far_are_two` |
| AC4 | same (gold outside the safe area at 0.5 s; band clean) |
| AC5 | same (still under the voice from 2.0 s; closing hold; duration); unit `motion::tests::*` |
| AC6 | same (loudness); unit `audio::tests::a_loud_gap_is_measured_against_its_lines` |
| AC7 | same (bare unit, internal id); unit `caption::tests::*` |
| AC8 | same (JSON, summary); `ac8_a_missing_input_is_an_error_naming_it`; unit `report::tests::the_summary_never_passes_forty_lines` |
| AC9 | `ac9_two_renders_that_differ_fail` |
| AC10 | unit `guion-cli reel::config::tests::*`; out of tree: `guion reel voice` on *frenar-y-girar* and *euler* (md5 of `voz.wav`, `voz.json` equal to `tools/narra.py`'s), `guion reel --twice` on *frenar-y-girar* (rotorf-sico, 2026-10-16) |

## 3. Decision log

See R-0012 §6.

## Changelog

- 2026-10-16 — created with R-0012.
