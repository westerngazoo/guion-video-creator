# SPEC-0015 — The studio

- **Status:** In progress (phase 1 in review)
- **Realizes:** R-0015
- **Created:** 2026-10-20
- **Module(s):** `guion-gadget` (new crate), `guion-estudio` (new crate), `guion-reel` (`Reel::frame_rgb`), `guion-cuenta` (`[estudio]`), `guion-cli` (`guion studio`)

## 1. Design

```
crates/guion-gadget/src/          # the contract: no server, no window
├── lib.rs        # the crate's map
├── valor.rs      # Procedencia, Dato, Tipo, ParamSpec, SalidaSpec, Valor, Params (AC1)
├── gadget.rs     # Gadget, Contexto, Paleta, Salida, Oraculo, Categoria (AC1)
├── registro.rs   # Registro, comprobar_contrato, estantes (AC1, AC3)
├── linea.rs      # Tiempos (anchors), Curva, Tramo, animar, opacidad (AC4)
├── escena.rs     # Escena (escena.toml), compilar → Compilada: cuadro(t), reloj, avisos; a_toml (AC4, AC5)
├── dibujo.rs     # px drawing (bars, dashes, arrows, discs, text), colocar (obstacle avoidance)
└── integrados/   # palanca.rs, medidor.rs (AC2)

crates/guion-estudio/src/         # `guion studio <post>`
├── lib.rs        # run, run_con (an account's binary), the command line
├── plantilla.rs  # the account's template from cuenta.toml, or the neutral one
├── estudio.rs    # Estudio: the post in memory; cuadro_png, estado, fijar, guardar, comprobar, render
├── http.rs       # std-only server; responder (the router, no socket), servir
└── ui.html       # the page, embedded with include_str!
```

Dependencies point inward (CLAUDE.md §2): guion-gadget depends on
guion-reel (canvas, clock, voice, easing), motoreel (primitives),
motoreel-typeset (measuring labels) and physics-lab's `mecanica`;
guion-estudio adds guion-cuenta (the contract), `png` and serde_json. No
gadget knows the server; no server code knows a gadget's internals.

## 2. The gadget contract (AC1)

```rust
pub enum Procedencia { Calculado, Medido, Reportado, Supuesto }
pub enum Dato { Num(f64), Texto(String) }
pub enum Tipo {
    Num { min: f64, max: f64, paso: f64, unidad: &'static str, defecto: f64 },
    Texto { defecto: &'static str },
    Opcion { opciones: &'static [&'static str], defecto: &'static str },
    Token { defecto: &'static str },          // a palette role or token
}
pub struct ParamSpec { nombre, doc, tipo: Tipo, fisico: bool }
pub struct SalidaSpec { nombre, unidad, doc }
pub struct Valor { nombre, cantidad: f64, unidad, procedencia: Procedencia, nota }

pub struct Contexto<'a> { canvas: Canvas, paleta: &'a Paleta, caras: Option<&'a Fonts>, cara: usize, seguro: [f64; 4] }
pub struct Salida { prims: Vec<Prim2>, eventos: Vec<String>, valores: Vec<Valor>, avisos: Vec<String> }

pub trait Gadget: Send + Sync {
    fn tipo(&self) -> &'static str;           // the slug escena.toml uses
    fn categoria(&self) -> Categoria;         // Física, Geometría, 3D, Datos, Texto y voz
    fn doc(&self) -> &'static str;
    fn params(&self) -> &'static [ParamSpec];
    fn salidas(&self) -> &'static [SalidaSpec];
    fn eval(&self, t: f64, p: &Params, cx: &Contexto) -> Salida;   // pure
    fn comprobar(&self, p: &Params, cx: &Contexto) -> Vec<String>; // static asserts (default: none)
    fn oraculos(&self) -> Vec<Oraculo>;                            // default: none
}
```

- **`t`** is the time since the clip started; the params are already at
  that instant (animated, bound). A gadget never reads the clock, the
  file or the account.
- **`eventos`** are states that hold at `t` («maximo», «tope»); the scene
  writes the first frame each holds as `<id>:<evento>`.
- **`avisos`** are the asserts at an instant (a label that found no room,
  a gauge past its scale); `guion studio --check` fails on any.
