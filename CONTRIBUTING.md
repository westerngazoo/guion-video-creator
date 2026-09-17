# Contributing to `guion`

Thanks for looking under the hood. `guion` is small and rigorous by design; this
guide is how to keep it that way. The engineering method is in
[CLAUDE.md](CLAUDE.md) — this file is the practical checklist.

## Setup

```bash
# the toolchain is pinned in rust-toolchain.toml (1.83); rustup honours it
rustup show
cargo test --workspace     # should print 21 passed
```

## The loop

Every change of substance follows the artifact chain (see [CLAUDE.md §3](CLAUDE.md#3-the-methodology-loop)):

1. **Requirement** first (`docs/requirements/R-NNNN.md`) — state *what* and list
   numbered acceptance criteria. Copy the shape of an existing one, e.g.
   [R-0001](docs/requirements/0001-screenplay-schema.md).
2. **Spec** (`docs/specs/SPEC-NNNN.md`) — state *how*: modules, types, and the
   AC checklist it will satisfy.
3. **Implement**, with **one test per acceptance criterion**, and extend the
   traceability table in
   [DESIGN-guion-core.md §9](docs/DESIGN-guion-core.md#9-traceability).
4. If you had to decide something the requirement left open, **record it** in
   that requirement's decision log or as a new [ADR](docs/decisions/). A
   decision must never live only in code.

Small doc-only or fix-only changes don't need a new requirement — use judgment,
but keep the docs in sync in the same change.

## Before you push

All three must be green (this is exactly what CI runs — see
[`.github/workflows/ci.yml`](.github/workflows/ci.yml)):

```bash
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test --workspace
```

## Coding standards

See [CLAUDE.md §5](CLAUDE.md#5-coding-standards) and
[§6](CLAUDE.md#6-errors--failure-handling-fail-loudly-typed-located). In short:
public items are documented and tie back to the AC they serve; comments explain
*intent*, never restate code; input errors are typed, located, and never panic;
dependencies point inward and stay minimal.

## Branches, commits, PRs

- Branch from `main`; keep one logical change per commit with a clear message.
- A PR states which requirement/AC it advances and keeps the relevant docs and
  the traceability table in sync.
- Don't force-push shared branches or amend published commits without saying so.

## Adding an ADR

Copy the template shape from any file in [docs/decisions/](docs/decisions/):
**Status · Context · Decision · Consequences · Alternatives considered ·
References.** ADRs are append-only — supersede, never rewrite. Add a row to
[docs/decisions/README.md](docs/decisions/README.md).
