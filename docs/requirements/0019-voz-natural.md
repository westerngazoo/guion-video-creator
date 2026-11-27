# R-0019 — «Voz natural»: prosody, cached takes, fluent blocks, pronunciation

- **Status:** In progress (part 1: prosody, takes and their timing)
- **Milestone:** M2
- **Owner:** the house (every narrated account: fbf, Rotor, computar, …)
- **Created:** 2026-11-26
- **Depends on:** R-0007 (`narrate_lines`, the voice sets the clock), R-0012
  (`voz.toml` → `voz.wav` + `voz.json`), R-0014 (the account contract),
  R-0018 (a cast of voices)
- **Realized by:** SPEC-0019
- **QA:** the tests named in SPEC-0019 §3

## 1. Statement

A narrated reel's voice must sound natural: Piper's prosody noise on (the
owner's preset «C»), whole blocks of lines spoken as one utterance with the
pauses the punctuation asks for, and every word pronounced as written, with
its stress where the tilde puts it. Because a voice with noise is not
reproducible run to run, each utterance is **spoken once and kept** (a
*take*): the clock (`voz.json`) and the track (`voz.wav`) are derived from
the kept takes, so a re-run, a render and the gate's two renders give the
same bytes.

## 2. Rationale

The owner (2026-11-26): the voice of every reel sounds robotic, monotone and
choppy, and accented words are not pronounced right. Diagnosed:

1. **Prosody off.** `guion reel voice` builds `PiperEngine::deterministic`
   (`noise_scale = 0`, `noise_w = 0`): a flat pitch and the predictor's mean
   duration on every phoneme.
2. **Each line is its own utterance,** joined with fixed gaps (`pausa`,
   `antes`).
3. **Lines are short,** written for the subtitles.
4. **English terms are phonemized as Spanish** («byte» → *bite*, «IA» → *ya*,
   «N·m» → *ene punto medio eme*).
5. **Accents.** Unverified until now.

The owner heard preset «C» — `noise_scale` 0.75, `noise_w` 0.9,
`length_scale` 1.0, `sentence_silence` 0.35 — on `es_MX-claude-high` and found
it much more fluid. It is the new natural prosody for every account; each
account keeps its voice model.

Findings that shape this requirement (measured 2026-11-26, piper-tts 1.8.0):

- **Noise breaks R-0007's sentence timing.** On `es-carlfm-x-low` at «C»,
  pauses *inside* a sentence (after a comma or a colon) came out from 0 to
  1.8 s, longer than the 0.35 s between sentences, so R-0007's rule (the
  *n − 1* longest quiet stretches) would cut at a comma. Piper, though, writes
  exactly `int(sentence_silence × rate)` **zero samples** between sentences
  (5 takes × 5 boundaries: every boundary 5 600–5 602 zeros at 16 kHz; no
  other run of zeros of 20 ms or more). Those runs are exact markers.
- **Piper speaks each sentence on its own.** Both builds phonemize a text
  into sentences and run the model once per sentence
  (`PiperVoice.synthesize`). A block of whole sentences gains Piper's own
  pauses and one model load per block, not intonation across sentences; the
  intonation gain is noise («C») and not cutting a sentence across lines.
- **espeak-ng stresses correctly — if the text is composed.** Through Piper's
  own phonemizer, every word of the battery (agudas, graves, esdrújulas,
  público/publico/publicó, ñ, ü) gets its stress on the right syllable when
  the text is NFC. The same text in NFD (a base letter plus a combining
  accent, as macOS file names and some copy-paste sources write it) loses
  it: «público» → *publíco*, «física» → *fisíca*, «kilómetros» →
  *kilometros*. Nothing in guion composed the text. The 145 `voz.toml` in
  rotorf-sico are NFC today, so the audible problem is the flat prosody
  (stress is pitch and length, and both were frozen) plus the misread
  terms; the NFD path is a latent bug fixed here.
- **espeak-ng gets one of the battery wrong:** the conjunction «mas» is
  stressed like «más».

## 3. Acceptance criteria

### Prosody

- **AC1. Presets and parameters.** `voz.toml` `[voz]` takes
  `prosodia = "natural" | "determinista"` and optional `ruido`
  (`noise_scale`), `ruido_w` (`noise_w`), `silencio` (`sentence_silence`, s);
  `lento` stays `length_scale`. `natural` is «C» (0.75, 0.9, 1.0, 0.35);
  `determinista` is R-0007's (0, 0, 1.0, 0.25) and fixes both noises at 0. A
  `[voces.<nombre>]` (R-0018) may override `ruido`, `ruido_w`, `silencio`
  and `lento`; it inherits the rest from `[voz]`. An unknown preset, a
  negative or non-finite number, a `lento` ≤ 0, a noise with `determinista`,
  or a prosody key without `prosodia` fail, naming the table and the key.
