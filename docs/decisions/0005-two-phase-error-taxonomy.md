# ADR-0005 — Two-phase error taxonomy: `LoadError` vs `ValidateError`

- **Status:** Accepted
- **Date:** 2026-08-28 (refined during the R-0001 implementation pass)
- **Deciders:** owner + implementer
- **Realizes:** [R-0001 AC2–AC8](../requirements/0001-screenplay-schema.md#3-acceptance-criteria);
  [SPEC-0001 §2](../specs/0001-screenplay-schema.md#2-design)

## Context

R-0001 demands that *every* problem be a typed, located error — never a panic,
never a silent pass. Problems fall into two natural phases:

- **Structural / grammatical** — bad syntax, a missing required field, an
  unknown field, an unknown enum `kind`, a malformed `@` token. These are
  detected by `serde`/`toml` *during deserialization*.
- **Semantic** — a number out of range, an empty tag, a reference to an
  undeclared id. These can only be checked *after* a well-formed value exists.

SPEC-0001 sketched the error enums early, before the implementation revealed
exactly which phase catches which failure. Its sketch put `MissingField` and
`UnknownKind` under `ValidateError` — but those are caught by serde at load time
and can never reach a semantic pass.

## Decision

Model the two phases as two error types, aligned to *where the failure is
actually caught*:

```rust
pub enum LoadError { Io, Parse, UnknownFormat, Emit }   // the format boundary
pub enum ValidateError { OutOfRange, Empty, DanglingRef } // the semantic pass
pub enum GuionError { Load(LoadError), Validate(ValidateError) }
```

- `LoadError::Parse` carries the serde/toml diagnostic verbatim, which already
  names the missing field, the unexpected key, or the unknown `kind` and its
  valid alternatives — so no separate `MissingField`/`UnknownKind` variants are
  needed.
- `LoadError` gains `UnknownFormat` (bad extension) and `Emit` (serialize
  failure on the round-trip path), which the sketch omitted.
- Error messages are written in Spanish, matching the creator audience.
- No `thiserror`: R-0001 restricts deps to `std` + serde-family, so `Display`
  and `Error` are hand-written.

## Consequences

- The taxonomy is *narrower and truer* than the sketch: no variant exists that
  cannot occur. Divergence from SPEC-0001 is deliberate and recorded here.
- `load_and_check` composes both phases with `?` via `From` conversions into
  `GuionError`.
- Determinism (AC8): validation reports the **first** failure in a fixed
  traversal order, so identical bytes yield identical errors.

## Alternatives considered

- **A single flat error enum.** Rejected: it blurs the phase a caller is in and
  invites variants that cannot happen.
- **Keep the SPEC-0001 variants verbatim.** Rejected: `MissingField` /
  `UnknownKind` would be unreachable dead code, which is worse than an honest
  update to the spec's own "representative, not final" sketch.

## References

- `crates/guion-core/src/error.rs`
- `crates/guion-core/src/validate.rs` (traversal order)
- [DESIGN-guion-core.md §7](../DESIGN-guion-core.md#7-error-taxonomy-errorrs)
