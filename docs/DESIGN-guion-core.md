# Design: `guion-core`

The concrete design of the one crate that is actually built. This is the
document to read if you want to *audit* the implementation: it explains every
non-obvious decision, the trade-offs each one forced, where the code diverges
from its own spec sketch (and why), and a line-by-line map from acceptance
criteria to the tests that prove them.

- **Realizes:** [R-0001](requirements/0001-screenplay-schema.md) (schema, loader,
  validator) and [R-0006](requirements/0006-binding-tokens.md) (binding tokens).
- **Specified by:** [SPEC-0001](specs/0001-screenplay-schema.md).
- **Dependencies:** `serde` + `toml` only. Error types are hand-written; no
  `thiserror`.

---

## 1. Responsibility

`guion-core` is the **format boundary and typed contract** of the framework. It
answers exactly one question and answers it totally:

> Given some bytes, are they a valid screenplay — and if not, *precisely* what
> is wrong and where?

It stops at a validated in-memory `Screenplay`. It does not assemble, render,
resolve physics, or touch the network. Every other `guion` crate consumes the
type this crate produces, which is why its correctness and its *error quality*
are foundational.

The public happy path is one call:

```rust
use guion_core::load_and_check;
use std::path::Path;

let sp = load_and_check(Path::new("reel.screenplay.toml"))?; // LoadError | ValidateError
```

---

## 2. Module layout

```
crates/guion-core/src/
├── lib.rs        # re-exports; check() / load_and_check() entry points
├── model.rs      # the typed screenplay schema (structs + enums)
├── load.rs       # format boundary: bytes ⇄ Screenplay (TOML for M1)
├── validate.rs   # semantic checks deserialization cannot do
├── bind.rs       # binding token grammar (@id, @id.field) + symbol table
└── error.rs      # typed errors: LoadError, ValidateError, GuionError
```

