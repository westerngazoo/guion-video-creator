# R-0020 — «Voz externa»: a voice guion runs but does not link (Qwen3-TTS)

- **Status:** In review (awaiting the owner's ear on the first reel)
- **Milestone:** M2
- **Owner:** the house (first: one narrated reel, redone as the pilot)
- **Created:** 2026-11-27
- **Depends on:** R-0019 (takes, blocks, cuts at exact zeros), R-0012
  (`voz.toml` → `voz.wav` + `voz.json`)
- **QA:** `cargo test -p guion-audio command::`, `cargo test -p guion-cli externa`,
  `python3 -I -m unittest discover -s tools/voz-qwen`

## 1. Statement

A reel's main voice can be an **external program**: guion runs it the way it
runs Piper, hands it each block's sentences, and keeps what it said as a take
(R-0019). Everything downstream is unchanged: the cuts at exact zeros, the
clock, `voz.wav` and `voz.json`, and the gate's two identical renders.

## 2. Rationale

On 2026-11-27 the owner listened to R-0019's best Piper voice («C» on
`es_MX-claude-high`) and rejected it: «se escucha super mal muy robotizada
lenta no natural». He pointed at the two defects: phrases cut where a person
would not, and «la IIII… a». Piper's prosody tops out there, so the owner
said «vamos cambiando a qwen».

He then chose, by ear, one of three voices designed with Qwen3-TTS
VoiceDesign 1.7B: «la 1, fíjala».

A designed voice changes with every text. «Fixing» it means cloning:
Qwen3-TTS Base takes the chosen sample and its transcript as a reference
(in-context cloning), and every take is spoken in that voice.

Alternatives considered:

| Option | Why not |
|---|---|
| Keep Piper, tune further | The owner's ear rejected the best of it |
| A cloud TTS (ElevenLabs and the like) | The accounts' rule: AI runs on local or internal infrastructure, never an external service; the script would leave the machine |
| XTTS-v2, F5-TTS | Their weights are non-commercial (CPML; CC-BY-NC) |
| **Qwen3-TTS** (chosen) | Apache-2.0, runs locally on Apple MPS, speaks Spanish, and clones a voice from a short clip |
| Linking Qwen into guion (Rust bindings to PyTorch) | A heavy native dependency for one engine. A program behind a contract keeps guion small and lets the next engine plug in the same way |

## 3. Acceptance criteria

- **AC1. The contract.** `CommandVoice` (guion-audio) runs `programa
  argumentos…` in the folder of the voice's file. It writes the utterance's
  sentences to stdin, one per line, as `split_sentences` cuts them, and
  substitutes `{salida}` (the WAV to write) and `{silencio}` (s). A mono
  16-bit WAV with exit status 0 is the take. Anything else is an error
  carrying the end of stderr. Arguments without `{salida}`, or a silence
  that is not positive, are refused when the voice is built.
- **AC2. Identity.** A take's key is `programa` and `argumentos` **as
  written** (unexpanded: no machine's paths), the silence, and the FNV-1a 64
  of each `identidad` file's bytes (reference clip, transcript, scripts,
  weights). A missing identity file is an error, never an empty hash.
  Same files, same takes; a new reference clip, new takes.
- **AC3. `voz.toml`.** `[voz] externa = "<file>"`, relative to the post,
  makes the main voice external. It requires `prosodia = "natural"` (takes)
  and refuses Piper's `ruido`, `ruido_w`, `silencio` and `lento`. The
  voice's file has exactly `programa`, `argumentos`, `silencio` and
  `identidad`, with `${VAR}` from the environment. An unknown or missing
  key, or an unset variable, fails by name.
- **AC4. The Qwen voice** (`tools/voz-qwen/voz_qwen.py`) speaks the whole
  block in one go, cloned from a reference clip. It finds the sentence cuts
  by aligning the text's punctuation with the take's pauses
  (`alinear.py`). On the first takes a comma or colon paused as long as a
  period, so the longest pauses are not the cuts. It writes each cut as
  exact zeros (≥ the silence, the voice's own pause when longer) and no
  other zero sample. A take whose cut lands far from its sentence end, or
  whose sentence runs at a rate far from the take's, is rejected and
  spoken again with the next seed (4 tries).

## 4. Constraints & non-goals

- Only `[voz]`: the cast voices of R-0018 stay Piper (no account needs an
  external cast yet).
- guion does not install Python, qwen-tts or the model. The account's
  voice file names them through the environment.
- No speech recognition: the alignment's rate check catches a skipped or
  repeated sentence, but not a swapped word. The owner's ear stays the last
  check.

## 5. Decision log

- 2026-11-27 — Qwen3-TTS-12Hz-0.6B-Base (2.5 GB, the size the owner OK'd),
  not 1.7B-Base (4.5 GB): to try first; revisit if the clone sounds worse
  than the designed sample.

## Changelog

- 2026-11-27 — created and realized (AC1–AC4).
