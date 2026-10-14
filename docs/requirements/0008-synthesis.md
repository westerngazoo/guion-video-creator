# R-0008 — Generic, deterministic synthesis for a reel's sound

- **Status:** In review (implemented locally, awaiting the owner's sign-off)
- **Milestone:** M2
- **Owner:** físico buen físico (see project-specifics.md)
- **Created:** 2026-10-14
- **Depends on:** R-0007 (`Pcm16`, the WAV writer, `Narration`)
- **Realized by:** SPEC-0008
- **QA:** `qa` agent run scoped to this requirement

## 1. Statement

`guion-audio` must provide the building blocks of a reel's sound, with no
brand in them: oscillators, envelopes, seeded noise, filters, a room, pan,
mixing buses, ducking under a voice, and the way out to a 48 kHz stereo
16-bit WAV. The same input must give the same bytes, with no dependence on
threads or on the order in which independent sounds are rendered.

## 2. Rationale

The `rotorf-sico` studio designs every reel's sound in a Python file
(`audio.py`, numpy) of 260–340 lines. Ten of those files carry the same 12
functions copied by hand (filters, a noise room, a drone, a drum, a metal
bar, a bell…). The owner decided on 2026-10-14 to move the whole reel chain
to Rust. The studio keeps its instruments (they are the brand) and needs the
generic layer underneath them. Any creator who makes video with guion needs
the same layer: `bed.rs` (the fbf bed) is a smaller copy of it.

Porting the first reel (*imaginarios*) showed two problems that belong here,
not in a studio:

- **A plain noise room is a lottery for sustained tones.** A reverb built from
  seeded decaying noise has a random gain at every single frequency, and its
  power is exponentially distributed: across seeds a steady tone came out
  between −21 dB and +8 dB of the target, with 5.4 dB standard deviation. On
  the reel, the 55 Hz drone fundamental gained −0.6 dB in one channel and
  +3.9 dB in the other from the two Python seeds. Broadband noise came out at
  0.0 dB either way. Changing a seed must not move the drone's reverb by
  4 dB.
- **Noise must not depend on render order.** One numpy generator fed every
  call in program order, so inserting one hit changed the noise of every hit
  after it.

## 3. Acceptance criteria

- **AC1.** Determinism: the same scene (notes, noise, a room, a duck, a WAV)
  rendered twice gives identical bytes. Each sound's noise comes from its own
  stream named by `(seed, index)` (`seed_for`): adding a sound leaves every
  other sound's noise unchanged, and a different seed gives different bytes.
- **AC2.** Oscillators match their closed forms: a sine of 200 000 samples
  within 1e-9 of `a·sin(2πft + φ)` (the phasor is resynchronised, so its
  error does not grow); a sum of partials with their own decays and phases
  within 1e-9; the phase of a frequency track is its running sum.
- **AC3.** Envelopes: an exponential decay is `e⁻¹` at τ (an instant decay
  gives an impulse, never NaN); ADSR has its levels at the segment edges;
  breakpoints are linear inside and flat outside.
- **AC4.** Noise: Gaussian with mean within 0.01 and variance within 0.02 of
  N(0, 1) over 2·10⁵ samples; uniform in [0, 1); the same seed gives the same
  stream.
- **AC5.** Filters match their closed forms within 0.5 % on steady sines:
  zero-phase Butterworth low-pass and high-pass of order *n*, the resonant
  band-pass `1/√(1 + Q²(f/f₀ − f₀/f)²)`, and a one-pole within 1 % of
  `a / |1 − (1 − a)e^{−iω}|`. A zero-phase filter does not move a click in
  time. The FFT behind them matches the DFT and inverts within 1e-12.
- **AC6.** The room: its response has unit energy, falls 60 dB per `t60`
  (within 1.5 dB over half a `t60`), an impulse heard in the room is the
  response, and it is **level-stable**: across 43 tones from 30 Hz to
  3 kHz (on and off any FFT grid) and three seeds, the gain stays within
  1.5 dB RMS (and 6 dB at worst) of the target low-pass. A plain noise
  response fails this test (6.2 dB RMS).
- **AC7.** Pan has constant power: `L² + R² = 1`, centre is `cos(π/4)` on
  both sides, −1 is hard left, and values outside [−1, 1] are clamped.
- **AC8.** A bus places a sound at its sample (`round(t · rate)`, ties to
  even), cuts what falls outside, follows a pan track one sample per signal
  sample (the last one holds), sends to the room, and renders
  `dry + wet · (send ∗ response)`.
- **AC9.** Ducking: by line windows, the gain is `10^(depth/20)` inside each
  window widened by `pre` and `post`, 1 outside, halfway at the edge, with no
  sag at the ends of the track. By an envelope follower, the same shape
  appears where the voice's level crosses the threshold.
- **AC10.** WAV out: 48 kHz stereo 16-bit with the canonical 44-byte header
  (2 channels, byte rate `rate · 4`, block align 4); samples are `x · 32767`
  rounded ties-to-even and clamped; reading it back gives the same `Pcm16`.
- **AC11.** No brand and no screenplay: the module builds with
  `default-features = false` and adds no crates.io dependency (the FFT is
  plain `std` Rust).
- **AC12.** Failures are typed and located (CLAUDE.md §6): a room with a
  non-positive `t60`, `length` or `damping`, a duck with a non-finite depth
  or a negative ramp, a follower with a non-positive cutoff each return
  `AudioError::InvalidSynth` naming the parameter (`room.t60`, `duck.ramp`).
  A WAV whose samples do not split into its channels is refused before
  anything is written.

## 4. Constraints & non-goals

- `std` only. No threads in the synthesis: the bytes must not depend on the
  scheduler.
- Bit-for-bit equality holds on one platform. `sin`, `exp` and `ln` come from
  the platform's `libm`, so another libc can move the last bit. The md5 of a
  delivered sound belongs with its platform.
- Not here: instruments (a drum, a bell, a house drone), a studio's sound
  design format (rotorf-sico's `sonido.toml`), loudness (R-0005 owns
  −14 LUFS), live or streaming audio (the zero-phase filters see the whole
  signal).
- Output is **not** sample-identical to the numpy tools it replaces: the
  FFT, the noise generator and the room differ. Equivalence is measured,
  not hashed (SPEC-0008 §4).

## 5. Open questions

- Q1. Should `bed.rs` (the fbf 8-bit bed, screenplay path) move onto `synth`?
  Recommend yes, together with a deliberate re-take of that bed.
- Q2. Should the room offer a plain-noise mode for a studio that wants the
  old character? Recommend no until someone asks: the level lottery is the
  defect AC6 removes.
- Q3. Should a parallel render of independent buses be allowed (each bus on
  its own thread, summed in a fixed order)? It would keep the bytes and
  roughly halve a 90 s render (36 s on 4 cores today). Not needed yet.

## 6. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-10-14 | The room is level-stable: noise under the decay, then 12 rounds of alternating projections (spectrum magnitude to the target low-pass keeping phases, then the 10 ms level back to the decay) | A plain noise IR moved a sustained tone by up to 21 dB between seeds. Projections keep the decay exact (−10.7, −30.0, −60.1 dB at 0.5, 1.4, 2.8 s for `t60` = 2.8; measured on the prototype) and the gain within ~1 dB of the target. Ending each round on the decay keeps the tail exact, at the cost of a few rare dips (AC6) |
| 2026-10-14 | Spectral (FFT, zero-phase) filters, not IIR, for rendered sounds | They are the studio's sound (closed-form magnitudes) and do not shift attacks; a one-pole stays for control signals and the follower |
| 2026-10-14 | A long sine is a phasor resynchronised every 1024 samples | 144 drone partials × 4.3 M samples by `sin` would dominate the render; the phasor keeps the 1e-9 bound (AC2) |
| 2026-10-14 | Noise: xoshiro256** seeded through SplitMix64, Box–Muller | Small, std-only, well-mixed; `seed_for(seed, index)` names a stream per sound (AC1) |
| 2026-10-14 | 16-bit quantisation rounds ties-to-even | numpy's `astype` truncates toward zero, which biases every sample by half a step |
| 2026-10-14 | `Bus::render` takes built responses, not a `Room` | A reel's buses share one room; building the response once per channel saves rounds of projections |

## Changelog

- 2026-10-14 — created; realized by SPEC-0008 in the same change.
