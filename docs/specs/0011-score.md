# SPEC-0011 — A reel's sound design as data

- **Status:** In review
- **Realizes:** R-0011
- **Created:** 2026-10-16
- **Depends on:** SPEC-0007, SPEC-0008
- **Module(s):** `guion-audio::score` (feature `score`)

## 1. Motivation

Realizes R-0011 by moving the rotorf-sico motor's `src/sonido.rs` here,
minus its instruments. The motor keeps `liber_sonido.rs` and
`rf_sonido.rs`, each with a `registrar(&mut Instruments)`, and a 100-line
`sonido.rs` that builds the registry.

## 2. Design

```
score/
├── mod.rs         # format docs, RATE, voice_lines_from_json, render_post → PostOutput
├── timeline.rs    # Timeline (eventos.json, ordered), Series, time expressions
├── params.rs      # Params: key-tracked table, per-hit lists, paso, duration, frequency
├── instrument.rs  # Kind, Ctx, Rendered, Sound, sound/sound_lasting, Instruments
├── parse.rs       # Score::parse → RoomCfg, VoiceCfg, MixCfg, DuckCfg, Piece, Source
└── mix.rs         # render → Output (mix, voice, bed, dry, buses, report), hits_tsv, write_f32, merge_lines
```

### Use (a brand)

```rust
use guion_audio::score::{render_post, sound, Instruments, Kind};
let mut r = Instruments::new();
r.add(Kind::Hit, "bell", |d| {
    let (f0, dur) = (d.frequency(220.0)?, d.duration(3.0)?);
    Ok(sound(move |cx| my_bell(f0, dur, cx.noise).into()))
});
r.add(Kind::Drone, "hum", |d| { let f0 = d.positive("f0", 55.0)?; Ok(sound(move |cx| my_hum(cx.len, f0).into())) });
r.set_default(Kind::Drone, "hum");
let out = render_post(Path::new("content/posts/x"), &r, None, None)?;   // → x/audio.wav
```

`Ctx` gives a sound the track length, its own `Noise` (`seed_for(score seed, n)`),
that seed (to split it), and a follower's series per sample. Noise streams:
hits `(table + 1) · 100 + i`, drones `1 000 000 + i`, followers
`2 000 000 + i`, the bus room `3 000 000`/`+1`, the voice room `3 000 010`/`+1`
(unchanged from the motor, so the bytes are too).

## 3. Acceptance-criteria checklist

| AC | Test |
|----|------|
| AC1 | `tests/score.rs::ac1_no_brand_and_the_whole_format_parses` |
| AC2 | `tests/score.rs::ac2_times_are_named_events` |
| AC3 | `tests/score.rs::ac3_unknown_keys_and_errors_name_their_table` |
| AC4 | `tests/score.rs::ac4_repeated_hits_lists_steps_and_repetir` |
| AC5 | `tests/score.rs::ac5_instruments_by_name_seeds_by_place_extent_in_tsv` |
| AC6 | `tests/score.rs::ac6_deterministic_at_its_sample_with_its_pan_curve` |
| AC7 | `tests/score.rs::ac7_the_bed_ducks_under_lines_and_merge_closes_short_gaps` |
| AC8 | `tests/score.rs::ac8_the_bed_level_by_peak_or_rms` |
| AC9 | `tests/score.rs::ac9_a_post_folder_to_audio_wav_or_nothing` |
| AC10 | §4 (outside CI) |

## 4. Byte identity with the motor's engine (outside CI)

`sonido <post> --salida … --pistas …` in the rotorf-sico motor, before the
move (its own `sonido.rs`) and after (the thin `sonido.rs` over this
module), same `eventos.json`, `voz.json`, `sonido.toml` and `voz.wav`
(2026-10-16, linux x86_64). The stems (`voz`, `cama`, each bus dry and wet,
`golpes.tsv`) matched too.

| Post | `audio.wav` md5, before = after |
|---|---|
| imaginarios | `3f917f4beb80293762968cf049777a56` |
| pitagoras | `f65fc0972819a12857284b06aa1df4be` |
| tau-breve | `55b8bd8be19836007fa7d092f6150bc7` |
| que-es-ga | `e20a4bdad3bf8a8a0b7ee5031af50b89` |
| euler | `efc95ecd8017cad1003412f5b27c7d8d` |
| una-vuelta-breve | `6cee794283faf87fe0b9c0b9044ce2f8` |
| frenar-y-girar | `2207bfb3bf2ffdff33e9086a185f0bde` |
| deriva | `26077d42f55d75aa55076c6290c48625` |
