# ADR-0002 — `[motion]` is a flat timeline for M1, not a mode enum

- **Status:** Accepted
- **Date:** 2026-08-28 (confirmed during the R-0001 implementation pass)
- **Deciders:** owner + implementer
- **Realizes:** [R-0001 AC7](../requirements/0001-screenplay-schema.md#3-acceptance-criteria);
  [RFC-0001 §4](../RFC-0001-guion-framework.md#4-the-screenplay-format-creator-surface)

## Context

[SPEC-0001](../specs/0001-screenplay-schema.md) sketched motion as a rich,
internally-tagged `MotionMode` enum with five variants —
`Keyframes`, `Spin`, `InPlace`, `Stepped`, `Played` — each mapping to a `guion`
`Source` at resolve time. The spec explicitly labelled that sketch
"representative, not final."

Meanwhile the requirement's acceptance criterion (AC7) and the RFC's illustrative
screenplay both describe M1's motion as a single flat block:

```toml
[motion]
drive = "@lever.phi"
from  = 0.0
to    = 2.53
ease  = "smootherstep"
hold  = { start = 1.0, end = 0.87 }
```

The five-mode enum is only *meaningful* once a `Source` can be resolved to state
over time — and that resolution (the `InPlace`/`Stepped`/baked `Played` machinery)
is milestone M4, not M1. Modelling all five modes now would let a creator author
a `stepped` motion that M1 silently cannot execute.

## Decision

For M1, `Motion` is exactly the flat `drive / from / to / ease / hold` timeline
of AC7 — the declarative form of the current reels' hand-built `seq[]`. The
multi-mode `MotionMode` enum is **deferred to M4**, when its variants become
resolvable.

## Consequences

- The M1 schema only admits motion it can actually honour end-to-end; there is
  no "parses but cannot run" mode.
- `hold` fractions are validated to `[0, 1]` in `validate.rs`; `ease` is a
  closed `EaseName` enum validated at load.
- When M4 lands, `Motion` grows a mode (likely by wrapping today's flat form as
  the authored `Played` case), a backward-compatible extension rather than a
  rewrite.
- The divergence from the SPEC-0001 sketch is intentional and is called out in
  `model.rs` and [DESIGN §3.4](../DESIGN-guion-core.md#34-where-the-schema-diverges-from-the-spec-0001-sketch).

## Alternatives considered

- **Implement the full five-mode enum now.** Rejected: four of five modes have
  no resolver until M4, so they would be dead or, worse, misleading schema.
- **A free-form keyframe list.** Rejected for M1: the flat block reproduces the
  existing reel feel (dwell-in/dwell-out) with far less surface, and multi-track
  timelines are a named follow-up requirement.

## References

- `crates/guion-core/src/model.rs` (`Motion`, `Hold`, `EaseName`)
- `crates/guion-core/src/validate.rs` (`check_motion`)
