# R-0006 — Binding tokens & reference resolution

- **Status:** Discussing
- **Milestone:** M1
- **Owner:** físico buen físico (see project-specifics.md)
- **Created:** 2026-09-08
- **Depends on:** R-0001 (screenplay schema)
- **Realized by:** SPEC-0006 (pending)
- **QA:** `qa` agent run scoped to this requirement

> Numbering note: `R-0004+` (M4 source resolution) and `R-0005+` (M2 delivery)
> are already earmarked across the M1 requirements, so this carve-out takes the
> next free integer, `R-0006`. Renumber if the owner prefers a different slot.

## 1. Statement

A screenplay wires geometry, labels and motion to the outputs of the models and
objects it declares. guion must define the **binding token** — the tiny
reference language a creator writes, `@id` and `@id.field` — and the
**resolution** rule that turns every reference into either a declared target or
a typed, located error.

Two phases, matching R-0001's format boundary:

- **Grammar** is enforced when the screenplay is parsed: a malformed token fails
  loudly at load, before any semantics run.
- **Resolution** is a pure semantic pass over an otherwise well-formed
  screenplay: every token's *id* must name a declared `model`/`object`.

This capability is presently folded into R-0001. This requirement carves it out
so the token language and its resolution own their contract, acceptance
criteria and decision log independently of the schema's shape — the seam
`guion-assemble` and `guion-motion` will consume.

## 2. Rationale

Bindings are the joint between *what an object is* and *what a model computes*
(RFC-0001 §4) — the same idea as `physics-lab`'s readout slots. Every downstream
layer consumes *resolved* references, so the grammar and the resolution rule are
foundational and their error quality is load-bearing (the same argument R-0001
makes for the schema).

The first R-0001 implementation pass had to *invent* several binding rules that
the requirement did not pin down — the grammar's exact charset, whether model
outputs are referenced bare (`@elbow`) or qualified (`@lever.elbow`), how
tokens embedded in free strings are treated, and where grammar-vs-resolution
errors surface. Those decisions deserve to be written down and agreed, not
rediscovered on the next pass. (This requirement is the direct product of that
gap; see the decision log.)

## 3. Acceptance criteria

- **AC1 — Grammar.** `@id` and `@id.field` parse. The leading `@` is required;
  `id` and `field` are ASCII identifiers (`[A-Za-z_][A-Za-z0-9_]*`); at most one
  `.`. A malformed token (`elbow`, `@`, `@1x`, `@a.b.c`, `@a-b`) is a typed load
  error naming the offending token.
- **AC2 — Qualified outputs.** A model output is referenced *qualified* as
  `@model_id.output`. The *id* part is what resolves, so `@lever.elbow` is valid
  iff a `model`/`object` with id `lever` is declared.
- **AC3 — Resolution.** Every token used by an object shape, the motion drive,
  or a style paint resolves to a declared `model`/`object` id; a dangling
  reference is a typed error naming the token **and the site** where it was used.
- **AC4 — Embedded tokens.** Tokens embedded in a free-string field (e.g. a
  `stroke = "heat:@lever.force_N"` paint) are extracted and resolved; a dangling
  embedded token is caught with the same error shape as AC3.
- **AC5 — Bare id references.** A direct id reference that is not a token (e.g. a
  label anchor's `object`) resolves against declared object ids with the same
  error shape.
- **AC6 — Determinism.** Resolution is pure and side-effect free and reports the
  *first* failure in a fixed traversal order — the same bytes always yield the
  same result or the same error (R-0001 AC8).
- **AC7 — Id-only for M1.** Only the *id* part is resolved. The *field* part is
  **not** checked against a model's output catalogue here, because the models
  are not loaded yet; field validation is deferred to when a `Source` resolves
  (R-0004+, M4).

## 4. Constraints & non-goals

- **Grammar for M1 is `@id` and `@id.field` only.** Anchor sub-selectors
  (`start`/`mid`/`end`) are typed fields on the label/shape schema, not part of
  the token grammar (R-0001 Q2).
- **No field-existence check.** Validating `@lever.elbow`'s `elbow` against the
  `lever` model's declared outputs needs the model loaded — out of scope until
  M4.
- **No expression language.** A token is a reference, never an expression.
- `std` + serde-family deps only; dependencies point inward (`CLAUDE.md` §2).

## 5. Open questions

- **Q1.** Sub-selectors in the grammar (`@obj.mid`) vs typed anchor fields.
  Recommend typed fields (keeps the grammar to `@id`/`@id.field`).
- **Q2.** When models load (M4), should the *field* part be validated against
  the model's declared outputs, and under which requirement? Recommend yes, as
  part of the source-resolution requirement.
- **Q3.** Identifier charset/case: ASCII `[A-Za-z_][A-Za-z0-9_]*` for M1 —
  confirm no Unicode identifiers and that ids are case-sensitive.
- **Q4.** Should bare-id references (a label anchor's `object`) become tokens
  (`@object`) for uniformity, or stay bare? Recommend staying bare — they are
  direct id fields, not output bindings.

## 6. Decision log

Decisions taken during the first R-0001 implementation pass (2026-09-08),
recorded here for confirmation. Append-only.

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-09-08 | Grammar is `@id` / `@id.field`, at most one `.`, ASCII identifiers | Smallest useful surface (RFC-0001 §11 Q2) |
| 2026-09-08 | Model outputs are referenced qualified, `@model_id.output` | So every token resolves against a *declared* id; RFC-0001 §4's bare `@elbow` was illustrative, not resolvable |
| 2026-09-08 | Grammar validated at load (typed load error); resolution at validate (dangling-ref error) | Fail fast on shape; keep semantic checks in one deterministic pass |
| 2026-09-08 | Tokens embedded in free strings (`heat:@…`) are extracted and resolved | A dangling paint binding is still a creator bug and must fail |
| 2026-09-08 | Resolve the *id* only; defer *field* validation to M4 | Models are not loaded at schema time; honest scope for M1 |

## Changelog

- 2026-09-08 — created (draft, Discussing); carved out of R-0001 after its first
  implementation pass surfaced the binding rules as under-specified.
