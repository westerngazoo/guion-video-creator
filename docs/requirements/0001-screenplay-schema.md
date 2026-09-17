# R-0001 — Screenplay schema, loader & validator

- **Status:** Discussing
- **Milestone:** M1
- **Owner:** físico buen físico (see project-specifics.md)
- **Created:** 2026-08-28
- **Depends on:** none
- **Realized by:** SPEC-0001
- **QA:** `qa` agent run scoped to this requirement

## 1. Statement

guion must define a **screenplay**: a single declarative file that fully describes a
physics animation — metadata, camera, objects and their shapes, labels, and a motion
timeline — without any Rust or imperative drawing code. guion must load a screenplay
from disk into a typed, in-memory model and validate it, surfacing every problem as a
typed, human-readable error that names the offending field and location.

The schema is the creator-facing contract of the whole framework (RFC-0001 §4). It
carries *what the animation is*, never *how it is drawn*: physics is referenced by
model id, geometry by shape, motion by a declarative timeline. For M1 the only source
mode that must load and validate is the authored `Played` mode (keyframe/spin); the
other modes of the `Model` × `Source` split (RFC-0001 §3.2) — `in_place`, `stepped` —
parse into the model but are not required to *resolve* until M4.

## 2. Rationale

Today every reel re-implements its own pipeline in Python (RFC-0001 §1.2). A single
validated schema is what turns per-reel craft into a reusable framework and lets other
creators author without touching engine internals. Every downstream layer
(`guion-assemble`, `guion-motion`, `guion-encode`) consumes this typed model, so its
correctness and error quality are foundational — the same role R-0001 plays in
`motoreel`.

## 3. Acceptance criteria

- **AC1.** A well-formed screenplay file loads into a typed `Screenplay` value with all
  fields populated; a round-trip (load → serialize → load) is structurally identical.
- **AC2.** Required top-level sections (`meta`, `camera`) and required fields within
  them are enforced; a missing required field is a typed error naming the field path.
- **AC3.** Unknown/misspelled fields are rejected (deny-unknown-fields), not silently
  ignored, with an error naming the unexpected key.
- **AC4.** `meta.fps > 0`, `meta.format` is a known preset, and `meta.lang` is a
  non-empty tag; violations are typed errors, never panics.
- **AC5.** Object `shape` and `style` enums accept exactly the documented `kind`
  values; an unknown kind is a typed error listing the valid kinds.
- **AC6.** Cross-references resolve: every binding token (`@id`, `@id.field`) used by an
  object, label, or motion refers to a declared `model`/`object` id; a dangling
  reference is a typed error naming the token and where it was used.
- **AC7.** A `[motion]` timeline validates its own consistency: `from`/`to` present,
  `ease` is a known ease, and any `hold` fractions lie in `[0, 1]`.
- **AC8.** Loading is deterministic and side-effect free: the same bytes always produce
  the same `Screenplay` or the same error; no network, no environment reads.
- **AC9.** At least one real screenplay — the reel-09 lever example from RFC-0001 §4 —
  loads and validates clean, exercised as a golden fixture.

## 4. Constraints & non-goals

- **Format:** authoring format is TOML for M1 (RFC-0001 §11 Q2, leaning TOML); the
  loader is format-boundaried so RON/JSON could be added later without touching the
  typed model. No expression language or scripting in the file.
- **No resolution of physics motion here.** Parsing/validating an `in_place` or
  `stepped` motion block is out of scope beyond structural checks; *resolving* a
  `Source` to state-over-time is R-0004+ (M4).
- **No rendering, no assembly.** This requirement stops at a validated typed model.
- `std` + serde-family deps only; dependencies point inward (`CLAUDE.md` §2).

## 5. Open questions

- Q1. Exact set of `format` presets for M1 (at least `vertical` 1080×1920). Recommend
  shipping `vertical` only in M1, add `square`/`wide` with M2 delivery.
- Q2. Binding token grammar: `@id` and `@id.field` only, or also indexed/anchored forms
  (`@obj.mid`)? Recommend `@id` and `@id.field` for M1; anchor sub-selectors handled by
  the label/shape schema, not the token grammar.

## 6. Decision log

Decisions made together (owner + Claude). Append-only.

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-08-28 | Screenplay is a single declarative file; no imperative drawing | The core reusability thesis of RFC-0001 (owner) |
| 2026-08-28 | Format is TOML for M1, behind a format boundary | Creator-first mandate; RON/JSON deferrable (RFC-0001 §11 Q2) |
| 2026-08-28 | Unknown fields are rejected, not ignored | Typos must fail loudly; protects the creator contract (CLAUDE.md §6) |

## Changelog

- 2026-08-28 — created (draft, Discussing).
