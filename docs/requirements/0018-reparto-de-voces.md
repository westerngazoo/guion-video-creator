# R-0018 — A cast of voices: one voice per line in `voz.toml`

- **Status:** In review (implemented, awaiting the owner's sign-off)
- **Milestone:** M2
- **Owner:** the house (first user: rotorf-sico's «El Maistro Ñero», two voices per reel)
- **Created:** 2026-11-18
- **Depends on:** R-0007 (`narrate_lines`, the voice sets the clock), R-0012 (`voz.toml` → `voz.wav` + `voz.json`)
- **Realized by:** this requirement (§4); small enough not to need its own spec
- **QA:** the tests named in §5

## 1. Statement

A narrated reel must be able to speak **some lines with another voice**: a
`[voces.<nombre>]` table in `voz.toml` declares a voice (its Piper model and
its `lento`), and a `[[linea]]` with `voz = "<nombre>"` is spoken by it. A
line without `voz` keeps the reel's voice (`[voz]`). Everything else (the
clock, the pauses, the sentence timing, the subtitles, `voz.json`, the gate)
stays as it is.

## 2. Rationale

Until now a reel had one voice: `[voz] modelo` and one `PiperEngine` for all
the lines. rotorf-sico's new account «El Maistro Ñero» (2026-11-18) is a
dialogue: a stiff professor (a Spain-Spanish voice, `es_ES-davefx-medium`)
gives the academic answer and the Maistro (`es_MX-ald-medium`, `lento` 1.1)
explains it again. Two account-level workarounds were rejected:

- synthesizing two `voz.wav` and splicing them: the clock (`voz.json`) and
  the sentence timing would have to be redone outside guion, which is
  exactly what R-0007 moved here;
- an engine that picks its voice by looking at the text: hidden coupling,
  and wrong for any line both characters say.

The per-line voice is the smallest change that keeps one clock.

## 3. Acceptance criteria

- **AC1. The cast.** `voz.toml` accepts `[voces.<nombre>]` with `modelo`
  (required) and `lento` (optional, default `[voz] lento`). A `[[linea]]`
  accepts `voz = "<nombre>"`; without it, the line is spoken by `[voz]`.
- **AC2. Fail loud, write nothing.** An unknown `voz` name fails naming the
  line and the voices that exist; a `[voces.x]` without `modelo` or with an
  unknown key fails naming the table and the key. Every voice's model file
  is checked before anything is spoken; on any failure neither `voz.wav` nor
  `voz.json` is written.
- **AC3. One clock.** `guion_audio::narrate_cast(lines, engines, pacing,
  rule)` speaks line *i* with `engines[i]` and lays the lines out exactly as
  `narrate_lines` does (the same `lead_in`, gaps, `antes`/`pausa` and
  sentence timing); with the same engine on every line it returns what
  `narrate_lines` returns, sample for sample. `narrate_lines` is
  `narrate_cast` with one engine repeated.
- **AC4. One engine per line.** `narrate_cast` fails, with nothing
  produced, when the number of engines differs from the number of lines.
- **AC5. No change without a cast.** A `voz.toml` without `[voces]` and
  without `voz` gives the same `voz.wav` and `voz.json`, byte for byte, as
  before. `voz.json` does not name each line's voice: a scene knows its
  lines by `id`, and its readers (`guion-reel`'s `Narration`, the score)
  deny unknown fields.

## 4. Realization

- `crates/guion-audio/src/lines.rs`: `narrate_cast`; `narrate_lines`
  delegates to it.
- `crates/guion-cli/src/reel/voz.rs`: `[voces.*]` and `[[linea]] voz`; one
  deterministic `PiperEngine` per voice, all found before speaking. The
  `[voice] model` of `reel.toml` is still compared with `[voz] modelo` (the
  reel's main voice).

The voices of a cast can have different native rates (16 or 22.05 kHz):
each is resampled to the track's 48 kHz as before (R-0007 AC1).

## 5. Traceability

| AC | Test |
|---|---|
| AC1 | `guion-cli` `reel::voz::pruebas::reparto_por_linea` |
| AC2 | `reel::voz::pruebas::reparto_invalido_falla_por_nombre`, `reel::voz::pruebas::una_voz_que_falta_no_escribe_nada` |
| AC3 | `guion-audio` `tests/lines.rs` `cast::ac3_each_line_by_its_own_voice`, `cast::ac3_one_voice_cast_is_narrate_lines` |
| AC4 | `tests/lines.rs` `cast::ac4_engines_and_lines_must_match` |
| AC5 | the R-0007 and R-0012 tests, unchanged and green; `reel::voz::pruebas::claves_desconocidas_fallan` |

## 6. Decision log

- 2026-11-18 — The voice of each line is not written to `voz.json` (AC5):
  adding a field would break the readers that deny unknown fields and change
  the bytes of every published reel's timeline, for no reader that needs it.
- 2026-11-18 — No `speaker` (multi-speaker Piper models) yet: the first cast
  uses single-speaker voices. It goes in `[voces.x]` when a reel needs it.
