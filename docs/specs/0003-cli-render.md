# SPEC-0003 — `guion render` produces a frame sequence

- **Status:** Draft
- **Realizes:** R-0003
- **Author:** físico buen físico
- **Created:** 2026-08-28
- **Depends on:** SPEC-0001, SPEC-0002
- **Module(s):** `guion-cli`

## 1. Motivation

Realizes R-0003: the creator's command-line door and the harness that proves the M1
vertical slice end to end (RFC-0001 §8, §10 M1).

## 2. Design

### Module layout (`guion-cli`)

```
guion-cli/src/
├── main.rs       # arg parsing + dispatch (thin)
├── render.rs     # render command
├── check.rs      # check command
├── doctor.rs     # doctor command (seam pins)
└── exit.rs       # error → stderr + exit code mapping
```

The CLI holds no domain logic; it wires `guion-core` → `guion-assemble` → `motoreel`
(RFC-0001 §3.3, dependencies point inward).

### Commands

```
guion render <screenplay> [--out DIR] [--fps N]
guion check  <screenplay>
guion doctor
guion --help
```

### `render`

```rust
// render.rs
pub fn run(path: &Path, out: Option<PathBuf>, fps: Option<f64>) -> ExitCode {
    let sp = match guion_core::load_and_check(path) {          // R-0001
        Ok(sp) => sp, Err(e) => return exit::fail(e),         // AC3: no partial output
    };
    let theme = guion_brand::default_theme();
    let scene = match guion_assemble::assemble(&sp, &theme) {  // R-0002
        Ok(s) => s, Err(e) => return exit::fail(e),
    };
    let fps = fps.unwrap_or(sp.meta.fps);                      // AC2
    let dir = out.unwrap_or_else(|| default_out(&sp.meta.slug)); // out/<slug>/frames (Q1)
    let mut sink = PpmSink::new(&dir)?;                        // motoreel sink
    scene.render(fps, &mut sink)?;                             // AC1
    report(&dir, frame_count(&scene, fps));
    ExitCode::SUCCESS
}
```

Validation runs *before* any file is created, so a bad screenplay writes nothing
(AC3). Determinism is inherited from `motoreel`'s fixed-fps, derived-`t` render (AC6).

### `check`

Runs `guion_core::load_and_check` only; prints `ok` and exits 0, or prints the typed
error to stderr and exits non-zero. Writes no frames (AC4).

### `doctor`

Reads a `guion`-owned `seams.toml` pin file (RFC-0001 §7.1, R-0003 Q2) listing the
required versions of `motoreel`/`physics-lab`/`garust`, checks each sibling repo is
discoverable and compatible, prints a per-repo table, and exits non-zero if any required
seam is missing or incompatible (AC5). This is the runnable form of "orchestrate the
sub-projects" (RFC-0001 §7.2).

### Errors & help

`exit.rs` maps `GuionError`/`AssembleError`/`io::Error` to a clear stderr message + a
stable non-zero code. Arg parsing (standard Rust arg crate) provides `--help` and clean
usage errors for unknown flags (AC7).

## 3. Code outline

`main.rs` parses args and dispatches to `render`/`check`/`doctor`; each returns an
`ExitCode`. The render path above is the whole slice: file → validated model → scene →
frames. No logic beyond wiring lives in the CLI.

## 4. Non-goals

- Encoding/audio/vertical delivery (`guion encode`, M2).
- `new`/templates/`models`/`preview` (M5).

## 5. Open questions

- Default `--out` layout (R-0003 Q1): leaning `out/<slug>/frames/`.
- `doctor` version source (R-0003 Q2): leaning a `seams.toml` pin file.

## 6. Acceptance criteria

- [ ] `render --out DIR` writes `ceil(duration*fps)` frames, exit 0 (R-0003 AC1).
- [ ] `--fps` overrides; absent uses `meta.fps` (AC2).
- [ ] Invalid screenplay: `render`/`check` exit non-zero, typed error to stderr, no
      partial frames (AC3).
- [ ] `check` valid→0 / invalid→non-zero, no frames (AC4).
- [ ] `doctor` reports each seam, exits non-zero on missing/incompatible (AC5).
- [ ] reel-09 golden renders via CLI to expected frame count, deterministic (AC6).
- [ ] `--help` complete; unknown flags = clean usage error (AC7).

## 7. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-08-28 | M1 CLI = `render`/`check`/`doctor` only | Smallest surface proving slice + orchestration |
| 2026-08-28 | Validate before any file write | Safe reruns, fail fast (CLAUDE.md §6) |
| 2026-08-28 | `doctor` reads a `seams.toml` pin file | Concrete home for cross-repo contracts (RFC-0001 §7.1) |

## Changelog

- 2026-08-28 — created (draft).
