# R-0016 — Formats: one post, the reel, TikTok, the YouTube Short and the long YouTube video

- **Status:** In review (implemented locally, awaiting the owner's sign-off)
- **Milestone:** M2
- **Owner:** físico buen físico (see project-specifics.md)
- **Created:** 2026-10-20
- **Depends on:** R-0009 (`Canvas`, `Reel`, `Command`, `Narration`), R-0010 (guion-publish), R-0012 (the gate and its profile), R-0015 (`escena.toml`, `Contexto`, the studio's template)
- **Realized by:** SPEC-0016
- **QA:** `qa` agent run scoped to this requirement

## 1. Statement

The pipeline that makes a 9:16 narrated reel must make, **from the same
post source**, the other formats the house publishes:

| Format | Canvas | What the platform takes besides the MP4 |
|---|---|---|
| Instagram reel (`reel`) | 9:16, 1080 × 1920 | caption, alt text |
| TikTok (`tiktok`) | 9:16, 1080 × 1920 | the reel's MP4 |
| YouTube Short (`short`) | 9:16, 1080 × 1920, ≤ 3 min | title, description |
| Long YouTube video (`youtube`) | 16:9, 1920 × 1080 | title, description with chapters, `.srt`, thumbnail 1280 × 720 |

The format is a field of the post; the canvas flows from it into the scene
(guion-reel), the gadgets (guion-gadget's `Contexto`) and the gate, which
checks each format with its own bands and limits. Everything a platform
refuses that a machine can count is checked before anyone uploads.

## 2. Rationale

Until now the engine, the template and the gate assumed 9:16, and the skill
`hacer-video` told agents to fake a long video as chained reels. The owner
asked (2026-10-20) for the same post to produce Shorts and long videos, so
the work done once (voice, scene, sound, provenance) is published wherever
the audience is.

## 3. Acceptance criteria

- **AC1. The format.** `formato` in `reel.toml` (`[reel]`) and/or
  `escena.toml` (top level): `reel` (default), `tiktok`, `short`, `youtube`;
  an unknown name fails listing the valid ones; the two files must agree;
  `guion reel --formato` overrides them. The format's canvas
  (`Formato::canvas`: 1080 × 1920 or 1920 × 1080, 600 px per unit) is the
  studio's and the gadgets' `Contexto`; a scene binary reads it from
  `GUION_FORMATO`.
- **AC2. Bands per format.** The gate checks format F with the brand's
  profile as it applies to F (`Profile::para`): the profile itself when F
  has the orientation of `[canvas]`; the house's default bands for F
  (subtitle row, safe area; the formula band is dropped) otherwise; and the
  brand's `[formato.<F>]` overrides (canvas, subtitle, safe, formula band,
  max duration) on top. Unknown format names and keys fail by name. The
  defaults live in guion-reel; the longest length is the post's, the
  brand's and the platform's, the shortest.
- **AC3. 9:16 is byte-identical.** A reel made before this requirement gives
  the same MP4 and `eventos.json` bytes, the same gate findings, and an
  `escena.toml` without `formato` saves the same text.
- **AC4. Subtitles.** A long video gets `<name>.srt` from `voz.json`: one
  cue per measured sentence, what is said, its times to the millisecond.
  The gate fails (`srt`) when a cue's times or the count differ from
  `voz.json`.
- **AC5. Chapters.** `youtube.md` names chapters by anchors (a voice line,
  `<line>.fin`, `inicio`, an event of `eventos.json`, ± s); they are emitted
  as description timestamps (`00:00 Title`) and must follow YouTube's rules
  (first at 00:00, at least three, ascending, each ≥ 10 s, the last against
  the video's length). The export refuses them otherwise and the gate reads
  them back from the description (`chapters`).
- **AC6. Thumbnail, title, description.** `--miniatura <t>` (guion-reel's
  `Command`, `guion studio`) writes the frame at `t` resized to 1280 × 720 as
  `miniatura.png` and refuses one over 2 MB or from a canvas that is not
  16:9; guion-publish exports `titulo.txt` (≤ 100 characters) and
  `descripcion.txt` (≤ 5 000 characters and bytes, chapters and provenance
  in words), neither with `<` or `>`. The gate checks all of them
  (`thumbnail`, `yt-title`, `yt-description`, plus the brand's internal
  patterns) when the format is YouTube.
- **AC7. A 16:9 reel passes.** A 16:9 fixture rendered with guion-reel,
  with its voice, texts, `.srt` and thumbnail, passes the gate as `youtube`.

## 4. Constraints & non-goals

- No new crate in the lock: `serde_json` (already the workspace's) is added
  to guion-cli; guion-publish stays std-only.
- `Canvas`, `Reel::new` and `Command::from_args` keep their shapes: the
  account motors (rotorf-sico, fisicobuenfisico) build them with literals.
- Not here: uploading (the owner uploads; OQ-4 in the accounts), the
  platform's own captions editor, re-laying-out a 9:16 scene as 16:9 by
  itself (a scene decides its own layout for each canvas).

## 5. Open questions

- Q1. Burned-in subtitles and the `.srt` both show on a long video when the
  viewer turns captions on. Recommend: keep the brand's burned subtitle
  (it is the design), let the account drop it for 16:9 if it wants.
- Q2. The description limit is 5 000 *characters* in the Help Center and
  5 000 *bytes* in the Data API. Recommend: check both (done) until the owner
  uploads through the API or decides otherwise.
- Q3. Videos over an hour write `H:MM:SS` timestamps (`0:00:00` first); the
  Help Center only shows `00:00`. Unverified; the house's long videos are
  6–15 min.

## 6. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-10-20 | `Formato` lives in guion-reel; YouTube text rules (chapters, title, description) in guion-publish; the gate checks the emitted files | guion-reel owns the canvas and the gate; guion-publish is the platform-text crate (std only) and guion-reel already depends on it; checking what was written, not re-deriving it, is how the caption check works (R-0012 AC7) |
| 2026-10-20 | The profile's top level is the brand's own format; another orientation takes the house's defaults, then `[formato.<F>]` | A brand profile written for 9:16 keeps working unchanged (AC3), and a 16:9 test brand can be small (the fixture's canvas is an override) |
| 2026-10-20 | The formula band does not carry over to another orientation unless the brand writes `formula_band` | It is a design position in the brand's own canvas; guessing it in 16:9 would fail good layouts |
| 2026-10-20 | The `.srt` cues are the voice's sentences with what is said, not the on-screen subtitles | Captions are a transcript for whoever cannot hear; the ≤ 8-word subtitle is a design element that drops what the screen already shows |
| 2026-10-20 | Chapter starts are floored to the second | YouTube jumps to the second written; flooring never starts a chapter after what it names, and the rules are checked on the written seconds |
| 2026-10-20 | The thumbnail is area-resampled in Rust and written as PNG; 2 MB read as 2 000 000 bytes | Deterministic bytes (no scaler flags), and the strictest reading of «2MB» |
| 2026-10-20 | `guion reel` passes the format to the scene in `GUION_FORMATO` | A scene binary takes its own arguments (rotor's `--cuadros`); an environment variable reaches every motor without changing its command line, and an unset variable is the reel |
| 2026-10-20 | Byte identity verified out of tree: *frenar-y-girar* (rotorf-sico, default and `--features guion-reel`) and *como-leer* (fisicobuenfisico), MP4 and `eventos.json` sha256 equal before and after; `guion reel check` findings equal (18/18 PASS both) | AC3 |

## Changelog

- 2026-10-20 — created; realized by SPEC-0016 in the same change.