- **AC2. The account sets its natural voice.** `cuenta.toml` `[voz.prosodia]`
  takes `ruido`, `ruido_w`, `silencio`: they replace «C»'s numbers for every
  post of that account (the nearest `cuenta.toml` above the post) whose
  `voz.toml` says `prosodia = "natural"`, and `voz.toml`'s own numbers replace
  the account's. The account template writes `prosodia = "natural"` in every
  new post's `voz.toml`: new voices are natural by default.
- **AC3. The legacy voice is untouched.** A `voz.toml` without `prosodia`
  gives the same `voz.wav` and `voz.json`, byte for byte, as before this
  requirement (R-0007, R-0012, R-0018): deterministic Piper, one utterance
  per line, fixed gaps, no takes written. `PiperEngine::deterministic` and
  `PiperEngine::new` keep their meaning.

### Takes: determinism kept through caching

- **AC4. Spoken once.** In a natural (or explicitly `determinista`) voice,
  each utterance's take is stored as `voz.tomas/<key>.wav` (mono 16-bit at
  the voice's own rate, exactly as Piper spoke it) and its key is a stable
  64-bit FNV-1a hash of the voice's identity (the model's `.onnx` and
  `.onnx.json` bytes, the Piper build, the prosody) and the text the voice
  says. Re-running `guion reel voice` with unchanged inputs synthesizes
  nothing and writes the same `voz.wav` and `voz.json` bytes. Editing a
  subtitle, `desde`, `antes`, `pausa`, `inicio` or the global `pausa`
  re-synthesizes nothing; editing one block's text, the model, the build or
  a prosody number re-synthesizes exactly the takes it changes.
- **AC5. Kept with the post.** The takes live in the post
  (`<post>/voz.tomas/`) and is meant to be committed: guion writes
  `voz.tomas/.gitignore` (`!*.wav`) so an account that ignores `*.wav`
  still commits its takes, and `voz.tomas/tomas.txt`, an index (key, line
  ids, identity, text) with no dates, so a take can be found and deleted to
  record it again. After a successful run, takes no line uses any more are
  removed. `guion reel <post>` keeps an existing `voz.wav` as before, and
  when it is missing (a fresh clone, a worktree) it rebuilds it from the
  takes; the scene's two renders (`--twice`) and the gate read the same
  `voz.wav` and `voz.json`.
- **AC6. Times from Piper's exact silences.** A take's sentence boundaries
  are its internal runs of exact zero samples at least
  `0.9 × sentence_silence × rate` long. A line's start and end, and each of
  its sentences, come from them (sentences cut after `.`, `?`, `!` and a
  space, R-0007 AC3). When the runs found are not exactly the sentences
  expected minus one, the voice fails naming the lines, both counts and the
  likely cause, and writes nothing.

### Fluency

- **AC7. Blocks.** Consecutive `[[linea]]` with no blank line between them
  form one block, spoken as one utterance; a blank line starts a new block.
  `sigue = true` joins a line to the block before it, and `sigue = false`
  starts a new one, whatever the layout. A change of voice (R-0018) always
  starts a new block (`sigue = true` across voices fails). Inside a block,
  every line but the last must end a sentence (`.`, `?`, `!`, before any
  closing quote), so its end can be found; otherwise the voice fails naming
  the line.
- **AC8. Pauses follow punctuation.** Inside a block, the pause between two
  lines is the one Piper leaves after the sentence (`silencio`), plus that
  line's own `pausa` and the next line's `antes`; commas, colons and
  semicolons are the model's own pauses. The global `pausa` separates blocks
  only.

### Pronunciation

- **AC9. The accents reach the phonemes.** A battery — agudas, graves and
  esdrújulas with tilde; público/publico/publicó, cómo/como, más/mas; ñ and
  ü — goes through the exact phonemization path of synthesis (the `piper`
  executable with `--debug`, which logs the phonemes it synthesizes), and the
  primary stress mark lands on the expected syllable of each word; in
  context, the stressed form of each pair carries a primary stress and the
  unstressed one does not. The check of stress positions is tested in CI on
  phonemes recorded from piper-tts 1.8.0; the live run needs Piper and a
  Spanish voice (`#[ignore]`, run by hand).
- **AC10. Composed text.** Every text `PiperEngine` sends is composed first:
  a Latin vowel, `n`, `c` or `y` followed by a combining grave, acute,
  circumflex, tilde, diaeresis or cedilla becomes the precomposed letter
  (NFC for the letters Spanish uses). NFD and NFC input give the same
  phonemes. Composed text passes unchanged, so the legacy voice keeps its
  bytes.
- **AC11. Pronunciation lexicon.** `voz.toml` `[voz] lexico = [ … ]` lists
  substitution lists, applied in order (a later entry wins): `"guion:es"`,
  the shipped starter list (gym, tech and racing terms, units), or a TOML
  file relative to the post with a `[lexico]` table; a `[lexico]` table in
  `voz.toml` itself comes last. A term is replaced where it stands as a whole
  word (no letter or digit on either side), longest term first, in one pass;
  a lowercase term also matches its capitalized form. The replacement goes
  to the voice only: `voz.json` (texts, subtitles, sentences) keeps the
  written text. A term or replacement that is empty or holds `.`, `?` or
  `!` (it would move a sentence cut), or a list that cannot be read, fails
  naming the list and the term. Where espeak-ng is wrong (AC9: «mas»), the
  starter list overrides it.