- **Colours** are asked by role (`texto`, `secundario`, `acento`, `fondo`,
  `foco`) or token; [`Paleta`] resolves a role through the account's
  `[marca.roles]`. The same gadget takes every account's palette.
- **Text** is set in the account's label face; without faces a gadget
  estimates its label box (so placement asserts still run) and draws no
  text, saying so.
- **Pure.** No clock, environment, hash order or state between calls.
  `comprobar_contrato` evaluates twice and compares.

### Provenance rules (enforced at load)

| Param | Must carry |
|---|---|
| physical (`fisico: true`) | `procedencia`; with calculado, medido or reportado, a non-empty `fuente` |
| physical, bound (`de = …`) | nothing: it takes its source's provenance |
| layout | no `procedencia` (an error if written) |
| physical, not written | an error: never left to a default |

Every output a gadget computes is `calculado`. A gauge's `lectura` keeps
the provenance of the value it shows.

## 3. The registry (AC3)

`Registro::integrados()` holds the built-ins; `registrar(Box<dyn Gadget>)`
adds one (a slug, not taken). An account with its own gadgets:

```toml
# cuenta.toml
[estudio]
gadgets = ["kart-tres"]        # the tipo of each gadget it adds
motor = "motor/Cargo.toml"     # default
bin = "estudio"                # default
```

```rust
// <account>/motor/src/bin/estudio.rs
fn main() -> std::process::ExitCode {
    let mut r = guion_gadget::Registro::integrados();
    r.registrar(Box::new(mi_motor::KartTres)).unwrap();
    let args: Vec<String> = std::env::args().skip(1).collect();
    guion_estudio::run_con(r, &args)
}
```

`guion studio` on a post that uses `kart-tres` answers with the
`cargo run --manifest-path … --bin estudio -- <post>` to use. Each account
gadget passes `comprobar_contrato` in the account's own tests.

## 4. The timeline (AC4) and `escena.toml` (AC5)

```toml
# escena.toml — the curl of reel 47 as gadgets
version = 1
cola = 2.3                      # s after the voice; or, without voz.json, duracion = …

[[gadget]]
id = "brazo"
tipo = "palanca"
entra = "gancho"                # inicio | fin | <line id> | <line id>.fin, ± s
sale = "fin"

[gadget.params]
masa = { valor = 20.0, procedencia = "supuesto", nota = "la mancuerna que el reel declara" }
brazo = { valor = 0.32, procedencia = "reportado", fuente = "Winter: 0.20·H con H = 1.60 m" }
angulo = { valor = 0.0, procedencia = "supuesto" }
x = 300.0                       # layout: a bare value
y = 700.0

[[gadget.anim]]
param = "angulo"
de = 0.0
a = 90.0
desde = "sube"
hasta = "sube.fin"
curva = "suave"                 # lineal | suave | sale

[[gadget]]
id = "tau"
tipo = "medidor"
entra = "sube-0.3"

[gadget.params]
valor = { de = "brazo.tau" }    # bound to an earlier gadget's output
maximo = 70.0
marca = 62.784
rotulo = "τ en el codo"
```

- **Anchors** resolve against `Tiempos::de_voz(voz.json, duración)`:
  `inicio`, `fin`, each line's `<id>` (t0) and `<id>.fin` (t1). The offset
  is the last `+`/`−` whose prefix is a name (`beat-1+0.5` works). A bare
  number is refused; a missing name lists the names there are.
- **Length:** with `voz.json`, `round((voz + cola)·fps)/fps`; without,
  `duracion`. Both is an error.
- **Clips** fade in over `fundido` (0.3 s, ease-out) unless they start the
  reel, and out over 0.25 s before `sale` unless they last to the end.
  Every gadget is evaluated every frame (a binding always reads a value);
  only clips on screen draw.
- **Animations:** per param, in file order; the first governs from the
  clip's start (holding `de` until `desde`), each later one takes over at
  its `desde`.
- **Bindings** `{ de = "<id>.<salida>" }` read an earlier gadget only
  (order = evaluation order; no cycles, no solver).
- **The clock** (`Compilada::reloj` → guion-reel's `Clock`, written by
  `Reel::write_events`): `voz:<id>`, `<id>:entra`, `<id>:sale` (if before
  the end), `<id>:<evento>` at its first frame on screen, and a series
  `<id>.<salida>` per output over the clip's frames. The score reads them
  as it reads any scene's (R-0011).
