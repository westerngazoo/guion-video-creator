# R-0010 — Publishing export: an Instagram post's text, with the platform's limits checked

- **Status:** In review (implemented locally, awaiting the owner's sign-off)
- **Milestone:** M2
- **Owner:** físico buen físico (see project-specifics.md)
- **Created:** 2026-10-14
- **Depends on:** none
- **Realized by:** SPEC-0010
- **QA:** `qa` agent run scoped to this requirement

## 1. Statement

`guion-video-creator` must turn a post's `caption.md` (sections `## Caption`,
`## Hashtags`, `## Alt text`) into the two texts a person pastes in the
Instagram app, `caption.txt` and `alt-text.txt`, and must refuse, before
writing anything, a post Instagram would refuse: more than five hashtags, or a
caption longer than 2 200 characters.

## 2. Rationale

Every reel this framework delivers ends with a person pasting a caption. The
sibling `rotorf-sico` studio wrote that step twice (`tools/caption.py`, then
`tools/herramientas`' `caption` in Rust, byte-identical on its 19 captions),
and found both limits the hard way: a caption with six hashtags, and one
(*rueda*) at 2 304 characters, that the app would cut. Neither limit is about a
brand; @fisicobuenfisico publishes to the same app. The owner decided
(rotorf-sico `docs/decisions.md`, 2026-10-14) that what is reusable lives in its
repo. Publishing itself stays a person's decision: nothing here talks to
Instagram.

## 3. Acceptance criteria

- **AC1. The two texts.** The caption is the `## Caption` section, a blank
  line, and the `## Hashtags` section; the alt text is `## Alt text`. Each
  non-blank paragraph becomes one line (runs of space, tab, CR and LF become
  one space); `**bold**` and `_italic_` lose their marks; paragraphs are
  separated by one blank line. Section names are case-insensitive; a repeated
  section starts over. Input newlines are universal (`\r\n`, `\r`).
- **AC2. Hashtags.** `#` followed by word characters counts as a hashtag, in
  the caption and the hashtags together; more than 5 is a typed error with
  the count and the file.
- **AC3. Length.** The caption as pasted (caption, blank line, hashtags,
  without the file's final newline), counted in Unicode code points, may not
  exceed 2 200; longer is a typed error with the count and the file.
- **AC4. Missing sections.** A missing section is a typed error naming the
  section and the file.
- **AC5. All or nothing.** A failed export writes no file; a successful one
  writes both, each ending in one newline.
- **AC6. A drop-in for the tool it came from.** On every `caption.md` of the
  rotorf-sico repo, the `caption` binary writes the same bytes as
  `tools/herramientas`' `caption` (and so as `tools/caption.py`), and fails on
  the same ones. Word and space classes are Python's `re` / `str` classes
  (Unicode 14.0), from generated tables.

## 4. Constraints & non-goals

- `std` only: the publishing step must not compile a renderer.
- Not here: posting through the Instagram Graph API (rotorf-sico OQ-4 is the
  owner's), choosing a cover, other platforms. Another platform's rules would
  be a sibling module of `instagram`.

## 5. Open questions

- Q1. Keep Python's character classes once no Python tool remains to match?
  They are the reason the tables exist; Rust's `char::is_alphanumeric`
  differs on a few marks (Other_Alphabetic). Recommend keeping them: a caption
  that exported one way should keep exporting the same way.
- Q2. Should the 2 200 count use Instagram's own counting if it ever differs
  from code points (e.g. grapheme clusters for emoji)? Recommend: measure on
  a real post before changing anything.

## 6. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-10-14 | A new crate, `guion-publish`, with an `instagram` module and a `caption` binary | Publishing is its own step after delivery (guion-encode); it shares nothing with rendering |
| 2026-10-14 | The two limits are constants of the module, with the date and source in the docs | 5 hashtags is Instagram's limit since 2025-12-19 (reported by Social Media Today, as the rotorf-sico studio records); 2 200 characters is the app's caption limit |
| 2026-10-14 | Byte equivalence verified on rotorf-sico's 20 `caption.md` (19 export, 1 fails in both) | AC6 |

## Changelog

- 2026-10-14 — created; realized by SPEC-0010 in the same change.