### Regression

- **AC12. Nothing else moves.** Every existing test passes unchanged;
  published reels (their `voz.wav`, `voz.json`, MP4s) are not touched by
  this change, and a post is re-voiced only on purpose (`guion reel voice`,
  or a `voz.toml` edited to opt in).

## 4. Constraints & non-goals

- Piper and ffmpeg stay external processes; no new crates.io dependency (the
  hash and the composition table are plain `std`).
- Not here: phoneme alignment (no CLI of either build exposes it), cutting a
  line in the middle of a sentence inside a block, a seed for Piper's noise
  (onnxruntime's random ops take none), choosing voices, multi-speaker
  models.

## 5. Open questions

- Q1. «C» on `es-carlfm-x-low` (the voice of fbf, Rotor and computar) left
  pauses of 1.3–1.8 s after some commas in 2 of 5 takes. If the owner hears
  it, the account can tame it in `cuenta.toml` `[voz.prosodia]` (a lower
  `ruido_w`), or a bad take can be deleted and spoken again. Decide by ear.
- Q2. `es-carlfm-x-low`'s espeak voice is `es` (Castilian `θ` for «c» and
  «z»). Seseo would need the voice's config to say `es-419`, which changes
  the voice: the owner's call.
- Q3. Should `narrate_screenplay`'s `engine_by_name("piper")` move to the
  natural preset too? It is not used by any account reel today.
- Q4. «C» speaks slower: fbf's `lateral-3d` lasted 28.8 s and lasts 33.0 s
  natural (every line 5–35 % longer), over its 30 s limit. A post re-voiced
  on purpose may need a `lento` below 1.

## 6. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-11-26 | A `voz.toml` without `prosodia` keeps the legacy voice; new posts get `prosodia = "natural"` from the template | Account repos ignore `*.wav`, so every fresh clone and every agent worktree re-voices a post: a silent default change would re-roll the clock of published reels (and a scene that checks its marks against the voice could fail). Opting in is one line, and it is on purpose (AC3, AC12) |
| 2026-11-26 | Takes live in the post (`voz.tomas/`) and are committed; guion writes `voz.tomas/.gitignore` with `!*.wav` | A take with noise cannot be made again: it is source, like a recording. It is small (native rate: 32 kB/s at 16 kHz, 44 kB/s at 22.05 kHz; about 1 MB for a 30 s reel). Without it, a worktree would speak the reel anew and change its clock |
| 2026-11-26 | The folder is `voz.tomas/` («takes»), not a cache | A cache can be thrown away and rebuilt; a take with noise cannot. The name says so, in the Spanish of `voz.toml`, `voz.json` and `voz.wav` |
| 2026-11-26 | The key is FNV-1a 64 of identity + text, as `guion-profesor`'s voice cache | `std`'s `DefaultHasher` is not stable across Rust releases; a cache key must be. 64 bits are plenty for a few dozen takes per post |
| 2026-11-26 | The build in the key is the `piper-tts` release (from its `dist-info`) when `piper` is a Python launcher, else the executable's bytes | A launcher script holds its machine's paths (the shebang, a `PYTHONPATH`), so hashing it would make a committed take useless on another machine with the same release. The release pins the phonemizer and its espeak-ng data; onnxruntime, which pip resolves per machine, only moves the numerics and stays out. A binary release has the same bytes everywhere |
| 2026-11-26 | Sentence and line times come from runs of exact zeros, not from the energy rule of R-0007 | Measured: with noise, in-sentence pauses beat the between-sentence ones, but the zero runs Piper writes are exact and nothing else in speech is a run of zeros |
| 2026-11-26 | Inside a block, lines end on a sentence | Piper speaks sentence by sentence; finding a line's end inside a sentence would need phoneme alignment, which no CLI exposes (piper-tts 1.8 gives it in Python only, with a model patched through the `onnx` package) |
| 2026-11-26 | The block boundary is a blank line in the source, or `sigue` | The owner's rule. Every existing `voz.toml` puts a blank line between its lines, so an opted-in legacy post keeps one line per utterance until its author joins lines on purpose |
| 2026-11-26 | A table composes the Latin letters with combining marks; the `unicode-normalization` crate is not added | Spanish needs eleven letters (á é í ó ú ü ñ and capitals, plus ç, è…); a std table covers them in a few lines with no dependency. A text with other combining marks passes as it is |
| 2026-11-26 | Lexicon entries may not hold `.`, `?`, `!` | The voice text and the written text must cut into the same sentences: `voz.json` keeps the written sentences with the voice's times |

## Changelog

- 2026-11-26 — created; part 1 (AC1–AC6, AC12) realized by SPEC-0019.
