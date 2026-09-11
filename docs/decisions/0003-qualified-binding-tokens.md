# ADR-0003 — Model outputs are referenced by qualified token (`@model.output`)

- **Status:** Accepted
- **Date:** 2026-09-08 (formalized as [R-0006](../requirements/0006-binding-tokens.md))
- **Deciders:** owner + implementer
- **Realizes:** [R-0001 AC6](../requirements/0001-screenplay-schema.md#3-acceptance-criteria);
  [R-0006](../requirements/0006-binding-tokens.md)

## Context

The RFC's illustrative screenplay references model outputs *bare*:

```toml
shape = { segment = { from = "@elbow", to = "@hand" } }   # RFC-0001 §4, illustrative
```

But `@elbow` names nothing resolvable: there is no declared id `elbow`. The
model is declared as `id = "lever"`, and `elbow` is one of *its* outputs. For the
validator to give AC6's guarantee — *every* binding resolves to a declared
`model`/`object` id — a token must carry the id it belongs to.

This gap surfaced only during implementation, which is why
[R-0006](../requirements/0006-binding-tokens.md) was later carved out of R-0001:
to give the binding language its own agreed contract instead of leaving it
inferred.

## Decision

Model outputs are referenced **qualified**: `@model_id.output`, e.g.
`@lever.elbow`, `@lever.phi`, `@lever.biceps_force_N`. The grammar is `@id` or
`@id.field` (at most one `.`, ASCII identifiers). The **id** part is what
resolves against the symbol table; the **field** part is *not* checked against
the model's output catalogue in M1, because models are not loaded yet — that is
deferred to source resolution (M4).

Tokens embedded in free strings (e.g. `stroke = "heat:@lever.force_N"`) are
extracted and resolved with the same rule.

## Consequences

- Every token in a loaded screenplay resolves against a declared id, or is a
  located `DanglingRef` error — AC6 is total.
- The RFC's bare `@elbow` illustrative form is superseded; the golden fixture
  uses the qualified form.
- Field-existence validation has an honest, explicit home (M4), rather than
  being silently skipped or faked.
- The grammar's charset and one-dot rule are pinned in R-0006 AC1 and unit
  tested (`bind::tests::rejects_malformed_tokens`).

## Alternatives considered

- **Bare outputs with a flat global namespace.** Rejected: it needs a
  cross-model output index that does not exist at schema time, and collides when
  two models expose the same output name.
- **Full dotted paths (`@lever.joint.elbow`).** Rejected for M1: more than one
  `.` adds grammar complexity with no M1 use case; revisitable when models load.

## References

- `crates/guion-core/src/bind.rs` (`Token`, `Token::extract`, `SymbolTable`)
- `crates/guion-core/src/validate.rs` (`check_refs`, `collect_refs`)
- [R-0006 decision log](../requirements/0006-binding-tokens.md#6-decision-log)
