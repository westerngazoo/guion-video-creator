# R-0017 — Cover framing (`encuadre`): a vertical cover centred in Instagram's crops

- **Status:** In review (implemented, awaiting the owner's sign-off)
- **Milestone:** M2
- **Owner:** the house (every account that publishes vertical reels)
- **Created:** 2026-11-14
- **Depends on:** R-0012 (the gate, its profile and its pixel scan), R-0016 (`Formato`, `Bandas`, `[formato.<name>]`)
- **Realized by:** SPEC-0017
- **QA:** `qa` agent run scoped to this requirement

## 1. Statement

The gate (`guion reel check`) must check, for the vertical formats (`reel`,
`tiktok`, `short`), that the content of the reel's **cover frame** sits
where Instagram shows it: not above the feed's 4:5 crop, not under the Reels
UI, and centred in the profile grid's 3:4 crop. The check is named
`encuadre` in `check.json`. It fails, naming what it measured and the limit,
so the owner never has to re-adjust the grid crop by hand.

## 2. Rationale and what is known about the crops

The owner uploads 1080 × 1920 reels from several accounts and re-adjusts the
profile-grid crop by hand every time, because the content is not centred in
what Instagram shows. Measured on 2026-11-14 across ~95 covers of 5 accounts:
the content's vertical centre ranges from ~680 to ~1100 (medians 842–1070),
and titles often start at y ≈ 250–350, at or above the feed crop's edge.

What Instagram does with a 9:16 reel is **not documented by Meta**. The
figures below are *reportado* by third parties and used as estimates:

| Where | What is shown of 1080 × 1920 | Source |
|---|---|---|
| Profile grid | a centred 3:4 crop, 1080 × 1440, y 240–1680 | <https://posteverywhere.ai/blog/instagram-reels-aspect-ratio>, <https://moda.app/resources/sizes/instagram-reel> |
| Main feed | a centred 4:5 crop, 1080 × 1350, y 285–1635 | <https://posteverywhere.ai/blog/instagram-reels-aspect-ratio>, <https://admakeai.com/blog/instagram-reel-size> |
| Reels viewer | the bottom ~380 px and a column of buttons on the right covered by UI | <https://admakeai.com/blog/instagram-reel-size>, <https://moda.app/resources/sizes/instagram-reel> |

## 3. Acceptance criteria

- **AC1. Defaults.** `Formato::bandas()` carries `encuadre:
  Option<Encuadre>`: for `reel`, `tiktok` and `short`, `min_top = 300`
  (the 4:5 crop's 285 + 15 px), `max_bottom = 1545` (above the Reels UI,
  the subtitle band's bottom edge), `center_y = 960` (the 3:4 crop's
  centre), `max_offset = 60`; for `youtube`, `None`. The vertical default
  safe area's top moves from 250 to 300, the same line.
- **AC2. What is measured.** On the cover frame, the rows spanned by ink
  (the safe-area check's ink: the brand's `safe.ink`, or every token that
  is not background, within `scan.tolerance`), leaving out the subtitle
  band, the profile's `safe.except` rectangles (grown by `safe.bleed`), and
  up to `scan.min_stray − 1` stray pixels at each end. Fewer than
  `scan.min_stray` ink pixels in all is "nothing to frame", a FAIL.
- **AC3. Pass.** A cover whose content starts at or below `min_top`, ends
  at or above `max_bottom`, and is centred within `max_offset` of
  `center_y` passes, and the finding says what was measured.
- **AC4. Fail: top.** Content starting above `min_top` fails; the finding
  names the measured top and `min_top`.
- **AC5. Fail: bottom.** Content ending below `max_bottom` fails; the
  finding names the measured bottom and `max_bottom`.
- **AC6. Fail: centre.** A box centre further than `max_offset` from
  `center_y` fails; the finding names the centre, its offset and the limit.
- **AC7. Overrides.** A brand overrides any of the four keys in its gate
  profile: `[encuadre]` for its own (vertical) format, `encuadre = { … }`
  under `[formato.<name>]` for one format, on top. The house's defaults are
  scaled to the height of the canvas the format is checked on. Unknown
  keys fail by name; `encuadre` under a horizontal format, `[encuadre]` in
  a horizontal brand, and `min_top ≥ max_bottom`, a negative `max_offset`
  or a non-finite value fail at load, naming where.
- **AC8. Horizontal.** For `youtube` the check is not run: the finding
  `encuadre` passes, saying it was not checked.
- **AC9. The cover frame.** The gate measures the MP4's frame at the
  cover's time (`Inputs.cover`, which `guion reel` takes from `[reel]
  cover` in `reel.toml`), or the first frame when there is none; a cover
  past the last frame fails, naming it.

## 4. Constraints & non-goals

- No new dependency; one decode of the MP4 serves this check with the rest.
- Not here: the right-hand button column of the Reels viewer (already a
  `safe.free` rectangle if a brand wants it), horizontal centring, and
  any account's reels. Existing reels whose covers are not centred will
  now FAIL the gate: that is intended.
- The figures in §2 are estimates; when Meta documents them, or the owner
  measures them, the defaults change here and nowhere else.

## 5. Open questions

- Q1. The cover is measured on the delivered MP4, with the subtitle band
  left out, not on a render without the subtitle: the gate has only the MP4,
  and the cover Instagram shows is a frame of it. Recommend: keep it until
  a scene can render a subtitle-less cover the gate can read.
- Q2. The scaffold's title (`[marca.titulo] y`) now defaults to 986, so a
  new account's hook title, alone on its cover, is centred on 960. Existing
  accounts keep their own value.

## 6. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-11-14 | The crop figures (3:4 grid y 240–1680, 4:5 feed y 285–1635, ~380 px of Reels UI) are *reportado* estimates from third parties, not Meta | Meta does not document them; the three sources agree; recorded as estimates so the defaults move when better data arrives |
| 2026-11-14 | Thresholds live in `Bandas` (`Encuadre`) and are scaled to the checked canvas's height | The crops are proportions of the 9:16 frame; a small test canvas (216 × 384) gets the same rule |
| 2026-11-14 | "Content" is the safe-area check's ink (palette tokens that are not background), not "any pixel not matching the background" | Anti-aliased blends and textures match no token (`NONE`); counting them would make a textured background a full-frame box. A cover's title and figures are drawn in palette tokens. Same definition as `safe-area`, one meaning of ink in the gate |
| 2026-11-14 | Strays: up to `scan.min_stray − 1` ink pixels are trimmed at each end of the box | The existing meaning of `min_stray` ("fewer stray pixels than this, in a zone, are codec noise") |
| 2026-11-14 | The cover frame is the MP4's frame at `[reel] cover` (else frame 0), with the subtitle band left out | The gate only reads the MP4; Instagram's default cover is the first frame; the subtitle is the line being said, not the cover's design (Q1) |
| 2026-11-14 | An empty cover frame FAILS | Nothing on the cover is a bad cover; the message points at `[reel] cover` |
| 2026-11-14 | `youtube` emits a passing `encuadre` finding ("not checked") instead of none | `check.json` has the same check names in every format, as `determinism` does when not run |
| 2026-11-14 | Default vertical safe-area top 250 → 300 | Ink between 250 and 300 is outside the 4:5 feed crop, which `encuadre` fails anyway; one line for both. It only applies where a profile takes the house's bands (another orientation), so no brand's own profile changes |
| 2026-11-14 | The scaffold's title baseline 760 → 986 | R-0014 AC7 (the template passes the gate): the template's cover is its title alone; at 760 its box is centred on ~734, at 986 on 960 |

## Changelog

- 2026-11-14 — created; realized by SPEC-0017 in the same change.
