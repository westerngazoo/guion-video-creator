# SPEC-0008 — Generic, deterministic synthesis

- **Status:** In review
- **Realizes:** R-0008
- **Created:** 2026-10-14
- **Depends on:** SPEC-0007 (`Pcm16`, `read_wav_pcm16`, `Narration`)
- **Module(s):** `guion-audio::synth`

## 1. Motivation

Realizes R-0008. It was proven by porting the `rotorf-sico` reel
*imaginarios* from its numpy `audio.py` to data (`sonido.toml`), read by the
studio's Rust `sonido` binary (`mathematicus/motor/src/bin/sonido.rs`). The
studio's instruments (`liber_sonido.rs`) are built from this module.

## 2. Design

### Module layout (`guion-audio/src/synth/`)

```
synth/
├── mod.rs     # re-exports; sample_at (round ties-to-even)
├── fft.rs     # Complex, rfft / irfft (radix-2, real via half-size complex), fft_filter, convolve
├── osc.rs     # add_sine / sine (resynchronised phasor), Partial, partials, phase_of
├── env.rs     # exp_decay, linear_attack, rise, Adsr, breakpoints
├── noise.rs   # Noise (xoshiro256**, Box–Muller), seed_for
├── filter.rs  # lowpass / highpass / bandpass (zero-phase) and their gains; OnePole
├── room.rs    # Room (t60, length, damping, seed) → level-stable impulse_response
├── bus.rs     # pan_gains, Pan, Stereo, Bus (dry + send), render
├── duck.rs    # DuckShape, duck_windows, narration_windows, follow, Follower, duck_follower
└── out.rs     # fade, normalize_peak, to_i16, to_pcm16, from_pcm16
```

`pcm.rs` gains `write_wav(path, &Pcm16)` for any channel count;
`write_wav_pcm16` (mono) keeps its bytes. `AudioError` gains
`InvalidSynth { what, message }`.

### Use

```rust
use guion_audio::synth::*;
let sr = 48_000.0;
let n = (sr * 10.0) as usize;
let mut bus = Bus::new(n);
let mut g = Noise::new(seed_for(post_seed, 7));            // this hit's own noise
let hit: Vec<f64> = g.white(14_400).iter().zip(exp_decay(14_400, sr, 0.05)).map(|(a, b)| a * b).collect();
bus.place(&hit, sample_at(2.5, sr), 0.4, Pan::Fixed(-0.2), 0.5);
let l = Room::new(2.8, 3.4, 4200.0, seed_for(post_seed, 100))?.impulse_response(sr);
let r = Room::new(2.8, 3.4, 4200.0, seed_for(post_seed, 101))?.impulse_response(sr);
let mut mix = bus.render((&l, &r), 0.8);
mix.apply_gain(&duck_windows(n, sr, &narration_windows(&voice), &DuckShape { depth_db: -10.0, pre: 0.08, post: 0.12, ramp: 0.08 })?);
normalize_peak(&mut mix, 0.89);
write_wav(Path::new("audio.wav"), &to_pcm16(&mix, 48_000))?;
```

### The room (AC6)

`m = round(length · rate)` samples of Gaussian noise times `e^{−3 ln10 · i / (t60 · rate)}`.
Then, `FLATTEN` = 12 times: the spectrum on `2 · next_pow2(m)` points gets the
magnitude of a Butterworth low-pass (order 2, at `damping`) with its own
phases (the doubled grid makes a tone between bins see the same gain); back
in time, truncated to `m`, each sample is scaled by `decay / local RMS`
(10 ms centred window). Finally unit energy. Building one 3.4 s response costs
24 FFTs of 2¹⁹ points.

### Determinism (AC1)

One thread; every loop in index order; prefix sums in a fixed order; FFT
twiddles computed directly from their index. No `HashMap` iteration. Noise
streams are independent by construction (`seed_for` mixes seed and index
through SplitMix64).

## 3. Acceptance-criteria checklist

| AC | Test |
|----|------|
| AC1 | `tests/synth.rs::ac1_deterministic_bytes_and_independent_streams` |
| AC2 | `tests/synth.rs::ac2_oscillators` |
| AC3 | `tests/synth.rs::ac3_envelopes` |
| AC4 | `tests/synth.rs::ac4_seeded_noise` |
| AC5 | `tests/synth.rs::ac5_filters_match_closed_forms`; `synth::fft::tests::*` |
| AC6 | `tests/synth.rs::ac6_room` (fails with `FLATTEN = 0`: 6.21 dB RMS) |
| AC7 | `tests/synth.rs::ac7_constant_power_pan` |
| AC8 | `tests/synth.rs::ac8_bus` |
| AC9 | `tests/synth.rs::ac9_ducking` |
| AC10 | `tests/synth.rs::ac10_stereo_wav`; `tests/synth.rs::fades_and_peak` |
| AC11 | `cargo test -p guion-audio --no-default-features` (synth tests run; `Cargo.toml` unchanged) |
| AC12 | `tests/synth.rs::ac12_failures_name_the_parameter` |

## 4. Equivalence with the numpy tool it replaces (outside CI)

The two are not hashed against each other (another FFT, another generator,
another room). Measured on *imaginarios* (89 s; 1 drone, 1 follower, 49 hits
on 34 instants; voice from `voz.wav`), numpy `audio.py` against the Rust
`sonido` on the same `eventos.json`, `voz.json` and `voz.wav`:

| Measure | numpy | Rust |
|---|---|---|
| integrated loudness of `audio.wav` | −14.6 LUFS | −14.7 LUFS |
| true peak | −1.0 dBTP | −1.0 dBTP |
| voice over the bed, in the lines (both channels) | 18.9 dB | 19.0 dB |
| bed between lines minus in them | 7.4 dB | 7.7 dB |
| attacks within 1 ms of `eventos.json` (LPC-residual detector; swells removed) | 33/34 | 33/34 |
| spectral centroid per family (12 families) | — | within 2.8 % |

The one attack not detected in either is a 0.2 s sin² fade-in with no
attack. Through R-0005 (`mux_at_loudness`), the reel comes out at
−14.1 LUFS and −1.6 dBTP (the delivered numpy one: −14.1 LUFS, −1.8 dBTP).
Two Rust runs give the same md5.
