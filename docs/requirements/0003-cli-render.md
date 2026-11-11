# R-0003 — `guion render` produces a frame sequence

- **Status:** Discussing
- **Milestone:** M1
- **Owner:** físico buen físico (see project-specifics.md)
- **Created:** 2026-08-28
- **Depends on:** R-0001, R-0002
- **Realized by:** SPEC-0003
- **Extendido por:** R-0007 — el CLI reconoce el segundo dialecto,
  `[comparacion]`. R-0003 daba por hecho que sólo había uno
- **QA:** `qa` agent run scoped to this requirement

## 1. Statement

guion must provide a command-line entry point that takes a screenplay file and produces
a numbered frame sequence on disk: `guion render <screenplay.toml> [--out DIR]
[--fps N]`. The command loads and validates the screenplay (R-0001), assembles the scene
(R-0002), renders through a `motoreel` sink, and reports what it produced. A companion
`guion check <screenplay.toml>` validates only, and `guion doctor` reports whether the
sibling engine repos are present and on compatible pinned versions (RFC-0001 §7.2).

## 2. Rationale

The CLI is the creator's actual door into the framework and the harness that proves the
M1 vertical slice end to end (RFC-0001 §8, §10 M1). `check` gives fast authoring
feedback without a full render; `doctor` operationalizes the "orchestrate the
sub-projects" contract as a runnable command.

## 3. Acceptance criteria

- **AC1.** `guion render fixture.toml --out DIR` exits 0 and writes
  `ceil(duration * fps)` numbered frame files into `DIR`.
- **AC2.** `--fps` overrides the screenplay `meta.fps`; absent, `meta.fps` is used.
- **AC3.** A schema-invalid screenplay causes `render` and `check` to exit non-zero and
  print the R-0001 typed error (field path + reason) to stderr; no partial output is
  written for `render`.
- **AC4.** `guion check fixture.toml` exits 0 on a valid file and non-zero with the typed
  error on an invalid one, writing no frames in either case.
- **AC5.** `guion doctor` reports, for each of `motoreel`/`physics-lab`/`garust`,
  whether it is discoverable and version-compatible with the pins, and exits non-zero if
  any required seam is missing or incompatible.
- **AC6.** The reel-09 lever golden screenplay renders through the CLI to the expected
  frame count; the run is deterministic (identical bytes in, identical frames out).
- **AC7.** `--help` documents every command and flag; unknown flags/args are a clean
  usage error, not a panic.

## 4. Constraints & non-goals

- **No encoding.** `render` stops at frames; `guion encode` (ffmpeg + audio + vertical
  presets) is M2 (R-0005+).
- **No `new`/templates/`models`/`preview`.** Those creator-toolkit commands are M5.
- CLI parsing via a standard Rust arg crate; the CLI holds no logic beyond wiring
  `guion-core` → `guion-assemble` → `motoreel` (dependencies point inward).

## 5. Open questions

- Q1. Default `--out` directory: `out/<slug>/frames/` (mirrors the current reels' layout)
  vs a flat `out/`. Recommend `out/<slug>/frames/` for parity with existing habits.
- Q2. `doctor` version source: git tags, `Cargo.toml` version fields, or a `guion`-owned
  compatibility manifest. Recommend a small `seams.toml` pin file in `guion` (RFC-0001
  §7.1) read by `doctor`.

## 6. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-08-28 | M1 CLI ships `render`, `check`, `doctor` only | Smallest surface that proves the slice and the orchestration contract |
| 2026-08-28 | `render` writes no partial output on validation failure | Predictable, safe reruns; fail fast (CLAUDE.md §6) |

## Changelog

- 2026-08-28 — created (draft, Discussing).