Each module maps to a section of [SPEC-0001 §2](specs/0001-screenplay-schema.md#2-design).
The dependency order inside the crate is `error ← {model, bind} ← {load,
validate} ← lib`, with no cycles.

---

## 3. The typed model (`model.rs`)

The schema is a tree of `serde`-derived structs and enums. Three design rules
govern all of them.

### 3.1 `deny_unknown_fields` on every struct

Every plain struct carries `#[serde(deny_unknown_fields)]`. A misspelled key
(`tilte = "..."`) is a **loud error**, not a silently dropped field. This is the
single most important property for a creator-facing format: the file *is* the
contract, and a typo in the contract must fail. See
[ADR-0004](decisions/0004-deny-unknown-fields-and-kind-tagging.md).

### 3.2 Internal `kind` tagging for enum-shaped fields

Polymorphic fields (`Shape`, and the enum presets `Format`, `CameraKind`,
`Scale`, `EaseName`, `AnchorAt`) use serde's internal tagging so the TOML stays
flat and hand-writable:

```toml
shape = { kind = "segment", from = "@lever.elbow", to = "@lever.hand" }
```

An unknown `kind` produces an error that *lists the valid kinds* — the
deserializer already knows them. This is [R-0001 AC5](requirements/0001-screenplay-schema.md#3-acceptance-criteria).

> **Trade-off (documented in `model.rs`).** serde does **not** support
> `deny_unknown_fields` *together with* internal tagging on the same enum. So
> `Shape` cannot reject unknown *sibling* fields. This is an accepted, bounded
> gap: a real typo lands on one of the plain structs (which do reject it), and
> the `kind` itself is still validated. The alternative — adjacently-tagged or
> externally-tagged enums — would make the TOML markedly uglier for the creator,
> which violates the primary mandate. See
> [ADR-0004](decisions/0004-deny-unknown-fields-and-kind-tagging.md).

### 3.3 Bindings are a validated newtype

Every reference into a model/object output is a [`Token`](#4-binding-tokens-bindrs),
not a bare `String`. Because `Token` validates its grammar at deserialization
time, *any* `Token` value in a loaded `Screenplay` is guaranteed well-formed —
the type system carries the invariant.

### 3.4 Where the schema diverges from the SPEC-0001 sketch

[SPEC-0001](specs/0001-screenplay-schema.md) explicitly labels its `model.rs` as
"representative, not final." Two places where the implementation deliberately
departs, both driven by the requirement's acceptance criteria:

| SPEC-0001 sketch | Implementation | Why |
|------------------|----------------|-----|
| a rich `MotionMode` enum (`Keyframes`/`Spin`/`InPlace`/`Stepped`/`Played`) | a flat `Motion { drive, from, to, ease, hold }` | [R-0001 AC7](requirements/0001-screenplay-schema.md#3-acceptance-criteria) and [RFC-0001 §4](RFC-0001-guion-framework.md#4-the-screenplay-format-creator-surface) describe M1's timeline as exactly this flat block; the multi-mode enum's *resolution* is M4. Loading a mode we cannot yet resolve would be dishonest. See [ADR-0002](decisions/0002-motion-as-flat-timeline.md). |
| bare model outputs (`@elbow`) | qualified outputs (`@lever.elbow`) | so that *every* token resolves against a *declared* id ([R-0001 AC6](requirements/0001-screenplay-schema.md#3-acceptance-criteria)); RFC-0001 §4's bare form was illustrative, not resolvable. See [R-0006](requirements/0006-binding-tokens.md) and [ADR-0003](decisions/0003-qualified-binding-tokens.md). |

These divergences are not drift — they are the requirement winning over an
illustrative sketch, and each is recorded as an ADR so the next contributor
inherits the reasoning rather than re-litigating it. (R-0006 exists *because*
the first implementation pass had to invent the binding rules; carving it out
turned an inferred decision into an agreed contract.)

---

## 4. Binding tokens (`bind.rs`)

Two responsibilities live here so nothing else re-implements them.

**`Token` owns the grammar.** `@id` or `@id.field`, where `id`/`field` are ASCII
identifiers `[A-Za-z_][A-Za-z0-9_]*` and there is at most one `.`. `Token::parse`
is the only constructor and it runs inside `Deserialize`, so a malformed token
fails at **load** as a `LoadError::Parse`. Accessors `id()`, `field()`, and
`as_str()` decompose a token without re-parsing.

**`Token::extract`** pulls every well-formed `@…` out of a free string (e.g. a
`stroke = "heat:@lever.force_N"` paint). Malformed fragments inside a free
string are *skipped*, not raised — a free string is not a token field, so only
its resolvable tokens matter ([R-0006 AC4](requirements/0006-binding-tokens.md#3-acceptance-criteria)).

**`SymbolTable` owns resolution.** It is the set of declared `model.id` and
`object.id` values; `validate.rs` asks it whether a token's *id* is declared.

> **Scope honesty.** Only the *id* is resolved for M1. The *field* part
> (`@lever.elbow`'s `elbow`) is **not** checked against the model's output
> catalogue, because models are not loaded at schema time — that check is
> deferred to when a `Source` resolves in M4 ([R-0006 AC7](requirements/0006-binding-tokens.md#3-acceptance-criteria)).

---

## 5. The format boundary (`load.rs`)

All syntax knowledge is isolated here so `model.rs` never learns which format it
came from. `load` dispatches on file extension **before** touching the disk — an
unsupported extension fails identically whether or not the file exists (this
ordering is a deliberate correctness choice, exercised by
`unknown_extension_is_typed_error`). Adding RON or JSON later is a new `Syntax`
arm and nothing else changes ([ADR-0001](decisions/0001-toml-format-boundary.md)).

```rust
pub fn load(path: &Path) -> Result<Screenplay, LoadError>;      // read + parse
pub fn from_str(src: &str, syntax: Syntax) -> Result<Screenplay, LoadError>;
pub fn to_string(sp: &Screenplay, syntax: Syntax) -> Result<String, LoadError>;
```

`to_string` is the inverse used by the round-trip guarantee (AC1). Loading is
side-effect free: the only I/O is reading the path handed in.

---

## 6. Validation (`validate.rs`)

By the time `validate` runs, structure/fields/kinds/token-grammar are already
guaranteed by the format boundary. What remains is *meaning*, checked in a
fixed traversal order so the first failure is deterministic (AC8):

1. `check_meta` — `meta.fps` is finite and `> 0`; `meta.lang` is non-empty.
2. `check_motion` — any `hold` fraction lies in `[0, 1]`.
3. `check_refs` — every binding token and bare id reference resolves against
   the `SymbolTable`, in order: `motion.drive`, then each object's shape
   endpoints and embedded stroke tokens, then each label anchor. A miss is a
   `ValidateError::DanglingRef { token, at }` naming the token *and the site*.

`meta.format` and `motion.ease` need no check here — their enums already
constrained them to known presets at load.

> **A subtle correctness point.** `fps` is checked as
> `!fps.is_finite() || fps <= 0.0`, not `!(fps > 0.0)`. This rejects `NaN`
> explicitly and avoids a negated float comparison (clippy's
> `neg_cmp_op_on_partial_ord`). Small, but the sort of thing this crate is
> supposed to get right.

---

## 7. Error taxonomy (`error.rs`)

Two families mirroring the two phases, plus a union for the combined entry
point:

```rust
pub enum LoadError { Io{..}, Parse{..}, UnknownFormat{..}, Emit{..} }
pub enum ValidateError { OutOfRange{..}, Empty{..}, DanglingRef{..} }
pub enum GuionError { Load(LoadError), Validate(ValidateError) }
```

All three implement `Display` (in Spanish, matching the creator audience) and
`std::error::Error`; `From` conversions let `load_and_check` use `?`.

This taxonomy is *narrower* than SPEC-0001's sketch, on purpose. The sketch had
`ValidateError::MissingField` and `::UnknownKind`; in reality those are caught by
`serde` at **load** time, so they are `LoadError::Parse` (whose `msg` already
names the field / bad kind and its valid alternatives). Inventing separate
validate variants for errors that can never reach validation would be dead code.
Conversely, `LoadError` gained `UnknownFormat` (bad extension) and `Emit`
(serialize failure on the round-trip path) that the sketch omitted. The
divergence is recorded in [ADR-0005](decisions/0005-two-phase-error-taxonomy.md).

---

## 8. Public API surface

Re-exported from the crate root (`lib.rs`):

- **Entry points:** `load`, `from_str`, `to_string`, `check`, `load_and_check`,
  `Syntax`.
- **Errors:** `GuionError`, `LoadError`, `ValidateError`.
- **Bindings:** `Token`, `SymbolTable`.
- **Model:** `Screenplay`, `Meta`, `Format`, `ModelRef`, `Camera`, `CameraKind`,
  `Pose`, `ObjectSpec`, `Shape`, `Scale`, `Style`, `LabelSpec`, `Anchor`,
  `PoseAnchor`, `AnchorAt`, `Motion`, `Hold`, `EaseName`, `Hook`, `Span`,
  `Footer`, `Audio`.

The crate-level doc comment carries a runnable example, verified as a doctest.

---

## 9. Traceability

Every acceptance criterion of R-0001 and R-0006 is proven by a named test.
`tests/screenplay.rs` holds the acceptance tests (one per criterion); `bind.rs`
holds the grammar unit tests. All 21 pass (`cargo test --workspace`).

### R-0001 — schema, loader, validator

| AC | Criterion | Proven by | Code |
|----|-----------|-----------|------|
| AC1 | round-trip load→serialize→load is identical | `ac1_roundtrip_is_structurally_identical` | `load::to_string` / `from_str`, `#[derive(PartialEq)]` |
| AC2 | missing required field names it | `ac2_missing_required_field_names_it` | serde required fields → `LoadError::Parse` |
| AC3 | unknown field rejected | `ac3_unknown_field_is_rejected` | `deny_unknown_fields` |
| AC4 | `fps>0`, known `format`, non-empty `lang` | `ac4_fps_must_be_positive`, `ac4_lang_must_be_non_empty`, `ac4_unknown_format_is_rejected_at_load` | `validate::check_meta`, `Format` enum |
| AC5 | unknown shape kind lists valid kinds | `ac5_unknown_shape_kind_lists_valid_kinds` | `#[serde(tag = "kind")]` on `Shape` |
| AC6 | dangling `@` ref names token + site | `ac6_dangling_motion_drive`, `ac6_dangling_shape_and_label_refs`, `ac6_embedded_stroke_token_resolves` | `validate::check_refs`, `bind` |
| AC7 | motion consistency (`from`/`to`/`ease`/`hold∈[0,1]`) | `ac7_missing_from_is_a_load_error`, `ac7_unknown_ease_is_a_load_error`, `ac7_hold_fraction_out_of_range` | `Motion` struct, `validate::check_motion` |
| AC8 | deterministic, side-effect free | `ac8_same_bytes_same_result`, `unknown_extension_is_typed_error`, `load_and_check_reports_validate_errors` | `load`, `validate` order |
| AC9 | reel-09 golden fixture loads + validates | `ac9_golden_fixture_loads_and_validates` | whole crate + `tests/fixtures/reel-09-biceps.screenplay.toml` |

### R-0006 — binding tokens

| AC | Criterion | Proven by | Code |
|----|-----------|-----------|------|
| AC1 | grammar `@id` / `@id.field` | `bind::tests::parses_id_and_field`, `bind::tests::rejects_malformed_tokens` | `Token::parse`, `valid_ident` |
| AC2 | qualified outputs resolve on id | `ac9_golden_fixture_loads_and_validates` | `Token::id`, `SymbolTable` |
| AC3 | resolution names token + site | `ac6_dangling_motion_drive`, `ac6_dangling_shape_and_label_refs` | `validate::check_refs` |
| AC4 | embedded tokens extracted + resolved | `ac6_embedded_stroke_token_resolves`, `bind::tests::extracts_embedded_tokens` | `Token::extract` |
| AC5 | bare id references resolve | `ac6_dangling_shape_and_label_refs` (label anchor) | `validate::collect_refs` |
| AC6 | determinism | `ac8_same_bytes_same_result` | fixed traversal order |
| AC7 | id-only for M1, field deferred | *(deliberate non-check — §4 scope honesty)* | documented; field resolution is M4 |

---

## 10. Known limitations & deferred work

Stated plainly so a reviewer is not surprised:

- **Field-existence is not checked.** `@lever.elbow` resolves on `lever`; the
  `elbow` output is validated only once models load (M4).
- **`Shape` cannot reject unknown sibling fields** (the serde tagging trade-off,
  §3.2). Bounded and documented.
- **Only `vertical` format and one `Scale` (`auto`)** ship in M1; more arrive
  with the delivery (M2) and brand (M3) layers.
- **Motion is a single flat drive.** Multi-track timelines and the physics
  source modes (`in_place` / `stepped` / baked `played`) are M4.

None of these are bugs; each is a scoped M1 boundary with a home on the
[roadmap](ROADMAP.md).

---

## 11. Verify

```bash
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test --workspace
```

Expected: formatting clean, zero clippy warnings, `21 passed` (3 unit + 17
acceptance + 1 doctest).
