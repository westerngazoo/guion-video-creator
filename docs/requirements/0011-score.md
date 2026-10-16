# R-0011 — A reel's sound design as data (`sonido.toml` → `audio.wav`)

- **Status:** In review (implemented locally, awaiting the owner's sign-off)
- **Milestone:** M2
- **Owner:** físico buen físico (see project-specifics.md)
- **Created:** 2026-10-16
- **Depends on:** R-0007 (`voz.json`, `read_wav_pcm16`), R-0008 (`synth`), R-0009 (`eventos.json`)
- **Realized by:** SPEC-0011
- **QA:** `qa` agent run scoped to this requirement

## 1. Statement

`guion-audio` must turn a score written as data (`sonido.toml`), the
events a scene wrote (`eventos.json`) and, when the score has a voice, the
narration (`voz.wav`, `voz.json`) into a reel's `audio.wav`: drones, tones
that follow a series, hits at named instants, one room for the buses, the
bed ducked under the voice, and the mix. **The instruments are the
caller's**: a brand registers them by name; none lives in this crate.

## 2. Rationale

The owner decided on 2026-10-16 that everything generic about narrated
reels lives in guion-video-creator (rotorf-sico `docs/proceso.md`). The
engine was written in the rotorf-sico motor (`src/sonido.rs`, 1404 lines)
around two brands' instruments (PMMO's `liber_sonido.rs`, Rotor Físico's
`rf_sonido.rs`). Its parser, time expressions, buses, ducking and mix carry
no brand; only the instrument list did. Splitting there lets any brand do
`sonido.toml` + `eventos.json` → `audio.wav` with its own sounds.

## 3. Acceptance criteria

- **AC1.** No brand: an empty `Instruments` has no names; the whole format
  (`[sonido]`, `[voz]`, `[mezcla]`, `[agachar]`, `[[dron]]`, `[[seguidor]]`,
  `[[golpe]]`) parses with the caller's instruments, a table with no
  `instrumento` plays the caller's default for its kind, and buses default
  to `dron` (drones, followers) and `golpe` (hits).
- **AC2.** One clock: an instant is an event, `evento ± s` (sign between
  spaces, or attached when the whole is no event; an exact name wins),
  `fin`, a number, or `a..b @ p`. Series are linear inside, flat outside.
  The first of a repeated event wins; numbers parse correctly rounded.
- **AC3.** Fail loud, located (CLAUDE.md §6): an unread key is an error
  naming the table and the valid keys; a bad event, a backwards envelope,
  a missing series, `dur` with `hasta`, a pan out of [−1, 1], a duck with
  no voice each name their table.
- **AC4.** Repeated hits: a list in `en` or `repetir` (n with `hasta`,
  `cada`/`cada_cuadros` with `hasta`, n with `cada`); per-hit lists of the
  right length, `paso` adding `i ·` step to any number the instrument
  reads, an envelope scaling each hit's gain at its instant; more than 100
  hits per table is refused.
- **AC5.** Instruments by name: an unknown name lists the registered ones in
  registration order; each piece's noise stream is `seed_for(seed, n)` with
  `n` fixed by its table and position (adding a table leaves the others'
  noise unchanged); a sound with an extent reports its end in `golpes.tsv`.
- **AC6.** Determinism and placement: the same score renders the same mix
  twice, a different seed changes it; a hit lands at `round(t · 48000)`; a
  moving pan follows the instrument's curve.
- **AC7.** Ducking: by lines the bed sits `depth` dB lower inside a line and
  opens between lines; `unir` closes gaps shorter than it.
- **AC8.** Bed level: `cama` sets the buses' peak against the voice's
  (peak 1); `cama_rms` sets the bed's RMS in dB against the voice's.
  Both at once, or a peak over 1, are refused.
- **AC9.** A post folder → `audio.wav` (48 kHz, stereo, 16 bit), and with a
  stems folder the voice, bed, each bus dry and wet as `f32` and
  `golpes.tsv`; on any failure nothing is written; two runs give the same
  bytes.
- **AC10.** Byte identity with the engine it replaces: every rotorf-sico
  post with a `sonido.toml` renders the same `audio.wav` md5 (and stems)
  before and after the move (outside CI, SPEC-0011 §4).

## 4. Constraints & non-goals

- Behind the feature `score` (adds `toml`, `serde`, `serde_json`), so the
  synthesis alone still adds no crate (R-0008 AC11).
- The score's keys and error messages stay in Spanish: the scores already
  written are the contract.
- Not here: instruments, loudness (R-0005), the voice (R-0007).

## 5. Open questions

- Q1. Should `guion reel` (the one-command pipeline) call `render_post`
  with a brand's registry? Recommend yes, once a brand can be named from
  `reel.toml`.

## 6. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-10-16 | Instruments are builders registered by `(Kind, name)` that read their own keys through `Params` and return a `Sound` | The engine keeps the common keys (gain, pan, room, bus, seed, timing); typo detection still covers instrument keys, since every key goes through the same tracked table |
| 2026-10-16 | Defaults (`tritono`, `giro` in rotorf-sico) are set by the caller | A default instrument is a brand choice |
| 2026-10-16 | `eventos.json` is read in file order with repeated keys kept; numbers with `float_roundtrip` | The studio's reader did the same; anything else could move an event by one ULP and a hit by one sample |
| 2026-10-16 | Spanish keys and messages | The scores and their authors are Spanish; renaming would break every post |

## Changelog

- 2026-10-16 — created; realized by SPEC-0011 in the same change.
