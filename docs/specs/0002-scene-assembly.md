# SPEC-0002 — Screenplay → `motoreel::Scene` assembly (keyframe motion)

- **Status:** Draft
- **Realizes:** R-0002
- **Author:** físico buen físico
- **Created:** 2026-08-28
- **Depends on:** SPEC-0001
- **Module(s):** `guion-assemble` (+ read-only use of `guion-core`, `guion-brand` default theme)

## 1. Motivation

Realizes R-0002: translate a validated `Screenplay` into a `motoreel::Scene`. This is the
single seam that couples guion to `motoreel` (RFC-0001 §3.4, §7.1).

## 2. Design

### Module layout (`guion-assemble`)

```
guion-assemble/src/
├── lib.rs        # assemble(&Screenplay, &Theme) -> Result<motoreel::Scene, AssembleError>
├── shape.rs      # ObjectSpec.shape -> motoreel::Shape (resolving bound tokens)
├── style.rs      # StyleSpec + Theme -> motoreel::Style (colors, width)
├── camera.rs     # CameraSpec -> motoreel::Camera (pinhole/orthographic)
├── motion.rs     # Motion timeline -> Source (M1: authored Played) -> motoreel::Track
├── label.rs      # LabelSpec -> motoreel::Label + Anchor
└── error.rs      # AssembleError (typed, names object/field)
```

### Entry point

```rust
// lib.rs
pub fn assemble(sp: &Screenplay, theme: &Theme) -> Result<Scene, AssembleError> {
    let mut scene = Scene::new(motion::duration(&sp.motion));   // AC1
    scene.camera = camera::build(&sp.camera)?;                  // AC4
    let ids = build_object_ids(&sp.object);
    for obj in &sp.object {
        let shape = shape::build(&obj.shape, &sp.model)?;       // AC2
        let style = style::build(&obj.style, theme)?;
        let track = motion::track_for(obj, &sp.motion)?;        // AC3
        scene.add(Object::new(shape).with_style(style).with_track(track));
    }
    for lbl in &sp.label { scene.add_label(label::build(lbl, &ids)?); }  // AC5
    Ok(scene)
}
```

### Motion (M1: authored `Played` source)

The `Model` × `Source` split (RFC-0001 §3.2) is the target abstraction; M1 implements
**only the `Source::Played` path with authored samples** (keyframe/spin). `InPlace`
(closed-form wasm) and `Stepped` (live integration) are M4. For M1, a keyframe object's
"model" is trivial — its state *is* a pose — so `motion.rs` builds an authored `Played`
source and lowers it to a `motoreel::Track` (which is precisely a played sequence of
poses). This keeps M1 concrete while making later modes a pure addition, not a rewrite.

The `[motion]` timeline (`drive`, `from`, `to`, `ease`, `hold`) lowers as:

- `duration` = timeline duration; frame count later is `ceil(duration*fps)`.
- `ease` name → `motoreel::Ease` (`Linear`/`SmoothStep`/`SmootherStep`/custom).
- `hold = { start, end }` inserts constant-value key spans at each end so the value
  dwells before/after the eased ramp — the declarative form of the reels' hand-built
  `seq = [0]*n + ramp + [1]*m + ...` (R-0002 AC3).

```rust
// motion.rs — M1 lowers an authored Played<Pose> source into a motoreel::Track.
// Same shape as Source::Played { samples } (RFC-0001 §3.2); the hold dwell
// reproduces the current reel feel.
pub fn track_for(obj: &ObjectSpec, m: &Motion) -> Result<Track, AssembleError> {
    let (t0, t1) = (0.0, m.duration());
    let (hs, he) = m.hold_fractions();          // validated ∈[0,1] in R-0001
    let ramp_start = t0 + hs * (t1 - t0);
    let ramp_end   = t1 - he * (t1 - t0);
    Track::keys(vec![                           // == Played samples over poses
        (t0,         pose_at(obj, m.from)?),
        (ramp_start, pose_at(obj, m.from)?),
        (ramp_end,   pose_at(obj, m.to)?),
        (t1,         pose_at(obj, m.to)?),
    ]).map_err(AssembleError::from)            // eased on the middle span
}
```

In M4 the same call site instead asks `guion-motion` for `source.at(m, p, t)` per frame:
an `InPlace` model recomputes closed-form, a `Stepped` model integrates from 0, a
`Played` model samples a baked buffer — and each object's state is drawn identically.

For the M1 golden, physics numbers (`@biceps_force_N`, `@elbow`, …) are supplied as
static per-key inputs (R-0002 §4); value-driven motion arrives with M4.

### Camera / shape / style / label

- `camera::build` → `Camera::orthographic(pose)` or `Camera::pinhole(pose, focal)`.
- `shape::build` resolves bound tokens to concrete model-space points, then constructs
  the matching `motoreel::Shape`.
- `style::build` resolves theme palette names to `Rgb`; M1 uses a minimal built-in
  default `Theme` (full theming is M3/`guion-brand`).
- `label::build` maps to `Anchor::Point` / `Anchor::Pose { object, at }` /
  `Anchor::Screen`, validating pose anchors against the object id set (AC5).

### Errors

`AssembleError` is typed and names the object id and field for every unmappable case;
assembly never panics on R-0001-valid input (AC7).

## 3. Code outline

The `assemble` function above plus the per-concern builders. `guion-assemble` is the
only crate that `use`s `motoreel::*` (RFC-0001 §3.4). Rendering is delegated to
`motoreel` unchanged: the caller (R-0003) picks a sink and calls `scene.render(fps, sink)`.

## 4. Non-goals

- `InPlace` (closed-form wasm) and `Stepped` (live integration) sources (R-0004+, M4);
  M1 is the authored `Played` path only.
- Encoding/audio/formats (M2).
- Full theme system (M3) — a default theme only.

## 5. Open questions

- Single vs multi-drive timeline in M1 (R-0002 Q1). Leaning single-drive.
- Value-driven style (`heat:@value`) in M1 vs M4 (R-0002 Q2). Leaning static in M1.

## 6. Acceptance criteria

- [ ] Scene duration = motion duration (R-0002 AC1).
- [ ] One `motoreel::Object` per screenplay object, mapped shape+style (AC2).
- [ ] Track hits `from`@t0 and `to`@t_end; ease applied; hold spans present (AC3).
- [ ] Camera maps to correct projection + pose (AC4).
- [ ] Labels map to correct `Anchor`; pose anchors resolve (AC5).
- [ ] Scene renders `ceil(duration*fps)` frames via a `motoreel` sink (AC6).
- [ ] Unmappable input → typed `AssembleError`, no panic (AC7).
- [ ] reel-09 golden assembles + renders; frame count + sampled prims match golden (AC8).

## 7. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-08-28 | `guion-assemble` is the sole `motoreel` consumer | One seam owner (RFC-0001 §7.1) |
| 2026-08-28 | `hold` → constant end key-spans | Reproduces reel dwell without hand-built `seq[]` |
| 2026-08-28 | M1 uses a built-in default theme | Unblocks the slice; real theming is M3 |

## Changelog

- 2026-08-28 — created (draft).
