# SPEC-0001 — Screenplay schema, loader & validator

- **Status:** Draft
- **Realizes:** R-0001
- **Author:** físico buen físico
- **Created:** 2026-08-28
- **Depends on:** none
- **Module(s):** `guion-core`

## 1. Motivation

Realizes R-0001: a single typed, validated model for a declarative screenplay, the
contract every other guion crate consumes (RFC-0001 §3.3, §4).

## 2. Design

### Module layout (`guion-core`)

```
guion-core/src/
├── lib.rs        # re-exports; load() / check() entry points
├── model.rs      # typed screenplay structs + enums (the schema)
├── load.rs       # format boundary: bytes → Screenplay (TOML for M1)
├── validate.rs   # semantic checks beyond deserialization
├── bind.rs       # binding token grammar (@id, @id.field) + reference table
└── error.rs      # typed errors (LoadError, ValidateError) with field paths
```

### Typed model (schema)

`serde`-derived with `#[serde(deny_unknown_fields)]` on every struct (AC3). Enum-shaped
fields use internally-tagged `kind` (RFC-0001 §11 Q2) so TOML stays flat and readable.

```rust
// model.rs — representative, not final
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Screenplay {
    pub meta: Meta,
    pub camera: Camera,
    #[serde(default)] pub model: Vec<ModelRef>,
    #[serde(default)] pub object: Vec<ObjectSpec>,
    #[serde(default)] pub label: Vec<LabelSpec>,
    pub motion: Motion,
    #[serde(default)] pub hook: Option<Hook>,
    #[serde(default)] pub footer: Option<Footer>,
    #[serde(default)] pub audio: Option<Audio>,
}

#[serde(deny_unknown_fields)]
pub struct Meta { pub title: String, pub slug: String,
    pub format: Format, pub fps: f64, #[serde(default)] pub theme: Option<String>,
    pub lang: String }

pub enum Format { Vertical }   // M1: vertical only (R-0001 Q1)

#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Shape {
    Point { at: Token },
    Segment { from: Token, to: Token },
    Polyline { points: Vec<Token> },
    Edges { verts: Vec<Token>, edges: Vec<(usize, usize)> },
    Arrow { at: Token, value: Token, scale: Scale },
}

// The screenplay-level motion mode; each maps to a guion `Source` (RFC-0001 §3.2)
// at resolve time. M1 loads all, but only `played` (keyframes/spin) resolves.
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum MotionMode {
    Keyframes { keys: Vec<KeyframeEntry>, ease: EaseName },      // → authored Played (M1)
    Spin { rate: f64, plane: PlaneName, duration: f64 },        // → authored Played (M1)
    InPlace { model: String, bind: BTreeMap<String, Token>, sample: TimeSampling }, // → InPlace (M4)
    Stepped { model: String, bind: BTreeMap<String, Token>, dt: f64 },              // → Stepped (M4)
    Played  { model: String, bind: BTreeMap<String, Token>, dt: f64, steps: usize },// → bake→Played (M4)
}
```

`Token` is a newtype over `String` validated by `bind.rs` into `@id` / `@id.field`.

### Loading (format boundary)

```rust
// load.rs
pub fn load(path: &Path) -> Result<Screenplay, LoadError>;      // reads + parses
pub fn from_str(src: &str, fmt: Format) -> Result<Screenplay, LoadError>;
```

`load` dispatches on extension (`.toml` for M1) so RON/JSON slot in later without
touching `model.rs`. Deserialization errors are mapped to `LoadError` carrying the
serde path (AC2/AC3).

### Validation

`validate(&Screenplay) -> Result<(), ValidateError>` runs the checks deserialization
cannot: `fps > 0`, known `format`/`ease`/`plane`, `hold` fractions in `[0,1]` (AC4/AC7),
and reference resolution via a symbol table of declared `model`/`object` ids; every
`Token` used anywhere must resolve or yield a `ValidateError::DanglingRef { token, at }`
(AC6). `load_and_check(path)` = `load` then `validate`.

### Errors

```rust
pub enum LoadError { Io(..), Parse { path: String, msg: String } }
pub enum ValidateError {
    MissingField { path: String },
    UnknownKind { field: String, got: String, valid: &'static [&'static str] },
    OutOfRange { field: String, value: f64, bound: &'static str },
    DanglingRef { token: String, at: String },
}
```

All typed, all naming the offending location (CLAUDE.md §6; R-0001 AC2–AC7).

## 3. Code outline

The `Screenplay` struct tree + `load`/`validate` free functions above. `bind.rs` exposes
`Token::parse(&str) -> Result<Token, ...>` and a `SymbolTable` built from declared ids.
No I/O beyond reading the given path; no env/network (AC8).

```rust
pub fn load_and_check(path: &Path) -> Result<Screenplay, GuionError> {
    let sp = load(path)?;      // LoadError
    validate(&sp)?;            // ValidateError
    Ok(sp)
}
```

## 4. Non-goals

- Resolving `in_place`/`stepped`/baked `played` sources to state-over-time (R-0004+, M4)
  — only structural parse/validate here.
- Assembly or rendering (R-0002).
- RON/JSON front-ends (deferred; boundary is in place).

## 5. Open questions

- Whether `Token` sub-selectors (`@obj.mid`) are parsed here or in the label/shape layer.
  Leaning: grammar knows `@id` and `@id.field`; anchor selectors are typed fields.

## 6. Acceptance criteria

- [ ] Round-trip load→serialize→load is structurally identical (R-0001 AC1).
- [ ] Missing required field → typed error with path (AC2).
- [ ] Unknown field rejected (AC3).
- [ ] `fps>0`, known `format`, non-empty `lang` enforced (AC4).
- [ ] Unknown shape/style `kind` → typed error listing valid kinds (AC5).
- [ ] Dangling `@` reference → typed error naming token + site (AC6).
- [ ] Motion consistency: `from`/`to`/`ease`/`hold∈[0,1]` (AC7).
- [ ] Load is deterministic, side-effect free (AC8).
- [ ] reel-09 lever fixture loads + validates clean (AC9).

## 7. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-08-28 | `deny_unknown_fields` everywhere; `kind`-tagged enums | Loud typos + flat TOML (R-0001 AC3, §11 Q2) |
| 2026-08-28 | Format dispatch isolated in `load.rs` | RON/JSON deferrable without model churn |

## Changelog

- 2026-08-28 — created (draft).
