# RFC-002 — Simulated lessons: a bounded integrator in the browser

- **Status:** Proposal (Discussing)
- **Owner:** Gustavo Delgadillo (westerngazoo)
- **Scope:** physics-lab — add a second lesson *kind* that integrates in the browser,
  beside the existing closed-form kind, without abandoning the honesty discipline.
- **Depends on / aligns with:** the `Model` × `Source` split in
  `guion` RFC-0001 §3.2 (this RFC is the `physics-lab` realization of the `Stepped` source).
- **Staged from:** `fisicobuenfisico/docs/physics-lab/`; destined for `physics-lab/docs/RFC-002`.

> **Note on staging:** this file lives temporarily in the `fisicobuenfisico` workspace so
> it can be drafted alongside the `guion` proposal. It is written to move verbatim into
> `physics-lab/docs/` and be adopted through that repo's normal review.

---

## 1. The question this answers

RFC-001 established the founding rule: **physics-lab is closed-form only — no integrators
in the browser**, because "every on-screen state is a pure function of `(params, t)`, so
scrubbing cannot drift." That rule is correct and stays the default.

But it forecloses an entire class of *teaching*: systems with **no closed form** that are
most instructive precisely when a student can **drag the initial conditions and watch them
evolve live** — the double pendulum being the canonical example. RFC-001 routes those to
`motoreel` as offline films. A film is not draggable.

This RFC proposes a **second, clearly-labeled lesson kind** — a *simulated lesson* — that
runs a **bounded, deterministic integrator in the browser**, gated by the *same* honesty
discipline `motoreel` already uses (measured error bands), so it earns its place beside
the closed-form kind rather than eroding it.

## 2. What changes, precisely (and what doesn't)

- **Unchanged:** every existing lesson. Closed-form lessons remain pure `state_at(params)`
  functions; their guarantee ("scrubbing cannot drift, ever") is untouched.
- **Added:** a lesson may declare `"kind": "simulated"`. Such a lesson carries persistent
  state and advances it by stepping. It **must** display a measured error band.
- **Amended invariant:** "no integrators in the browser" becomes *"closed form when it
  exists; a **bounded, error-reported** integrator when it doesn't — and the two are
  distinct, labeled lesson kinds."* This restates, rather than abandons, the house rule
  ("closed form when it exists; honest integration with measured bounds when it doesn't").

## 3. The design: `Model` × `Source`, with `Stepped` as the lesson kind

This RFC adopts the same split as `guion` RFC-0001 §3.2 so the classroom and the film
studio share one vocabulary:

- **`Model`** — *what* the system is: a `State` type + a `draw(state) → prims` rule + a
  conserved `energy(state)` used to measure integration error.
- **`Source`** — *how* state at time `t` is obtained:
  - **`InPlace`** — closed-form recompute. **This is today's lesson.** (`state_at`.)
  - **`Stepped`** — integrate from the initial condition. **This is the new lesson kind.**
  - **`Played`** — sample a precomputed rollout (a baked `Stepped`, or a `motoreel` film
    imported as an asset). Free, drift-free scrubbing of already-computed motion.

A closed-form lesson is `Model + ClosedForm`, driven `InPlace`. A double pendulum is
`Model + Integrable`, driven `Stepped` live — or `Played` once baked.

```rust
// lessons-common — new, beside the existing closed-form `lesson!` macro.
pub trait SimLesson {
    type State: Copy;
    fn init(params: &[f64]) -> Self::State;                 // initial conditions
    fn step(state: &mut Self::State, params: &[f64], dt: f64); // the integrator
    fn draw(state: &Self::State, params: &[f64], out: &mut Prims, read: &mut Readouts);
    fn energy(state: &Self::State) -> f64;                  // the invariant we monitor
}
// sim_lesson!(DoublePendulum);  // wires the trait to the stepped ABI (§4)
```

## 4. ABI extension (additive; the closed-form ABI is untouched)

