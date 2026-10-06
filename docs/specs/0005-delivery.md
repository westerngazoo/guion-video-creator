# SPEC-0005 — Delivery: tagged colour, streaming, ordered parallel, loudness

- **Status:** In review
- **Realizes:** R-0005
- **Created:** 2026-10-06
- **Depends on:** SPEC-0003
- **Module(s):** `guion-encode`

## 1. Motivation

Realizes R-0005. The pieces were proven first in the `rotorf-sico` studio
(`mathematicus/motor/src/estudio.rs`, `tools/mezcla.py`); this lifts them
into the framework so the studio, the CLI and the apps share one path.

## 2. Design

### Module layout (`guion-encode`)

```
guion-encode/src/
├── lib.rs        # encode_ppm_dir (now on the shared flags), presets, re-exports
├── ffmpeg.rs     # ffmpeg discovery (PATH or GUION_FFMPEG), VIDEO flags, fps
├── stream.rs     # Encoder: RGB frames from memory → .mp4, no partial output
├── ordered.rs    # encode_ordered: N render threads, one ordered writer
├── loudness.rs   # measure, mux_at_loudness, Loudness, LoudnessTarget::REELS
└── error.rs      # + Render { frame, message }, + Loudness { reached, target, rounds }
```

### The shared video flags (AC1–AC3)

```
-c:v libx264 -pix_fmt yuv420p -crf 18 -movflags +faststart
-vf scale=out_color_matrix=bt709:out_range=tv,format=yuv420p
-colorspace bt709 -color_primaries bt709 -color_trc bt709
```

The conversion happens inside the filter graph (the matrix the encoder
receives is BT.709), and the tags say so. `encode_ppm_dir`, `Encoder` and
`encode_ordered` all use `ffmpeg::VIDEO`.

### Streaming (AC4, AC7)

```rust
let mut enc = Encoder::start(out, VideoSpec { width, height, fps })?;
enc.push(&rgb)?;            // exactly width × height × 3 bytes
let n = enc.finish()?;      // waits for ffmpeg, renames the temp file
```

ffmpeg reads `-f rawvideo -pix_fmt rgb24` on stdin and writes
`.<name>.parcial.mp4` beside `out`. `Drop` without `finish` kills ffmpeg and
removes the temp file.

### Ordered parallel (AC5)

`encode_ordered(out, spec, frames, threads, render)`. Frame `i` is rendered
by thread `i mod n` into a `sync_channel(2)`; the caller's thread receives
in index order and pushes. `render: Fn(usize) -> Result<Vec<u8>, E> + Sync`.
A render error stops the writer, drops the receivers (workers stop at their
next send) and returns `Render { frame, .. }`.

`encode_ordered_with(out, spec, frames, threads, init, render)` (AC8): each
worker calls `init(h)` once, inside the worker, and lends the state to
`render(&mut state, i)`. `encode_ordered` is this with `()` state.

### Loudness (AC6)

`mux_at_loudness(video, audio, out, LoudnessTarget::REELS)`:

1. two-pass `loudnorm` (I = target, TP = ceiling − 0.5, LRA = 11) to 48 kHz WAV;
2. exact gain to the target, then `alimiter` at 192 kHz (sample peak ≈ true peak);
3. mux with the video copied, encode AAC 192 kbps, measure with `ebur128`.
   Out of tolerance: correct the gain. Over the ceiling: lower the limiter.
   At most 8 rounds; the accepted round is copied to `out` via a temp file.

## 3. Acceptance criteria

- [x] BT.709 tags in TV range (R-0005 AC1)
- [x] flat colours round-trip ±3 (AC2)
- [x] `moov` before `mdat`, with and without audio (AC3)
- [x] memory = disk, decoded (AC4)
- [x] 1 thread = N threads, bytes (AC5)
- [x] −14 ± 0.5 LUFS, ≤ −1 dBTP, deterministic (AC6)
- [x] typed, located errors; no leftovers (AC7)
- [x] per-worker state, built once per worker (AC8)

## 4. Traceability

All in `crates/guion-encode/tests/entrega.rs`; the parsers have unit tests
in `loudness.rs`.

| AC | Criterion | Proven by | Code |
|----|-----------|-----------|------|
| AC1 | BT.709 tags, TV range | `ac1_el_video_va_etiquetado_bt709` | `ffmpeg::VIDEO` |
| AC2 | colour round-trip ±3 | `ac2_el_color_llega_como_se_mando` | `scale=out_color_matrix=bt709` |
| AC3 | `moov` before `mdat` | `ac3_el_indice_va_antes_que_los_datos` | `+faststart` in `ffmpeg::VIDEO` |
| AC4 | memory = disk | `ac4_en_memoria_igual_que_en_disco` | `stream::Encoder` |
| AC5 | 1 = N threads, all frames | `ac5_uno_o_cuatro_hilos_dan_los_mismos_bytes` | `ordered::encode_ordered` |
| AC6 | loudness target, deterministic | `ac6_el_audio_queda_a_menos_14_lufs`, `loudness::tests::*` | `loudness::mux_at_loudness`, `measure` |
| AC7 | typed errors, no leftovers | `ac7_errores_con_nombre_y_sin_archivo` | `VideoSpec::check`, `Encoder::push`, `Drop for Encoder`, `EncodeError::Render` |
| AC8 | per-worker state | `ac8_cada_hilo_con_su_lienzo` | `ordered::encode_ordered_with` |

## 5. Non-goals

- The CLI flag to choose the streaming path (R-0005 Q1).
- A screenplay loudness field (R-0005 Q2).

## Changelog

- 2026-10-06 — created with the implementation.
- 2026-10-06 — AC8, `encode_ordered_with`.