- **Canonical text** (`Escena::a_toml`): a header, `version`, `cola`,
  `duracion`; per gadget `id`, `tipo`, `entra`, `sale`, `fundido` (if not
  0.3), params in the gadget's declared order, animations in file order.
  Floats in Rust's shortest round-trip form with a point; strings with
  TOML escapes. Comments are not kept: the header says the studio owns
  the file.

## 5. The 3D plan (phase 3)

Deterministic and CPU-only: the output of a 3D gadget is the same
`Prim2::Path` polygons every other gadget returns, rasterized by
motoreel's CPU sink. There is no GPU, no z-buffer and no floating-point
order that depends on threads.

- **Pose:** garust PGA motors (`Motor3`, rotor · translator) for every
  object and for the camera, exactly as motoreel's `Camera` already holds
  its pose; no quaternion or matrix code of our own.
- **Camera:** motoreel's `Projection` (`Pinhole { focal }`,
  `Orthographic`) plus an **oblique** projection (rf-marca `tres.rs`'s
  `Cam`: `KX`, `KY`, the 3/4 view where a plane x = const keeps its true
  shape), added upstream in motoreel. Camera params (azimut, elevación,
  distancia, focal) are ordinary params, so an orbit is an animation
  anchored to the voice and shows on the timeline's camera track.
