# ADR-0004 — `deny_unknown_fields` everywhere + internal `kind` tagging

- **Status:** Accepted
- **Date:** 2026-08-28
- **Deciders:** owner + implementer
- **Realizes:** [R-0001 AC3, AC5](../requirements/0001-screenplay-schema.md#3-acceptance-criteria);
  [RFC-0001 §11 Q2](../RFC-0001-guion-framework.md#11-open-questions--decisions-needed)

## Context

The screenplay *is* the creator contract. Two failure modes must be impossible:

1. A **misspelled field** silently ignored (`tilte = "..."` doing nothing while
   `title` stays empty) — a class of bug that wastes hours.
2. An **unknown enum variant** accepted, or rejected with an unhelpful message.

serde offers `deny_unknown_fields` for (1) and internal tagging (`tag = "kind"`)
for readable polymorphic TOML for (2). But serde has a well-known limitation:
`deny_unknown_fields` **cannot** be combined with internal tagging on the same
enum, because the tag is itself an "extra" field to the variant's deserializer.

## Decision

- Put `#[serde(deny_unknown_fields)]` on **every plain struct** — `Meta`,
  `Camera`, `ModelRef`, `ObjectSpec`, `Style`, `Motion`, `Hold`, and the rest.
- Tag enum-shaped fields internally with `kind` (`Shape`) or as string presets
  (`Format`, `CameraKind`, `Scale`, `EaseName`, `AnchorAt`), so an unknown value
  is rejected with a message that lists the valid ones.
- **Accept** that `Shape` (internally tagged) cannot also
  `deny_unknown_fields`. This is a bounded gap: a real typo lands on a plain
  struct field (rejected), and the `kind` value is still validated.

## Consequences

- A misspelled key anywhere in the flat structure is a loud
  `LoadError::Parse` naming the key (AC3, `ac3_unknown_field_is_rejected`).
- An unknown `shape.kind` lists the five valid kinds (AC5,
  `ac5_unknown_shape_kind_lists_valid_kinds`).
- A stray *sibling* field inside a `shape = { kind = "...", ... }` inline table
  is **not** rejected — the one accepted soft spot, documented in `model.rs`.
- The TOML stays flat and hand-writable, satisfying the creator-first mandate.

## Alternatives considered

- **Adjacently-tagged enums** (`{ type = "segment", value = { ... } }`).
  Rejected: they *do* allow `deny_unknown_fields` but force a nested `value`
  wrapper that makes hand-authoring noticeably worse.
- **Externally-tagged enums.** Rejected for the same readability reason.
- **A custom `Deserialize` that re-checks fields.** Rejected as disproportionate
  hand-rolled code for a soft spot a typo cannot realistically hit unnoticed.

## References

- `crates/guion-core/src/model.rs` (struct/enum derives; the note on `Shape`)
- [DESIGN-guion-core.md §3.2](../DESIGN-guion-core.md#32-internal-kind-tagging-for-enum-shaped-fields)
