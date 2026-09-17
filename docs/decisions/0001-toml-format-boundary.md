# ADR-0001 — TOML as the authoring format, behind a format boundary

- **Status:** Accepted
- **Date:** 2026-08-28
- **Deciders:** owner + implementer
- **Realizes:** [R-0001](../requirements/0001-screenplay-schema.md) §4;
  [RFC-0001 §11 Q2](../RFC-0001-guion-framework.md#11-open-questions--decisions-needed)

## Context

A screenplay is written and edited by hand by a creator who does not write Rust.
The format must be human-first. Three candidates were on the table:

- **TOML** — human-first, ubiquitous, great for flat config, weak at deep enums.
- **RON** — maps 1:1 to Rust enums (`Source`/`Shape`), but is Rust-flavoured and
  unfamiliar to non-Rust creators.
- **JSON** — tooling-first, but noisy and comment-less for hand authoring.

The mandate ("creators without Rust") points at TOML; the schema's enum-shaped
fields (which RON would render most naturally) point at RON. The choice is
independent of the model-interop decision.

## Decision

**Author in TOML for M1**, and put all syntax knowledge behind a *format
boundary* — a single `load.rs` module with a `Syntax` enum — so the typed model
in `model.rs` never learns which on-disk format produced it.

Enum-shaped fields are made TOML-friendly with internal `kind` tags
(see [ADR-0004](0004-deny-unknown-fields-and-kind-tagging.md)); a JSON Schema
(`schema/`) drives editor autocompletion.

## Consequences

- Adding RON or JSON later is a new arm of the `Syntax` match and a new
  `parse`/`emit` branch — `model.rs`, `validate.rs`, and `bind.rs` do not change.
- `to_string` (serialize) lives at the same boundary, giving the round-trip
  guarantee (R-0001 AC1) for free per syntax.
- `load` dispatches on file extension and does so **before** reading the file,
  so an unknown extension fails as `UnknownFormat` regardless of file existence.
- RON stays the documented fallback if enum ergonomics in TOML prove painful for
  creators.

## Alternatives considered

- **RON as the primary format.** Rejected for M1: it optimizes for the Rust
  author, not the creator, contradicting the core mandate. Kept as a fallback
  precisely because the boundary makes switching cheap.
- **No boundary (parse TOML directly in the model).** Rejected: it would weld
  the schema to one syntax and make a future RON/JSON front-end a rewrite.

## References

- `crates/guion-core/src/load.rs`
- [DESIGN-guion-core.md §5](../DESIGN-guion-core.md#5-the-format-boundary-loadrs)