Today's exports (`params_ptr`/`state_at`/`prims_ptr`/`readouts_ptr`) stay exactly as they
are for closed-form lessons. Simulated lessons add a stepped surface:

```rust
#[no_mangle] pub extern "C" fn init(n_params: usize);   // set ICs from params, reset state
#[no_mangle] pub extern "C" fn step(dt: f64);            // advance the integrator by dt
#[no_mangle] pub extern "C" fn render() -> usize;        // current state → prims (returns len)
#[no_mangle] pub extern "C" fn state_ptr() -> *mut f64;  // phase vector: snapshot / restore
#[no_mangle] pub extern "C" fn state_len() -> usize;
// prims_ptr / readouts_ptr reused unchanged
```

State lives in a `static` in linear memory (like `LESSON_PARAMS` today) — no allocator,
instantiate with `{}`, same as now. `state_ptr` enables the scrubbing strategies in §6.

## 5. The integrator: fixed timestep, decoupled from the frame rate

The one thing that must not be gotten wrong. Browser frames (`requestAnimationFrame`)
arrive at variable intervals; stepping by the wall-clock frame `dt` makes results
dt-dependent, non-reproducible, and prone to blow-up. The runtime uses a **fixed-timestep
accumulator**:

```js
// runtime.js — the simulated-lesson loop (selected by kind === "simulated")
let acc = 0, prev = performance.now();
const DT = 1 / 240;                                  // fixed physics step, display-independent
function frame(now) {
  acc += Math.min((now - prev) / 1000, 0.1);         // clamp → no spiral-of-death on tab-away
  prev = now;
  while (acc >= DT) { wasm.step(DT); acc -= DT; }     // integrate in fixed chunks
  paint(readPrims(wasm.render()));
  requestAnimationFrame(frame);
}
```

Fixed `DT` buys **stability** and **determinism** at once: identical ICs + identical step
count ⇒ identical trajectory. Integrator choice: RK4 with small `DT` + an energy monitor
is the pragmatic default; a symplectic/Verlet-family integrator is preferred where
long-run bounded energy drift matters.

## 6. Scrubbing: the real tradeoff, and how we keep faith with RFC-001

Closed-form's headline property — scrub anywhere, never drift — comes from jumping to `t`
by formula. An integrator can't jump; reaching `t` means integrating from 0. Determinism
is preserved (**step index is the coordinate**: state at step *N* from given ICs is always
identical), but a free time-slider is no longer O(1). We pick per lesson:

| Strategy | UX | Cost | When |
|---|---|---|---|
| **Play / pause / reset only** | live sim, nudge params | simplest | "feel the chaos" sandbox |
| **Deterministic replay + keyframe cache** | scrub works; snapshot `state_ptr` every K steps, re-integrate from nearest | O(K)/scrub, some memory | scrubbable sim page |
| **Bake to `Played`** | free O(1) scrub of a stored trajectory | re-integrate on param change | polished explainer (== `motoreel`) |

The last row is the reconciliation: **integrate once from 0, bake to `Played`, then scrub
a fixed buffer** — drift-free again, exactly RFC-001's spirit, and identical to
`motoreel::record()`. Live interactivity and drift-free scrubbing are simply two `Source`
modes over one `Model`.

## 7. The honesty surface (mandatory, non-negotiable)

A simulated lesson **must** show its numerical error, or it doesn't ship. Driven by
`energy(state)` vs the initial energy, the page displays a readout — e.g.
`deriva de energía: 0.03%` — and a visible warning past a per-lesson threshold. This is
the browser twin of `motoreel`'s measured error bands (RFC-001: "energy drift −0.52% at
dt = 1/960 … measured, then binding"). Closed-form (`InPlace`) lessons owe no such band
because they never approximate.

## 8. Verification parity (why this can live in physics-lab at all)

