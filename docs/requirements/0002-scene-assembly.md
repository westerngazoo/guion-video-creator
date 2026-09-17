# R-0002 — Screenplay → `motoreel::Scene` assembly (keyframe motion)

- **Status:** Discussing
- **Milestone:** M1
- **Owner:** físico buen físico (see project-specifics.md)
- **Created:** 2026-08-28
- **Depends on:** R-0001
- **Realized by:** SPEC-0002
- **QA:** `qa` agent run scoped to this requirement

## 1. Statement

guion must translate a validated `Screenplay` (R-0001) into a `motoreel::Scene` ready to
render: mapping each screenplay object to a `motoreel::Object` with its `Shape`, `Style`,
and a `Track`; the screenplay camera to a `motoreel::Camera`; and each screenplay label
to a `motoreel::Label` with the correct `Anchor`. For M1, object motion comes from the
`[motion]` timeline resolved as an **authored `Played` source** only (the keyframe/spin
path of the `Model` × `Source` split, RFC-0001 §3.2); `InPlace` and `Stepped` sources are
M4. The resulting `Scene` must render to frames via
an existing `motoreel` sink with no further guion involvement.

## 2. Rationale

This is the seam where the creator-facing model meets the render engine (RFC-0001 §3.4).
Getting the mapping right — and confining all coupling to `motoreel` in this one layer —
is what lets guion evolve the schema and lets `motoreel` evolve its API without the two
churning each other (RFC-0001 §7.1). It is the vertical-slice payoff: a file becomes a
scene becomes frames.

## 3. Acceptance criteria

- **AC1.** A validated keyframe-only `Screenplay` assembles into a `motoreel::Scene`
  whose `duration` equals the screenplay's motion duration.
- **AC2.** Each screenplay object becomes exactly one `motoreel::Object` with the mapped
  `Shape` (point/segment/polyline/edges/arrow) and `Style` (stroke color, width) from the
  active theme's resolved values.
- **AC3.** The screenplay `[motion]` block produces a `motoreel::Track` such that: the
  driven value equals `from` at t=0 and `to` at t=duration; the named `ease` is applied;
  and `hold` dwell fractions reserve constant-value spans at the ends (reproducing the
  hand-built `seq[]` dwell-in/dwell-out of the current reels).
- **AC4.** The screenplay camera maps to `Camera::pinhole`/`Camera::orthographic` with
  the specified pose; projection matches the requested kind.
- **AC5.** Each screenplay label becomes a `motoreel::Label` with the correct `Anchor`
  variant (`Point`, `Pose { object, at }`, or `Screen`); pose anchors resolve to a
  declared object id.
- **AC6.** The assembled `Scene` renders a frame count of `ceil(duration * fps)` with no
  error, over an in-memory or temp-dir sink (uses `motoreel`'s own render path).
- **AC7.** Assembly is total over any R-0001-valid screenplay: any case guion cannot map
  is a typed `AssembleError` naming the object/field, never a panic.
- **AC8.** The reel-09 lever golden screenplay (with its physics values supplied as
  static keyframe inputs for M1) assembles and renders its full frame sequence; frame
  count and a sampled mid-frame primitive set match a committed golden.

## 4. Constraints & non-goals

- **Authored `Played` source only.** `InPlace` (closed-form `.wasm`) and `Stepped`
  (live integration) / baked `Played` (`garust`/`record`) sources are R-0004+ (M4). See
  RFC-0001 §3.2 for the `Model` × `Source` split. Where the golden reel needs physics
  numbers, M1
  supplies them as literal keyframe inputs.
- **No encoding.** Output is frames via a `motoreel` sink; ffmpeg/audio/vertical delivery
  is M2 (R-0005+).
- **`motoreel` coupling lives only here.** No other guion crate imports `motoreel`
  types (RFC-0001 §3.4, §7.1).
- Theme resolution (palette → concrete colors) is consumed here but owned by
  `guion-brand` (M3); for M1 a minimal built-in default theme suffices.

## 5. Open questions

- Q1. Does `[motion]` in M1 drive a single scalar (`drive = "@x"`) mapped to one object's
  track, or multiple simultaneous drives? Recommend single-drive for M1; multi-track
  timelines are a follow-up requirement.
- Q2. Style color tokens like `heat:@value` require a value at assembly time; for M1
  keyframe motion the value is known per key. Confirm whether per-key style is in scope
  or deferred. Recommend static style in M1, value-driven style with M4 motion.

## 6. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-08-28 | All `motoreel` coupling is confined to `guion-assemble` | Single seam owner; enables independent evolution (RFC-0001 §7.1) |
| 2026-08-28 | M1 renders to frames only; encoding is M2 | Keeps the vertical slice provable without the delivery layer |
| 2026-08-28 | `hold` fractions reproduce the reels' dwell-in/out via constant end spans | Preserves the current motion feel while removing hand-built `seq[]` |

## Changelog

- 2026-08-28 — created (draft, Discussing).
