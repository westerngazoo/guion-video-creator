# R-0012 — The narrated reel: one command and an automatic gate

- **Status:** In review (implemented locally, awaiting the owner's sign-off)
- **Milestone:** M2
- **Owner:** físico buen físico (see project-specifics.md)
- **Created:** 2026-10-16
- **Depends on:** R-0005 (`mux_at_loudness`, `measure`), R-0007 (`narrate_lines`), R-0009 (`Narration`, `subtitle`, `eventos.json`), R-0010 (`export`)
- **Realized by:** SPEC-0012
- **QA:** `qa` agent run scoped to this requirement

> Numbering: R-0011 is the sound design as data (`score`, guion-audio),
> written in parallel; this is the next free integer.

## 1. Statement

`guion-video-creator` must make a narrated reel with **one command** from
the data its post declares (`reel.toml`), and must **gate** the result
automatically: everything about a rendered reel that a machine can
measure (subtitles, colour roles, layout, motion, audio, caption,
determinism) is checked against a **brand profile** the brand writes, with
one PASS/FAIL per check, located in time and frames, before any person
reviews it.

## 2. Rationale

The first studio (rotorf-sico, 2026-10-14 to 16) counted 3 to 5 review
rounds per narrated reel, and most FIX verdicts were rules already written
or measurable (its `docs/proceso.md`): a subtitle over 840 px, two golds at
once, something in the subtitle band, a still frame under the voice, a
closing hold under 1.5 s, the bed pumping between lines, U+202F, hashtags,
something internal in the caption. The owner decided (2026-10-16) that,
once the design, template and voice are fixed, the pipeline and those
checks live here, as an abstraction layer, so reviewers read a short report
instead of forty frames. Judgement (a model's result said as a fact, a
circular cause, a physically false drawing) stays with the reviewers.

## 3. Acceptance criteria

- **AC1. The brand profile.** A TOML the brand provides gives every value
  the gate compares with: canvas, opaque palette tokens, roles (background,
  red, focus), the subtitle (face, size and its unit, box, baseline, band,
  words, forbidden signs, long line, fade), the formula band, the safe
  area (with free and exempt rectangles), motion, scan, audio and caption
  limits. Unknown keys fail by name; a role naming a token not in
  `[tokens]` fails naming both. The crate holds no brand value (R-0009 AC7).
- **AC2. Subtitles.** From `voz.json`: each subtitle on one row no wider
  than the box, measured in the brand's face (em or line-height scale);
  at most N words (formulas do not count); no digits or forbidden signs; a
  line longer than `long_line` with more than one sentence enters on a
  later sentence (`desde`); never two at once.
- **AC3. Colour roles.** On every `stride`-th decoded frame, the red role
  makes at most one region, and the focus role (if any) at most one region
  of `focus_min_area` pixels or more; a region is a role's pixels merged
  within `merge` px.
- **AC4. Layout.** Ink outside the safe area (or in a free rectangle), not
  in an exempt one, fails, unless it lasts under `safe.min_duration` (an
  entrance sweeping through); outside the subtitle's measured box nothing
  but background is in the subtitle band; a formula region (not taller than
  `formula.max_height`) that crosses its band fails.
- **AC5. Motion.** Every frame is compared, by 8×8 block means outside the
  subtitle band, with the last frame that moved; a still stretch with more
  than `max_still_with_voice` s of voice fails, and the closing hold (the
  last change to the end) must last `closing_hold` s or more. The MP4 has
  the frames `eventos.json` says and lasts no more than `max_duration`.
- **AC6. Audio.** Integrated loudness within tolerance of the target and
  true peak at or under the ceiling (guion-encode's `measure`); in every gap
  between lines shorter than `short_gap` that holds no beat event, the mix
  stays `gap_max_db` or more under the lines around it.
- **AC7. Caption.** On the texts guion-publish exports: hashtags and
  characters within the limits; U+202F between a number and each unit the
  profile lists; none of the profile's internal patterns (`#` = a digit).
- **AC8. The report.** A JSON (`check.json`) and a human summary of 40
  lines or fewer; a missing input is an error naming it; `guion reel check`
  exits 0 on PASS, 1 on FAIL, 2 on error.
- **AC9. Determinism.** Given two renders, the gate compares their bytes.
- **AC10. One command.** `guion reel <post>` reads `reel.toml` and runs:
  the voice if `voz.wav` is missing (`voz.toml` → `voz.wav` + `voz.json`,
  byte-identical to the studio's narrator), the scene's binary, the sound
  command, the mix to −14 LUFS / −1 dBTP, `caption.txt` + `alt-text.txt`,
  the cover frame, and the gate; `--twice` renders the scene twice and
  feeds both to AC9.

## 4. Constraints & non-goals

- No new crate in the lock (`ab_glyph` is motoreel-typeset's; `toml`,
  `serde_json`, guion-publish are the workspace's).
- The sound step calls the post's command until R-0011's engine replaces it.
- Not here: judging the physics or the script; the contact sheet (R-0009).

## 5. Open questions

- Q1. "Repeats the screen" (a subtitle saying what a label shows) is
  only almost measurable: it needs the scene's label texts. Recommend: a
  scene-side `labels.json`, as its own requirement.
- Q2. Should `check.json` be versioned with the post? Recommend no: it
  holds a runtime; the reviewers read it next to the render.
- Q3. Run the scan in parallel (it takes 40–55 s on an 80 s reel, most of
  it decoding and classifying at full size)? Recommend yes, if reels batch.

## 6. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-10-16 | The gate is a module of guion-reel (`gate`), the commands live in guion-cli (`guion reel`) | The gate reads what guion-reel defines (`voz.json`, subtitles, `eventos.json`); the pipeline composes guion-audio, guion-encode and guion-publish, which is the CLI's job |
| 2026-10-16 | Motion by 8×8 block means against the last moving frame, not per pixel against the previous frame | Measured on *frenar-y-girar*: a keyframe re-quantizes the logo's edges by up to 60 levels per pixel (a false "change" 0.5 s before the end), but block means move under 8; comparing with the last moving frame catches slow fades that move under the threshold per frame |
| 2026-10-16 | The subtitle band trusts the subtitle's measured box; colours inside it are not classified | After 4:2:0, a gold italic's anti-aliased edges match no clean blend (seen on *euler*); outside the box everything non-background is foreign |
| 2026-10-16 | A focus region counts from `focus_min_area`; layout misses shorter than `safe.min_duration` are passing sweeps | A formula and its named symbols are one focus by the brand's own design (Rotor §11); an entrance zooming through the margin in one frame is motion, not layout (*euler* 53.0 s, 54.7 s) |
| 2026-10-16 | Short gaps holding a beat event are exempt from the pumping check | Rotor's designer allowed the bed up only at beat changes (`unir = 0.5`); the profile names the event prefix |
| 2026-10-16 | Pumping is the gap's level relative to the lines, with the limit halfway between the brand's ducked and released bed (the profile says which) | Subtracting `voz.wav` from the mix to measure the bed *under* the voice was tried: the fit's residual (the voice's room, the codec) sits at −28 to −30 dB, above the bed it should reveal. The level in the gap, where the voice is silent, is measurable; the brand's bed and duck values say where ducked and released sit |
| 2026-10-16 | Narration bytes verified: `guion reel voice` reproduces `voz.wav` and `voz.json` of *frenar-y-girar* and *euler* (same md5 as `tools/narra.py`) | AC10 |

## Changelog

- 2026-10-16 — created; realized by SPEC-0012 in the same change.