The reason the classroom can host an integrator without lowering its bar: **`cargo test`
runs the same Rust the browser runs** (RFC-001 §4.0). Simulated lessons keep that,
trading algebraic identities for conserved-quantity bounds and known limits:

```rust
#[test] fn energy_drift_bounded() {           // the honest error band, as a gate
    let mut s = DoublePendulum::init(&params);
    let e0 = DoublePendulum::energy(&s);
    for _ in 0..steps { DoublePendulum::step(&mut s, &params, DT); }
    assert!((DoublePendulum::energy(&s) - e0).abs() / e0.abs() < BOUND);
}
#[test] fn small_angle_matches_normal_modes() { // the limit that DOES have a closed form
    // equal pendulums: ω² = (2 ± √2)·g/l  — the simulation must reproduce it
}
#[test] fn rk4_matches_symplectic_short_term() { /* two integrators agree pre-chaos */ }
```

Plus the independent Python cross-check in `checks/` (L2), same as every lesson.

## 9. Manifest & runtime changes

- `lesson.json` gains `"kind": "closed" | "simulated"` (default `"closed"`).
- Params split into **initial-condition params** (changing them calls `init()`/reset — e.g.
  starting angles, masses) and **live params** (applied each `step` — e.g. gravity). The
  manifest tags each; the runtime wires controls accordingly.
- Controls: **play / pause / reset**; the accumulator loop of §5; the tab-away clamp.
- A required `errorReadout` slot bound to the energy band (§7).

## 10. Seams with the rest of the ecosystem

- **guion (RFC-0001 §3.2):** `guion-motion`'s `Stepped` source *is* this ABI; its `Played`
  source is a bake of it. One `Model` authored here serves the interactive lab, the reel,
  and the tests.
- **motoreel:** a `Played` lesson may embed a `motoreel` film as its precomputed
  trajectory (the existing one-way `motoreel → physics-lab` asset seam), now unified under
  the same `Source` vocabulary.

## 11. Acceptance criteria

- [ ] A `"kind": "closed"` lesson behaves bit-identically to today (no regressions).
- [ ] A `"kind": "simulated"` lesson runs a fixed-`DT` accumulator loop; the trajectory is
      independent of display refresh and reproducible for identical ICs + step count.
- [ ] The energy-band readout renders and updates every frame; exceeding the threshold
      shows the warning.
- [ ] IC params trigger `init()`/reset; live params apply per step.
- [ ] `cargo test` covers energy-drift bound, small-angle normal modes, and an
      integrator cross-check on the same code the browser runs.
- [ ] A `Played` lesson scrubs a baked trajectory in O(1) with no drift.
- [ ] The double pendulum ships as the reference simulated lesson (draggable bobs + band).

## 12. Open questions

- Q1. Default integrator: RK4 + monitor vs a symplectic method for long-run demos?
  Lean RK4 + monitor for v1; symplectic as an opt-in per lesson.
- Q2. Keyframe-cache stride K for the replay-scrub strategy (§6) — fixed, or adaptive to
  trajectory length?
- Q3. Should `Played`-from-film and `Played`-from-bake be distinguished in the manifest
  (provenance), or unified? Lean: record provenance (commit, integrator, dt, band).

## 13. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-08-29 | Add a *second* lesson kind rather than relax the closed-form rule globally | Keeps the drift-free guarantee for the lessons that have it; contains integration behind a labeled kind |
| 2026-08-29 | Fixed-timestep accumulator, `DT` decoupled from frame rate | Stability + determinism; the only correct way to integrate under `requestAnimationFrame` |
| 2026-08-29 | Energy-band readout is mandatory for simulated lessons | The browser twin of motoreel's measured error bands; honesty is the price of an integrator |
| 2026-08-29 | Adopt the `Model` × `Source` split from guion RFC-0001 §3.2 | One vocabulary across classroom, film studio, and tests; `Stepped` = this lesson kind |

## Changelog

- 2026-08-29 — created (draft, Discussing).
