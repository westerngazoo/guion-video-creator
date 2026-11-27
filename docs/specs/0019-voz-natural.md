# SPEC-0019 — «Voz natural»

- **Status:** In review (implemented in two parts, awaiting the owner's ear)
- **Realizes:** R-0019
- **Created:** 2026-11-26
- **Depends on:** SPEC-0007 (cue-list narration), R-0018 (the cast)
- **Module(s):** `guion-audio`, `guion-cli` (`reel::voz`), `guion-cuenta`,
  `templates/cuenta/plantilla/voz.toml`

## 1. Motivation

Realizes R-0019 in two changes: (1) prosody, takes and their timing; (2)
blocks, the pronunciation lexicon, composed text and the accent battery.

## 2. Design

### Module layout

```
guion-audio/src/
├── engine.rs     # + Prosody (NATURAL «C», DETERMINISTIC), PiperEngine::with_prosody,
│                 #   impl Voice for PiperEngine (identity: model bytes + build + prosody),
│                 #   piper_build (the piper-tts release, else the binary's bytes),
│                 #   resample_pcm16 (public)
├── take.rs       # Voice (trait), Fnv (FNV-1a 64), fingerprint_file, TakeStore, TakeUse
├── utterance.rs  # Utterance, narrate_utterances, cut_take, LineCut, TakeNarration
├── lines.rs      # (R-0007) validate() and silence() shared with utterance.rs
├── compose.rs    # compose(): NFC for the Latin letters with combining marks (part 2)
├── lexicon.rs    # Lexicon (whole words, longest first, one pass), Lexicon::starter_es() (part 2)
└── error.rs      # + TakeTiming
guion-audio/tests/acentos.rs # the accent battery: recorded (CI) and live through Piper (part 2)
guion-cli/src/reel/voz.rs   # prosodia, ruido, ruido_w, silencio; legado() | natural();
                            # blocks (blank lines, sigue), [voz] lexico, [lexico] (part 2)
guion-cuenta/src/contrato.rs # Voz.prosodia: ProsodiaCuenta { ruido, ruido_w, silencio }
```

### Blocks (AC7)

`blancos_entre_lineas(source)` reads the raw `voz.toml`: for each `[[linea]]`
header after the first (spaces inside the brackets and a trailing comment
allowed), whether a blank line, or another table header, came since the one
before; lines inside a `"""`/`'''` string are skipped. Its count must match
the parsed lines, else the file fails asking for `sigue`. A line starts a
block when it is the first, its voice differs from the previous line's, its
`sigue` is `false`, or (no `sigue`) a blank line precedes it.

### What the voice says (AC10, AC11)

`spoken = lexicon.apply(compose(text))`; `PiperEngine` composes again before
writing stdin (idempotent) and sets `PYTHONIOENCODING=utf-8`. The lexicon is
`[voz] lexico` in order (`"guion:es"` → `Lexicon::starter_es()`, else a TOML
file relative to the post whose only table is `[lexico]`), then `voz.toml`'s
`[lexico]`; later entries win. A lowercase term also matches capitalized,
except when it maps to Piper's raw phonemes (`[[…]]`): at a sentence start
piper-tts 1.8.0 appends a raw block to the previous sentence, which would
lose the cut.

### Phonemes (AC9)

`PiperEngine::phonemes(text)` runs Piper with `--debug` and reads the last
`phonemes=[[…], …]` of its log (Python's repr: one list per sentence, one
code point per quoted item), composed. The C++ build logs no such line: the
call fails saying so.

### The two paths of `guion reel voice`

```
voz.toml ──leer_guion──▶ Guion { prosodia: None | Natural | Determinista, … }
   None          ─▶ legado(): PiperEngine::deterministic per line,
                    narrate_cast + SilenceRule::PIPER        (R-0007/R-0018, unchanged)
   Some(preset)  ─▶ natural(): prosodia() per voice
                    (preset → cuenta.toml [voz.prosodia] → [voz] → [voces.x], lento)
                    → PiperEngine::with_prosody → narrate_utterances(…, TakeStore(voz.tomas/))
both          ─▶ desde check → voz.json, voz.wav → (natural) store.settle()
```

### A take (AC4)

`key = FNV-1a 64(identity ‖ "\n" ‖ text)` as 16 hex digits;
`voz.tomas/<key>.wav` holds the take at the voice's own rate (mono 16-bit,
canonical 44-byte header, written beside and renamed). The identity of a
`PiperEngine` is

```
piper <name> model=<FNV of .onnx bytes, FNV of .onnx.json bytes> build=«<release>»
      noise_scale=… noise_w=… length_scale=… sentence_silence=…
```

computed once per engine (`OnceCell`). `settle(used)` removes every
`<16 hex>.wav` not used, and writes `.gitignore` (`!*.wav`) and `tomas.txt`
only when their text changed.

