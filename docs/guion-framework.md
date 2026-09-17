<!-- Éste era el README del marco `guion` cuando vivía dentro del repo de
contenido (fisicobuenfisico/guion, PRs #4 y #5). Se conserva íntegro como
documento: describe el marco declarativo. El README de la raíz es el del
repo, y es el que carga el mandato del motor único. -->

# guion

**A declarative framework for physics animation.** Write a *screenplay* — a
single TOML file describing objects, physics, motion, labels, and brand — and
`guion` directs, films, and cuts it into a finished vertical video.

> *guión* (Spanish: "script / screenplay"). The creator writes the screenplay;
> the framework produces the film. `guion` sits on top of the
> [`motoreel`](docs/RFC-0001-guion-framework.md) render engine and the
> [`physics-lab`](docs/physics-lab/RFC-002-simulated-lessons.md) closed-form
> models, owning the creator-facing surface (schema, CLI, brand kit, templates)
> that none of the lower engine layers should own.

---

## Why

Today a physics reel is a bespoke Python + PIL script that hand-codes the
physics, draws every frame, hand-rolls easing, bakes in brand styling, and
shells out to ffmpeg. Every reel re-invents the pipeline, and the physics is
trustworthy but unverified and non-reusable.

`guion` replaces that per-reel craft with one declarative pipeline:

```
screenplay.toml → load + validate → assemble → render → encode → reel.mp4
```

The full motivation, layer diagram, and the load-bearing **`Model` × `Source`**
idea are in **[RFC-0001](docs/RFC-0001-guion-framework.md)**. If you read one
document, read that.

---

## Status

`guion` is early. This repository is deliberately honest about what is built
versus designed — see **[ROADMAP.md](docs/ROADMAP.md)** for the milestone board.

| Component | What it does | State |
|-----------|--------------|-------|
| **`guion-core`** | screenplay schema, loader, validator | **implemented — 21 tests green** |
| `guion-assemble` | screenplay → `motoreel::Scene` | designed (SPEC-0002), not built |
| `guion-cli` | `guion render` / `check` / `doctor` | designed (SPEC-0003), not built |
| `guion-motion` | `Model` × `Source` resolver | designed (RFC-0001 §3.2), M4 |
| `guion-brand` | theme, LUTs, fonts, post-fx | designed (RFC-0001 §5), M3 |
| `guion-encode` | ffmpeg + audio + vertical presets | designed (RFC-0001 §6), M2 |

Only `guion-core` is code today. Everything else is a written, reviewable
design. That is the point of this repo: a small, verified core plus a rigorous
paper trail you can audit before more code is written.

---

## Quickstart

```bash
# build + run the full check suite (fmt, clippy, tests)
cargo test --workspace

# use guion-core as a library
```

```rust
use guion_core::load_and_check;
use std::path::Path;

let sp = load_and_check(Path::new("reel.screenplay.toml"))?;
println!("{} @ {} fps", sp.meta.title, sp.meta.fps);
# Ok::<(), guion_core::GuionError>(())
```

A complete, working screenplay lives at
[`templates/reel-09-biceps.screenplay.toml`](templates/reel-09-biceps.screenplay.toml)
— it is the golden fixture the acceptance tests load and validate.

---

## Repository layout

```
guion/
├── Cargo.toml                  # workspace manifest
├── rust-toolchain.toml         # pinned toolchain (1.83)
├── crates/
│   └── guion-core/             # schema + loader + validator  (implemented)
├── docs/
│   ├── RFC-0001-guion-framework.md     # the founding design (read first)
│   ├── ARCHITECTURE.md                 # layered design, seams, data flow
│   ├── DESIGN-guion-core.md            # concrete design of the built crate
│   ├── ROADMAP.md                      # milestones M0–M5 + requirement board
│   ├── decisions/                      # ADRs (why the schema looks like it does)
│   ├── requirements/                   # R-NNNN: what to build & acceptance criteria
│   ├── specs/                          # SPEC-NNNN: how each requirement is realized
│   └── physics-lab/                    # sibling RFC context
├── templates/                  # starter screenplays
├── schema/                     # JSON Schema for editor autocompletion
└── .github/workflows/ci.yml    # fmt + clippy + test on every push
```

## Documentation map

Start at the top and drill down; each layer is more concrete than the last.

| If you want… | Read |
|--------------|------|
| the whole vision & architecture | [RFC-0001](docs/RFC-0001-guion-framework.md) |
| how the pieces fit and evolve | [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) |
| the milestone plan & honest status | [docs/ROADMAP.md](docs/ROADMAP.md) |
| the design of the one built crate | [docs/DESIGN-guion-core.md](docs/DESIGN-guion-core.md) |
| *why* a specific decision was made | [docs/decisions/](docs/decisions/) |
| what a feature must do (contract) | [docs/requirements/](docs/requirements/) |
| how a requirement is realized | [docs/specs/](docs/specs/) |
| how to contribute / the method | [CONTRIBUTING.md](CONTRIBUTING.md) |

---

## Verifying this work

Everything in `guion-core` is meant to be checkable in under a second:

```bash
cargo fmt --all --check      # formatting is clean
cargo clippy --all-targets -- -D warnings   # zero warnings
cargo test --workspace       # 21 tests: 3 unit + 17 acceptance + 1 doctest
```

The 17 acceptance tests map one-to-one to the acceptance criteria of
[R-0001](docs/requirements/0001-screenplay-schema.md) and
[R-0006](docs/requirements/0006-binding-tokens.md); the traceability table in
[DESIGN-guion-core.md](docs/DESIGN-guion-core.md#9-traceability) shows exactly
which test proves which criterion.

---

## License

Dual-licensed under either of [Apache-2.0](LICENSE-APACHE) or
[MIT](LICENSE-MIT) at your option.