- **Meshes:** low-poly, convex faces as vertex-index lists: built-ins
  `caja`, `prisma` (N-gon, the 12-sided tyre of the house), `cilindro`,
  `icosaedro` (level 0/1), `extrusion` of a polygon; account meshes as data
  (`[[malla]]` in a TOML file, vertices and faces) or Rust (Rotor's kart).
- **Shading:** Lambert `n·l` against the house light (135°, upper left, as
  `tres.rs`), **quantized** to an ordered ramp of palette tokens declared
  by the account (`[estudio] tonos = ["ash-dim", "ash", "steel-dim",
  "steel"]`, thresholds as `tres.rs`: 0.6, 0.25, −0.1). Flat faces, no
  gradients (Rotor style §4).
- **Visibility:** back-face culling, then painter's order by the face's
  mean view depth with a total tie-break (mesh index, face index); faces
  of intersecting meshes are split by a BSP of their planes (phase 3b).
- **Oracles:** `tres.rs`'s projected kart points (`Cam::q`) reproduced to
  1e-12; a cube under each projection shows the expected faces and tones;
  an orbit's frames are byte-identical across runs and thread counts.

## 6. The UI architecture (AC6)

```
browser ── http://127.0.0.1:N ──► guion studio <post>   (std::net, one request at a time)
  page: library · gadgets · inspector · viewport · scrub · timeline
     GET  /api/estado?t=T          → JSON: template, library, anchors, voice, gadgets (params with spec,
                                     value, binding, provenance, animated), values, events, avisos
     GET  /api/cuadro.png?t=T&r=K  → PNG: Estudio::cuadro_png → Compilada::cuadro(t) → guion-reel
                                     Reel::frame_rgb (motoreel's CPU sink) → ÷K → png (Fast)
     GET  /api/escena.toml         → the canonical text as «guardar» would write it
     POST /api/param {id,param,valor,procedencia?} → Estudio::fijar (checked by the spec, then the
                                     whole scene; refused → 400 naming it, nothing changes)
     POST /api/guardar             → writes escena.toml
```

- `responder(&mut Estudio, &Peticion) → Respuesta` is the whole router,
  tested without a socket; `servir` only moves bytes. Bound to 127.0.0.1.
- The page is one HTML file with vanilla JS, no CDN, embedded with
  `include_str!`; it builds its DOM with `createElement`/`textContent`.
- The template is the account's: `Plantilla::para(post)` walks up to the
  nearest `cuenta.toml` (canvas, tokens and roles, safe area, faces); a
  post outside an account gets the neutral template (`--cara` gives it a
  label face).
- Headless: `--cuadro T [--r K] [--out F]`, `--check`, and guion-reel's
  `--video | --sheet | --frames | --events` from the same scene.
- Phase 4 adds tracks (voice, score, camera) and the gate strip by
  calling the existing crates (guion-audio, `guion reel check`); the
  account switcher reads the dashboard's repo list (R-0013 AC8).

### Use

```bash
guion studio ../fisicobuenfisico/reels/curl-palanca            # http://127.0.0.1:7878/
guion studio posts/que-es-un-bit --puerto 0                    # any free port
guion studio post/ --cara /usr/share/fonts/truetype/dejavu/DejaVuSans.ttf --cuadro 2.0 --r 2 --out f.png
guion studio post/ --check                                     # exit 1 on any aviso
guion studio post/ --video                                     # <post>/<name>.mp4 + eventos.json
```

## 7. Determinism (AC7)

- `eval` is pure (checked for every registered gadget).
- `Compilada::cuadro(t)` is a pure function of the scene and `t`.
- `Reel::frame_rgb(t)` rasterizes with a fresh sink in its own scratch
  folder (pid + counter) and removes it; its pixels are the frame
  `frames_png` writes and the MP4 encodes.
- The PNG encoder runs with fixed settings (`Compression::Fast`, the
  default adaptive filter), so the same pixels give the same file.
- Saving is canonical; load → save → load is the identity.

## 8. Roadmap

| Phase | What | Done when |
|---|---|---|
| 1 | Contract, palanca, medidor, escena.toml, `guion studio` minimal | this change (AC1–AC7 green) |
| 2 | Library: gráfica viva, fórmula armada, tabla, polea/cable, círculo de fricción, título/subtítulo as gadgets, regla y compás; `guion reel` renders a gadget-only post; Q2 (mixed scenes), Q3 (external edits), a frame cache | each gadget passes `comprobar_contrato` with oracles from its source reel; a post with no motor passes the gate 18/18 |
| 3 | 3D (§5): camera + orbit, meshes, quantized Lambert, oblique projection upstream in motoreel | `tres.rs` reproduced; byte-identical orbit |
| 4 | Full studio: voice/score tracks, clips dragged to anchors, the gate strip live, account switcher, contact sheet | the prototype's every part is live |
| 5 | Migration: fbf curl/polea, Rotor 1.3 as gadgets; then remove `apps/guion-studio` (Q1) | the re-rendered reels pass the gate; the owner signs off |

## 9. Acceptance-criteria checklist

| AC | Test |
|----|------|
| AC1 | `guion-gadget` `r0015_gadget.rs` `ac1_every_built_in_keeps_the_contract`, `ac1_eval_is_pure_bit_for_bit`, `ac1_provenance_is_required_and_kept`, `ac1_types_and_ranges_fail_by_name` |
| AC2 | `ac2_palanca_reproduces_fbf_model_numbers` (62.784 N·m, 196.2 N, 0.32 m; equal to `mecanica::palanca::momento`; fbf's `τ(45°, 0)`), `ac2_palanca_places_its_label_where_nothing_is` (DejaVu; skipped without it), `ac2_medidor_fills_marks_and_warns`; unit `dibujo::tests::*` |
| AC3 | `ac3_accounts_add_gadgets_to_the_registry`; `guion-estudio` `r0015_estudio.rs` `ac6_the_account_sets_the_template_and_names_its_own_gadgets` (`[estudio]` in a scaffolded account) |
| AC4 | `ac4_clips_and_bindings_follow_the_clock`, `ac4_anchors_are_never_typed_times`, `ac4_the_voice_sets_the_length_and_the_anchors`; unit `linea::tests::*` |
| AC5 | `ac5_load_save_load_is_the_identity`, `ac5_a_change_from_the_studio_lands_in_the_file`, `ac5_the_example_in_the_spec_parses` (the block in §4) |
| AC6 | `r0015_estudio.rs` `ac6_the_state_lists_gadgets_params_and_provenance`, `ac6_params_round_trip_through_guardar`, `ac6_the_server_answers_over_localhost` (a real socket), `ac6_the_check_runs_every_frame`, `ac6_the_account_sets_the_template_and_names_its_own_gadgets`; unit `http::tests::decodifica_la_query` |
| AC7 | `ac6_a_frame_is_a_png_and_its_bytes_are_deterministic` (same studio, new studio, PNG = `png(frame_rgb)`); `guion-reel` `r0009_reel.rs` `ac3_frame_rgb_is_the_frame_without_ffmpeg` (= the PNG `frames_png` writes) |

## 10. Decision log

See R-0015 §7.

## Changelog

- 2026-10-20 — created with R-0015; phase 1 implemented.