### Cutting a take (AC6)

`cut_take(take, counts, sentence_silence)`: the internal runs of samples
equal to 0, at least `max(1, ⌊0.9 × sentence_silence × rate⌋)` long, that
touch neither end. There must be exactly `Σ counts − 1`. The take's sentences
are the stretches between them; line *i* takes `counts[i]` of them, its
speech runs from the first one's start to the last one's end, and the zeros
after it (inside the block) are its `pause_after`.

### Layout (AC4, AC8)

As R-0007 (`round_ties_even` silences, times = samples / rate):

```
lead_in · [before · speech(line) · after + (pause_after | gap | 0)] …
```

`pause_after / native_rate` inside a block, the pacing's `gap` between blocks,
nothing after the last line. Each line's speech is the take's slice,
resampled to 48 kHz (`resample_pcm16`: nothing to do at the same rate, else
ffmpeg bit-exact). A sentence's times are its slice's offsets over the native
rate from the line's start; the last sentence ends with the line.

## 3. Acceptance-criteria checklist

| AC | Test |
|----|------|
| AC1 | `guion-audio` `engine::tests::the_natural_preset_is_c`; `guion-cli` `reel::voz::pruebas::la_prosodia_va_por_capas`, `reel::voz::pruebas::la_prosodia_invalida_falla_por_nombre` |
| AC2 | `guion-cuenta` `tests/r0019_prosodia.rs` (`ac2_the_contract_carries_the_natural_voice`, `ac2_a_bad_natural_voice_fails_by_name`, `ac2_a_new_post_asks_for_the_natural_voice`); `reel::voz::pruebas::la_cuenta_pone_su_voz_natural` |
| AC3 | `reel::voz::pruebas::sin_prosodia_la_voz_es_la_de_antes`; the R-0007, R-0012 and R-0018 tests, unchanged |
| AC4 | `guion-audio` `tests/takes.rs::ac4_spoken_once_then_read_back`; `engine::tests::the_build_is_the_release_not_the_launcher`; `take::tests::the_hash_is_fnv1a_64`; `reel::voz::pruebas::la_voz_natural_se_graba_una_vez` |
| AC5 | `tests/takes.rs::ac5_the_store_is_kept_with_the_post`; `reel::voz::pruebas::la_voz_natural_se_graba_una_vez` |
| AC6 | `tests/takes.rs::ac6_sentences_are_cut_at_the_zeros`, `ac6_a_take_that_does_not_cut_fails_by_name` |
| AC7 | `tests/takes.rs::ac7_a_block_is_one_take_with_each_line_on_the_clock`, `ac7_a_line_mid_sentence_inside_a_block_fails_before_speaking`; `reel::voz::pruebas::los_bloques_salen_del_renglon_en_blanco_y_de_sigue`, `sigue_invalido_falla_por_linea`, `un_bloque_se_dice_de_un_tiron`, `la_plantilla_de_la_casa_se_lee` |
| AC8 | `tests/takes.rs::ac7_a_block_is_one_take_with_each_line_on_the_clock`; `reel::voz::pruebas::un_bloque_se_dice_de_un_tiron` |
| AC9 | `tests/acentos.rs::ac9_the_recorded_battery_carries_each_stress` (CI); `ac9_live_the_accents_reach_the_phonemes` (`#[ignore]`, by hand with Piper); `engine::tests::the_phoneme_log_reads_back` |
| AC10 | `compose::tests::nfd_spanish_composes_and_nfc_passes`; `ac9_live_the_accents_reach_the_phonemes` (NFD and NFC give the same phonemes); `reel::voz::pruebas::el_lexico_cambia_lo_que_dice_la_voz_no_lo_escrito` |
| AC11 | `lexicon::tests::*`; `reel::voz::pruebas::el_lexico_cambia_lo_que_dice_la_voz_no_lo_escrito`; `guion-cuenta` `ac2_a_new_post_asks_for_the_natural_voice` (the template's `lexico`) |
| AC12 | `cargo test --workspace` |

End to end, outside CI (Piper 1.8.0, `es-carlfm-x-low`, this Mac): a copy of
fbf's `lateral-3d` with `prosodia = "natural"` speaks 9 takes (880 kB) and a
second run speaks none, in 0.6 s, with the same `voz.wav` and `voz.json`
md5; the same post without `prosodia` gives the committed `voz.json` byte
for byte and writes no takes.

Part 2, by hand: the live accent battery passes with `es-carlfm-x-low`
(espeak voice `es`), `es_MX-claude-high` and `es_AR-daniela-high` (`es-419`).
Before/after samples of the same lines, blocks and the starter lexicon
included, for fbf (`lateral-3d`), Rotor (`la-frenada`) and computar
(`que-es-un-algoritmo`): every block cut at its zeros (3, 3 and 6 takes for
9, 8 and 8 lines).
