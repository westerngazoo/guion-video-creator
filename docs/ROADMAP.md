# Roadmap

The milestone plan for `guion`, and an honest account of what is built versus
designed. It expands the milestones of
[RFC-0001 §10](RFC-0001-guion-framework.md#10-milestones-proposed) into a
requirement board.

## How to read this

Two different things get tracked, and conflating them is how roadmaps lie:

- **Doc state** — the lifecycle of a requirement/spec *document*:
  `Backlog → Discussing → Spec'd → In progress → In review → Done`.
- **Build state** — whether *code* exists and passes its acceptance criteria:
  `— (none) · partial · green`.

A requirement can be **built green while its document is still `Discussing`**
(the owner has not formally signed the spec off) — that is exactly the situation
`guion-core` is in today, and this board shows it rather than hiding it.

---

## Milestone summary

| Milestone | Theme | State |
|-----------|-------|-------|
| **M0** | Foundation — repo, methodology, adopt the RFC | **in progress** (this repo) |
| **M1** | First screenplay — schema → assemble → frames | **in progress** (core built) |
| M2 | Delivery — ffmpeg + audio + vertical presets | designed |
| M3 | Brand — theme, LUTs, fonts, post-fx | designed |
| M4 | Physics motion — the `Model` × `Source` split | designed |
| M5 | Creators — `new`, templates, `models`, `doctor` | designed |

---

## M0 — Foundation

Adopt the methodology and stand up the repository. No product code.

| Item | State |
|------|-------|
| `guion` repo as a Rust workspace per [RFC-0001 §3.3](RFC-0001-guion-framework.md#33-workspace--crate-layout) | **done** (this tree) |
| Founding design doc adopted (`docs/RFC-0001`) | **done** |
| Architecture / design / roadmap docs written | **done** |
| Methodology files (`CONTRIBUTING.md`, `CLAUDE.md`, `requirements/`, `specs/`, `decisions/`) | **done** |
| CI (fmt + clippy + test) green | **done** |
| Decisions recorded: model interop = `.wasm`; format = TOML | **done** ([decisions/](decisions/)) |
| Seams pinned to real `motoreel`/`physics-lab`/`garust` versions | not started — those repos are not yet vendored |
| `guion doctor` stub | M5 |

M0 is substantially complete for the parts that do not require the sibling
engine repositories to be present.

---

## M1 — First screenplay

Turn a keyframe-only screenplay into rendered frames through `motoreel`. This
realizes `motoreel`'s backlogged R-0008 ("declarative scenes") inside `guion`.
Physics-sourced motion is deferred to M4; M1 proves the
authoring → assembly → render slice with the authored timeline only.

| Req | Capability | Crate | Spec | Doc state | Build state |
|-----|------------|-------|------|-----------|-------------|
| [R-0001](requirements/0001-screenplay-schema.md) | schema + loader + validator | `guion-core` | [SPEC-0001](specs/0001-screenplay-schema.md) | Discussing | **green** — 21 tests, all 9 ACs |
| [R-0006](requirements/0006-binding-tokens.md) | binding tokens + resolution | `guion-core` | SPEC-0006 *(pending)* | Discussing | **green** — grammar + resolution built & tested |
| [R-0002](requirements/0002-scene-assembly.md) | screenplay → `motoreel::Scene` | `guion-assemble` | [SPEC-0002](specs/0002-scene-assembly.md) | Discussing | — |
| [R-0003](requirements/0003-cli-render.md) | `guion render` → frame sequence | `guion-cli` | [SPEC-0003](specs/0003-cli-render.md) | Discussing | — |

**Where M1 stands.** The creator-facing contract — the schema, the loader, the
validator, and the binding language — is *built and verified*. What remains for
M1 is the assembly seam to `motoreel` (R-0002) and the CLI that wires it
(R-0003); both are specced and blocked only on a `motoreel` dependency being
available to this workspace. M1 deliberately stops at frames on disk so the
vertical slice is provable without the delivery layer.

---

## M2 — Delivery

`guion-encode`: numbered frames → branded, audio-mixed vertical `.mp4`.
Reproduces and generalizes the current ffmpeg + audio flow, with named format
presets (`vertical`, `square`, `wide`) carrying safe-area metadata. Earmarked as
`R-0005+`. Designed in [RFC-0001 §6](RFC-0001-guion-framework.md#6-rendering--delivery-pipeline-guion-encode).

## M3 — Brand

`guion-brand` + `themes/fbf`: port the físico buen físico identity (palettes,
LUTs, fonts, halftone, grain, title treatments, mark primitives) into a
reusable, overridable theme so other creators can bring their own. Designed in
[RFC-0001 §5](RFC-0001-guion-framework.md#5-the-brand-kit-guion-brand).

## M4 — Physics motion

`guion-motion`: the load-bearing `Model` × `Source` split — `InPlace`
(closed-form via `physics-lab` `.wasm`), `Stepped` (live integration), and
`Played` (bake once, scrub freely, incl. `garust`/`record`). This is where the
`in_place`/`stepped` motion that M1 only parses becomes resolvable state over
time, and where a token's *field* part is finally validated against a model's
declared outputs. Earmarked as `R-0004+`. Designed in
[RFC-0001 §3.2](RFC-0001-guion-framework.md#32-the-one-load-bearing-idea-model-what--source-how).

## M5 — Creators

The creator toolkit: `guion new --template <t>`, a template catalogue,
`guion models` (list available `physics-lab` models), and `guion doctor`
(check sibling engines are present, compatible, and green). Designed in
[RFC-0001 §8](RFC-0001-guion-framework.md#8-how-other-creators-use-it).

---

## Migration from the Python pipeline

Independent of the milestones, the existing `fisicobuenfisico/tools/` reels are
retired incrementally, never in a big bang
([RFC-0001 §9](RFC-0001-guion-framework.md#9-migration-path-from-the-current-python-pipeline)):
extract 1–2 reels' physics into tested `physics-lab` models → build the M1→M2
vertical slice → port the brand kit → re-cut existing reels as screenplays. The
Python tools stay until each reel has a screenplay equivalent.
