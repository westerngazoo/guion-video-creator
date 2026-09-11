# Architecture Decision Records

An ADR records one architecturally significant decision: the context that
forced it, the decision taken, and the consequences accepted. They are
append-only and immutable — a decision is *superseded* by a later ADR, never
edited away.

These five ADRs capture the decisions the first `guion-core` implementation had
to make where the requirement or spec left a choice open. They exist so the next
contributor inherits the reasoning instead of rediscovering it (and so a
reviewer can challenge a decision at its root, not by reverse-engineering the
code). [R-0006](../requirements/0006-binding-tokens.md) is the requirement-level
formalization of ADR-0003.

| ADR | Decision | Status |
|-----|----------|--------|
| [0001](0001-toml-format-boundary.md) | TOML behind a format boundary | Accepted |
| [0002](0002-motion-as-flat-timeline.md) | `[motion]` is a flat timeline, not a mode enum, for M1 | Accepted |
| [0003](0003-qualified-binding-tokens.md) | Model outputs are referenced qualified (`@model.output`) | Accepted |
| [0004](0004-deny-unknown-fields-and-kind-tagging.md) | `deny_unknown_fields` + internal `kind` tagging | Accepted |
| [0005](0005-two-phase-error-taxonomy.md) | Two-phase error taxonomy (`LoadError` vs `ValidateError`) | Accepted |

## Format

Each ADR follows a light [MADR](https://adr.github.io/madr/)-style template:
**Status · Context · Decision · Consequences · Alternatives considered ·
References.**
