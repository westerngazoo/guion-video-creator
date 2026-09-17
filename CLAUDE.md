# CLAUDE.md — engineering method for `guion`

This file is the shared operating manual for anyone (human or agent) working in
`guion`. It is intentionally short and load-bearing: several requirements, specs,
and code comments cite it by section number (e.g. "`CLAUDE.md` §2", "§6"), so the
section numbering here is stable.

## 1. Purpose

`guion` is Layer 2 of a four-repo ecosystem (see
[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)). It turns a declarative screenplay
into a finished video by composing the `motoreel` render engine and the
`physics-lab` model catalogue. This file defines *how we build it* so the result
stays coherent across many small changes.

## 2. Layering & dependencies (dependencies point inward)

The single structural rule: **dependencies point inward, toward the schema.**

- `guion-core` (the schema + loader + validator) depends on nothing but
  `serde` and `toml`. It never imports an engine. It is the stable contract.
- Every other crate depends on `guion-core`, and coupling to a downstream engine
  lives in exactly **one** crate (`motoreel` → `guion-assemble`;
  `physics-lab`/`garust` → `guion-motion`).
- No crate reaches "sideways" into another guion crate's internals; they compose
  through `guion-core`'s types.

Consequences: the schema can be evolved without touching engines, an engine's
API can move without churning the schema, and when a seam does change, exactly
one crate's tests break first — `guion` is the ecosystem's integration canary.

Dependency policy: prefer `std`; add a dependency only when it earns its place.
`guion-core` is `std` + serde-family only (error types are hand-written, no
`thiserror`).

## 3. The methodology loop

Work flows through four artifacts, smallest surface first:

1. **RFC** (`docs/RFC-NNNN`) — a design proposal for a whole area. Rare;
   [RFC-0001](docs/RFC-0001-guion-framework.md) is the founding one.
2. **Requirement** (`docs/requirements/R-NNNN`) — *what* a capability must do,
   with numbered **acceptance criteria** and an append-only **decision log**.
3. **Spec** (`docs/specs/SPEC-NNNN`) — *how* a requirement is realized: module
   layout, types, and the acceptance-criteria checklist it will satisfy.
4. **Implementation + tests** — code, with **one test per acceptance criterion**
   and a traceability table (see [DESIGN-guion-core.md](docs/DESIGN-guion-core.md#9-traceability)).

Status legend (doc lifecycle):
`Backlog → Discussing → Spec'd → In progress → In review → Done`.

When implementation forces a decision the requirement left open, **record it** —
either in the requirement's decision log or as an [ADR](docs/decisions/). Do not
let a decision live only in code. (The existence of
[R-0006](docs/requirements/0006-binding-tokens.md) is a direct application of
this rule.)

## 4. Documents & where they live

| Kind | Path | Owns |
|------|------|------|
| RFC | `docs/RFC-*.md` | area-level design |
| Requirement | `docs/requirements/*.md` | contract + acceptance criteria |
| Spec | `docs/specs/*.md` | realization plan |
| ADR | `docs/decisions/*.md` | one significant decision, immutable |
| Architecture / Design / Roadmap | `docs/*.md` | the cross-cutting picture |

## 5. Coding standards

- **Rust 2021**, toolchain pinned in `rust-toolchain.toml` (the MSRV).
- `cargo fmt` clean; `cargo clippy --all-targets -- -D warnings` clean.
- Doc comments on every public item, tying it back to the requirement/AC it
  serves. Module docs state the module's single responsibility.
- Comments explain *intent and trade-offs*, never restate the code.
- Small, single-responsibility modules; no cyclic deps within a crate.

## 6. Errors & failure handling (fail loudly, typed, located)

The creator's file *is* the contract, so failure handling is a feature, not an
afterthought:

- **Never panic on input.** Every bad input is a typed `Result` error.
- **Name the location.** An error names the offending field path, key, `kind`,
  or token *and where it was used* — never just "invalid input."
- **Fail loud on the unknown.** Unknown/misspelled fields are rejected
  (`deny_unknown_fields`), never silently ignored. Unknown enum values list the
  valid ones.
- **Two phases, aligned to where the failure is caught.** Structural/grammar
  errors surface at load (`LoadError`); semantic errors at validation
  (`ValidateError`). See [ADR-0005](docs/decisions/0005-two-phase-error-taxonomy.md).
- **Deterministic.** The same bytes always produce the same value or the same
  first error, in a fixed traversal order. No env, no network at load.
- **No partial output on failure.** A command that fails validation writes
  nothing.

## 7. Definition of done

A change is done when:

1. It satisfies its requirement's acceptance criteria, each with a test.
2. `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings`, and
   `cargo test --workspace` are all green.
3. Any decision it forced is recorded (decision log or ADR).
4. The docs that describe it (design/roadmap/traceability) are updated in the
   same change.
