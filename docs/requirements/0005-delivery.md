# R-0005 — Delivery: an `.mp4` a phone shows as it was drawn

- **Status:** In review
- **Milestone:** M2
- **Owner:** físico buen físico (see project-specifics.md)
- **Created:** 2026-10-06
- **Depends on:** R-0003 (frames on disk)
- **Realized by:** SPEC-0005
- **QA:** `qa` agent run scoped to this requirement

> Numbering: the first of the `R-0005+` block earmarked for M2 delivery
> (R-0003 §4, R-0006 numbering note).

## 1. Statement

`guion-encode` must turn frames into a vertical `.mp4` that the platforms
show with the colours that were drawn, starts playing before it is fully
downloaded, and carries audio at the loudness the platforms normalize to.
Frames may come from disk (numbered PPM, as today) or from memory, and
frames computed in parallel must produce the same file as frames computed
in sequence.

## 2. Rationale

Three defects and one cost, all measured on published reels in the sibling
`rotorf-sico` studio, which solved them locally first:

- **Colour.** Without an explicit matrix ffmpeg converts RGB→YUV with
  BT.601 and leaves the stream untagged; phones decode HD as BT.709. A brand
  gold (227, 169, 58) arrived as (232, 167, 50).
- **Start-up.** `+faststart` was only set when audio was present; a silent
  render put the index after the data.
- **Loudness.** Platforms normalize to about −14 LUFS. On audio with ~23 dB
  crest factor (drum hits), one `loudnorm` pass followed by a fixed −4 dBFS
  limiter left reels at −15.4 to −15.9 LUFS; `loudnorm` alone, once in AAC,
  landed at −14.3 to −14.8 LUFS with a true peak anywhere from −1.2 to
  −0.1 dBTP.
- **Cost.** A 45 s reel at 1080×1920 is ~8 GB of PPM written and read back.
  Rendering is a pure function of the frame index, so it parallelizes, but
  only an ordered writer keeps the output deterministic.

Fixing these once here serves every creator on guion, and lets the studio
drop its private copy.

## 3. Acceptance criteria

- **AC1.** Every `.mp4` from `guion-encode` is tagged BT.709 (primaries,
  transfer, matrix) in TV range.
- **AC2.** A flat colour round-trips within ±3 per channel when decoded as
  BT.709, for brand-like colours (gold, near-black, paper, red, blue).
- **AC3.** The `moov` box precedes `mdat` in every output, with or without
  audio.
- **AC4.** Frames pushed from memory produce the same decoded video as the
  same frames written as PPM and encoded from disk.
- **AC5.** Ordered parallel encoding with 1 and with N threads produces
  byte-identical files containing every frame.
- **AC6.** Muxing audio "at loudness" yields −14 LUFS ± 0.5 integrated and
  ≤ −1 dBTP true peak measured on the output (EBU R128), returns that
  measurement, and is deterministic (same inputs, same bytes).
- **AC7.** Failures are typed and located (CLAUDE.md §6): odd or zero
  dimensions, bad fps, a frame of the wrong size (naming the frame, its size
  and the expected size), and a render error (naming the frame). No failure
  leaves an output or temporary file behind.

- **AC8.** Ordered parallel encoding can give each worker its own state,
  built once inside that worker (an engine's scratch canvas, such as
  `motoreel`'s `PpmSink` and its directory). The output equals the stateless
  form's. A failure to build it is reported as that worker's first frame,
  naming the worker, and leaves no file.

## 4. Constraints & non-goals

- ffmpeg stays an external process (found on `PATH`, or `GUION_FFMPEG`);
  no FFI, no new crate dependencies.
- `guion-encode` does not depend on `motoreel`: it takes RGB bytes, so any
  engine can feed it.
- Not here: audio synthesis or narration (`guion-audio`), format presets
  beyond vertical, the CLI surface for the new entry points.

## 5. Open questions

- Q1. Should `guion encode` (CLI) switch from PPM-on-disk to the streaming
  path by default? Recommend yes once `motoreel` can rasterize to memory
  (today its `PpmSink` writes files).
- Q2. Should the loudness target be a screenplay field (`meta.loudness`)?
  Recommend the platform default (`LoudnessTarget::REELS`) until a creator
  needs another.

## 6. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-10-06 | One set of video flags (`ffmpeg::VIDEO`) for every output | The colour and faststart fixes cannot drift between entry points |
| 2026-10-06 | No partial output: write to `.<name>.parcial.mp4`, rename on success | CLAUDE.md §6 |
| 2026-10-06 | Loudness by measured rounds (loudnorm 2-pass → exact gain + 192 kHz limiter → AAC → measure, ≤ 8 rounds) | Neither a fixed limiter nor `loudnorm` alone met both bounds on high-crest audio (§2); measuring the AAC output is the only check that counts |
| 2026-10-06 | ffmpeg-backed tests fail, not skip, without ffmpeg; CI installs it | A delivery criterion that cannot be checked is not met |
| 2026-10-06 | AC8: per-worker state (`encode_ordered_with`), built inside the worker, so it need not be `Send` | The first consumer (the rotorf-sico studio) renders through `motoreel::PpmSink`, which owns a scratch directory; a sink per frame reloads every font per frame, and a shared one would serialize the threads |

## Changelog

- 2026-10-06 — created; realized by SPEC-0005 in the same change.
- 2026-10-06 — AC8 added (per-worker state).
