# R-0015 — The studio: reels assembled from gadgets

- **Status:** In progress (phase 1 implemented locally, awaiting the owner's sign-off)
- **Milestone:** M2
- **Owner:** físico buen físico (see project-specifics.md)
- **Created:** 2026-10-20
- **Depends on:** R-0009 (the reel studio: `frame(t)`, the parallel MP4, `--sheet`, `--frames`, `eventos.json`), R-0007/R-0008/R-0011 (voice and score), R-0012 (the gate), R-0013 (the dashboard), R-0014 (accounts: `cuenta.toml`, guion-marca)
- **Realized by:** SPEC-0015
- **QA:** `qa` agent run scoped to this requirement

## 1. Statement

The owner (2026-10-20): «arranca el studio R-0015». A creator assembles a
reel from **gadgets** — a lever with its torque, a gauge, a live graph, a
formula built from the drawing, a low-poly mesh under a 3D camera, the
voice — in a studio with one template per account: close to a complete
engine, and an edge over Manim where Manim is weakest for this house:
every number carries its provenance, every instant comes from the voice,
and every frame is byte-deterministic.

The approved prototype
(<https://claude.ai/artifact/WFNNeGRWSmovXarHYPwP1t>) has: an account
switcher (the template changes with the account); a gadget library by
shelf (Física, Geometría, 3D, Datos, Texto y voz); a live 9:16 viewport;
an inspector with live params and a provenance tag on each value
(calculado / medido / reportado / supuesto); a timeline with one clock for
voice, beats, gadget clips, the 3D camera, `eventos.json` and the score;
and the gate's checks.

Phase 1 (this change) builds the contract that everything else stands on,
two real gadgets that prove it, and a minimal studio that edits a post's
gadgets live and saves them to the post's own files.

## 2. Rationale

Every reel of the house is code today: Rotor's and PMMO's scenes in
`rotorf-sico` (`brand/rf-marca`: `medidor`, `tres.rs`), fbf's in
`fisicobuenfisico/motor` (`escena.rs`: `palanca_con`, the τ gauge;
`curl.rs`, `polea.rs`). The same pieces are rewritten per reel and per
account, with their numbers hand-placed and their label positions found by
trial; a new account starts from nothing. The pieces are already pure
functions of time (R-0009) — what is missing is a contract that makes them
reusable, typed, checked, and editable without recompiling.

## 3. Acceptance criteria

- **AC1. The gadget contract.** A gadget declares typed params
  (`ParamSpec`: number with range, step and unit; text; option; palette
  token), each physical one marked so; outputs (`SalidaSpec`); and a pure
  `eval(t, params, contexto) → Salida { prims, eventos, valores, avisos }`.
  Every computed value leaves as `calculado`; a physical input carries a
  `Procedencia` (calculado, medido, reportado, supuesto), and a calculado,
  medido or reportado one its `fuente`. Each gadget may add static asserts
  (`comprobar`) and oracles; `comprobar_contrato` checks any gadget
  (defaults in range, oracles reproduced, `eval` pure, outputs declared and
  returned).
- **AC2. Two real gadgets.** «palanca» (τ = F·L·sen θ, the moment arm as a
  dimension line, the line of action dashed, the weight as an arrow, one
  label placed by obstacle avoidance, or an aviso if none fits) and
  «medidor» (a bar gauge with a fixed mark; its reading keeps its input's
  provenance), generalised from fbf's and Rotor's code. The physics is
  physics-lab's `mecanica::palanca`, consumed, never copied. Their oracles
  are fbf's model numbers: F·L = 62.784 N·m with 20 kg and 0.32 m.
- **AC3. The registry.** Built-in gadgets plus the account's: an account
  names its gadgets in `cuenta.toml` `[estudio] gadgets` (a slug each;
  `motor`, `bin` default to `motor/Cargo.toml`, `estudio`), registers them
  in its motor crate next to the built-ins, and serves the same studio
  (`guion_estudio::run_con`). The built-in studio opening a post that uses
  one says which binary to run. A duplicate or non-slug `tipo` is refused.
- **AC4. The timeline.** Clips (`entra`, `sale`) and animations (`desde`,
  `hasta`) are anchored to the voice: `inicio`, `fin`, a line id (its start)
  or `<id>.fin`, `± seconds`; a bare number is refused. Params may bind to
  an earlier gadget's output (`{ de = "brazo.tau" }`, no cycles by
  construction). The clock (`eventos.json`) gets `voz:<id>`, `<id>:entra`,
  `<id>:sale`, the first frame each gadget event holds, and a series per
  output.
- **AC5. `escena.toml` is the source of truth.** The post's gadgets live in
  `escena.toml` next to `voz.toml` and `sonido.toml`; the studio edits it
  and nothing else; load → save → load is the identity and saving twice
  gives the same bytes. Unknown keys, types, ranges, provenance and anchors
  fail at load, by name, in file order.
- **AC6. `guion studio <post>`.** A local server on 127.0.0.1 (std only),
  one embedded page: the post's gadgets and the library, a slider per param
  with its provenance tag, a scrub bar and play, the frame at `t` rendered
  by guion-reel on demand, values and avisos, a timeline of the voice and
  the clips; «guardar» writes `escena.toml`. Headless: `--cuadro T` (a
  PNG), `--check` (every aviso of every frame; exit 1 on any), and
  `--video | --sheet | --frames | --events` (guion-reel's render). The
  template is the account's (`cuenta.toml` above the post) or the neutral
  one. No gdk: it builds and runs in CI and in the cloud.
- **AC7. Determinism.** The same scene and `t` give the same frame bytes:
  across calls, across studio instances, through the HTTP socket, and the
  PNG holds exactly guion-reel's frame (`Reel::frame_rgb`, which equals the
  frame `frames_png` writes).

## 4. Constraints & non-goals

- No new crate in the lock: the server is `std::net`; PNG through `png`
  0.18 (already locked through the Tauri apps' icon crates); JSON through
  serde_json.
- The server binds 127.0.0.1 only, serves one request at a time, and has
  no authentication: it is a local tool, not a service.
- Not in phase 1: the 3D gadgets, graphs, formulas, the voice and score
  tracks as editable clips, the gate strip, dragging clips on the timeline,
  account switching from the page (the template follows the post), and
  `guion reel` reading `escena.toml` without a motor binary (§5).
- The existing `apps/guion-studio` (Tauri, the screenplay pipeline of
  R-0003, needs gdk/webkit, excluded from CI) is **replaced**, not reused:
  its model is the old screenplay, not `frame(t)`. It stays in the tree
  until the owner confirms its removal (Q1).

## 5. Phases (plan; SPEC-0015 §8 has the detail)

1. **The contract** (this change): guion-gadget, palanca and medidor,
   escena.toml, `guion studio` minimal.
2. **The library.** Gráfica viva (a curve that draws itself with the
   voice, axes and ticks by the house rules), fórmula armada (pieces that
   enter from the drawing; motoreel R-0015 inline formulas), tabla de
   condiciones, polea/cable (`mecanica::palanca::Linea::Cable`), círculo de
   fricción (physics-lab `vehiculo`), subtítulo and título from
   guion-marca as gadgets, regla y compás (PMMO). `guion reel` renders
   `escena.toml` directly (no motor binary for a gadget-only post).
3. **3D.** Camera and orbit, low-poly meshes, Lambert quantized to the
   palette, garust motors, painter's order — no GPU (SPEC-0015 §5).
4. **The full studio.** Voice and score tracks (guion-audio), clips dragged
   to anchors, the gate strip live (R-0012), the account switcher across
   the dashboard's repos (R-0013), the contact sheet from the page.
5. **Migration.** fbf's curl and polea reels, Rotor's 1.3 (`tres.rs`), as
   gadgets; their reels re-rendered byte-compared against the code versions
   where the drawing is meant to be identical.

## 6. Open questions

- Q1. Remove `apps/guion-studio` (and `docs/STUDIO.md`'s Tauri part) now
  that `guion studio` replaces it? Recommend yes, in its own change, once
  the owner has used the new one.
- Q2. Should a scene mix code and gadgets (a motor binary that draws its
  own beats and evaluates `escena.toml` for the rest)? Recommend yes in
  phase 2: `Compilada::cuadro` is already a plain function a scene can
  call.
- Q3. Edits made to `escena.toml` by hand while the studio is open:
  reload on change, or refuse to save over them? Recommend: the studio
  keeps the file's modification time and refuses «guardar» when it moved,
  offering to reload (phase 2).
- Q4. Should the server render frames in parallel for play? Phase 1
  renders one frame per request (about 0.4 s per frame, ÷2, in a debug build, process start included); recommend a small
  cache keyed by (scene hash, t) before parallelism.

## 7. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-10-20 | Two new crates: guion-gadget (the contract, registry, timeline, escena.toml, built-ins; couples to motoreel prims and `mecanica`) and guion-estudio (template, server, page; renders through guion-reel) | CLAUDE.md §2: the contract stays testable without a server; the server is replaceable without touching a gadget |
| 2026-10-20 | Replace `apps/guion-studio` rather than extend it | It is Tauri (gdk/webkit, out of CI) over the screenplay pipeline; R-0015 is `frame(t)` and must run in CI and the cloud |
| 2026-10-20 | The UI is a web page served on 127.0.0.1 by a std-only server | No toolkit dependency; the browser is the window; the router is a pure function testable without a socket |
| 2026-10-20 | A physical param is always written with its provenance; layout params never carry one | Rule 2 of the accounts («todo número tiene procedencia») as a type, not a review comment |
| 2026-10-20 | Anchors only: a bare number for a time is a load error | The voice moves when re-synthesised; a typed time goes stale silently |
| 2026-10-20 | Bindings read only earlier gadgets | No cycles, a fixed evaluation order, no solver |
| 2026-10-20 | `escena.toml` is written canonically (spec order, shortest round-trip floats); comments are not kept | The same scene, the same bytes; the studio owns the file, its header says so |
| 2026-10-20 | The first animation of a param governs from its clip's start (holding `de`); later ones take over when they start | A repetition starts at rest without a second keyframe; chained animations do not fight |
| 2026-10-20 | `Reel::frame_rgb`: one frame in memory, no ffmpeg | The studio needs a frame per slider move; the bytes are the MP4's |
| 2026-10-20 | `cuenta.toml` gets an optional `[estudio]` (gadgets, motor, bin) | Account gadgets are Rust; the contract only names them, so the shared studio can point to the right binary |

## Changelog

- 2026-10-20 — created; phase 1 realized by SPEC-0015 in the same change.
