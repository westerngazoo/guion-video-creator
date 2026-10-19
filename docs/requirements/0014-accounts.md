# R-0014 — Accounts: the account contract, `guion cuenta nueva`, and the dashboard across repos

- **Status:** In review (implemented locally, awaiting the owner's sign-off)
- **Milestone:** M2
- **Owner:** físico buen físico (see project-specifics.md)
- **Created:** 2026-10-18
- **Depends on:** R-0009 (the studio), R-0011 (the score), R-0012 (the gate and its brand profile, `guion reel`), R-0013 (`guion-cuentas.toml`, the dashboard, `guion reel nuevo`)
- **Realized by:** SPEC-0014
- **QA:** `qa` agent run scoped to this requirement

## 1. Statement

The house runs three Instagram accounts (Rotor Físico, Spanish, racing
physics; PMMO, English, mathematics in a Greek design, inside the Rotor repo;
@fisicobuenfisico, Spanish, gym physics, in its own repo) and is about to
start a fourth, «El extraño arte de computar» (technology, AI, computing).
The reel process must serve every account, and starting one must be one
command. So:

- an **account contract** says, in one file, everything an account provides,
  and nothing the shared layer provides;
- **`guion cuenta nueva`** scaffolds a whole account from generic templates
  that live in this repo, deterministically, never overwriting;
- the **process** of a reel is one generic document the accounts link to;
- **`guion tablero`** reads several repos at once, so every account shows on
  one screen.

### The contract

| The account provides | Where |
|---|---|
| Identity: name, slug, handle, language, subject, audience, owner | `cuenta.toml` `[cuenta]` |
| Voice: Piper model, tone, form of address, forbidden words | `cuenta.toml` `[voz]` |
| Brand tokens: canvas, palette tokens and their roles (background, text, secondary, accent, focus), faces, title and subtitle box, safe area | `cuenta.toml` `[marca]` |
| The gate's limits that are not drawing (motion, audio, caption) | `cuenta.toml` `[compuerta]` → `brand/reel-perfil.toml` (derived) |
| A scene template | `plantillas/reel/` (scaffolded neutral; the account grows it) |
| A domain reviewer: its name, subject and rules | `cuenta.toml` `[revisor]` → `.claude/agents/<nombre>.md` |
| Its decisions log, status file and deliverables | `docs/decisions.md`, `estado.json`, `publicar/<slug>/` |
| Its non-negotiable rules and its manual | `CLAUDE.md` |

| The shared layer provides (this repo) | Crate / file |
|---|---|
| The pipeline: voice → scene → sound → mix → caption → gate | `guion reel` (guion-cli, R-0012) |
| The gate, against the account's profile | `guion reel check` (guion-reel, R-0012) |
| Voice and score | guion-audio (R-0007, R-0008, R-0011) |
| The studio and the neutral primitives (title, subtitle, accent bar, closing card), read from the contract | guion-reel (R-0009), guion-marca (this requirement) |
| Neutral instruments for an account with none | guion-marca `sonido`, `guion reel sonido` |
| The dashboard and new posts from a template | guion-tablero (R-0013) |
| The scaffold, the contract and the derived profile | guion-cuenta (this requirement) |
| The generic process | `docs/proceso-reel.md` |
| Generic reviewer prompts, parameterised by domain | `templates/cuenta/agentes/` |

## 2. Rationale

Everything that measures a reel is already generic (R-0009, R-0011, R-0012,
R-0013). What still made each account by hand was the brand code (Rotor's
`rf-marca`, PMMO's `hellas.rs`), the agents (`fisico`, `designer`,
`helenista`), the process document and its skill, the layout of `publicar/`,
and the decisions and status files. A new account copied them from the
nearest one and edited them, which is slow and drifts. The owner asked
(2026-10-18) that the process be abstract enough to serve every account and
that creating an account be easy, before starting the fourth.

## 3. Acceptance criteria

- **AC1. The contract.** `cuenta.toml` holds `[cuenta]`, `[voz]`, `[marca]`
  (with `lienzo`, `tokens`, `roles`, `fuentes`, `titulo`, `subtitulo`,
  `seguro`, and an optional `rejilla`), `[compuerta]` (defaults: the platform's) and `[revisor]`.
  Unknown keys fail by name; every role names a token; the language is `es`
  or `en`; the slug and the reviewer's name are slugs; dates are
  `AAAA-MM-DD`. The crate holds no brand value.
- **AC2. The derived profile.** The gate profile (R-0012 AC1) is a pure
  function of the contract, written to `brand/reel-perfil.toml`; `guion
  cuenta perfil <dir>` derives it again and `--check` exits 1 when the file
  and the contract disagree. The voice's forbidden words join the caption's
  internal list.
- **AC3. The scaffold.** `guion cuenta nueva <dir>` makes, from flags or an
  interactive prompt (only what is missing, or everything with
  `--interactivo`; without a terminal a missing answer is an error naming
  its flag): `cuenta.toml`; `brand/reel-perfil.toml`; a reel template that
  compiles against guion-reel through guion-marca, with neutral primitives
  (`plantillas/reel/`, a `motor/` crate whose scenes `guion reel nuevo`
  registers); `docs/decisions.md` with its first row; `estado.json`;
  `publicar/<slug>/perfil/`; `.claude/agents/<revisor>.md` and a designer,
  both rendered from the generic templates; a skill; a `CLAUDE.md` holding
  the non-negotiable rules (nothing is published without the owner, every
  number has provenance, privacy and consent, the subject is reviewed before
  design); and the account's `guion-cuentas.toml` entry. The same answers
  give the same bytes (the date is an answer: `--fecha`, today by default).
- **AC4. Never overwrite.** If any file the scaffold would write exists, it
  refuses, names them all, and writes nothing.
- **AC5. Inside an existing repo.** With `--repo ROOT`, the account goes in
  a folder of that repo, its entry is appended to the repo's
  `guion-cuentas.toml` (never rewritten; an account of the same name is
  refused), and what Claude Code and the house look for at the root — the
  agents (the designer as `designer-<slug>`), the skill and `publicar/` —
  goes at the root.
- **AC6. Neutral primitives from the contract.** guion-marca's `Marca` loads
  `cuenta.toml` and its faces and gives a scene: colours by token and role,
  text sized in em as the gate measures it, the optional grid of
  `[marca.rejilla]` (a PCB, a ruled page: drawn in a background token, so
  the gate counts it as empty), the title (shrunk to the safe
  area), the subtitle of the line being said, an accent bar, the closing
  card (still after 0.6 s), the voice's clock, the studio with the house
  layout (`eventos.json`, `cuadros/hoja.png`), and the gate's own subtitle
  checks.
- **AC7. The template passes the gate.** A post made from a scaffolded
  account's template, with a two-line voice, builds, renders twice to the
  same bytes, takes its bed from the neutral kit (`guion reel sonido`, also
  `{guion}` in `reel.toml`'s sound command), mixes to −14 LUFS and passes
  all 18 checks of `guion reel check` with the derived profile.
- **AC8. The dashboard across repos.** `guion tablero` (and `lote`, and
  `guion reel nuevo`) take `--repo DIR`, repeatable, besides `--workspace
  DIR...`; `--todas` adds the user's list (`$GUION_REPOS`, else
  `$XDG_CONFIG_HOME/guion/repos.toml`, else `~/.config/guion/repos.toml`;
  `[[repo]] ruta = "…"`, `~/` and relative paths allowed); with no repo
  named, the current folder if it declares accounts, else the user's list.
  The same repo named twice is read once. `guion cuenta nueva --al-tablero`
  appends the new repo to the list, never rewriting it.
- **AC9. One process.** `docs/proceso-reel.md` states the fast, cheap flow
  for every account: the automatic gate first, one parallel review round,
  cheap-model confirmations, commit only your own paths. Scaffolded accounts
  link to it instead of copying it.

## 4. Constraints & non-goals

- No new crate in the lock: guion-cuenta is serde + toml; guion-marca uses
  ab_glyph 0.2, already there through motoreel-typeset.
- Not here: a design direction, a voice, or a logo for a new account. They
  are decided by hand with the owner and recorded in the account's
  `decisions.md`; the scaffold writes them as «por decidir». The faces are
  not shipped (licences): the account drops them in `brand/fonts/`.
- Not here: migrating Rotor, PMMO or fbf. Their brand Rust stays until the
  data-driven brand can express it (§5).
- Not here: publishing (OQ-4 of the brand repos).

## 5. Phase 2: the data-driven brand (plan)

Phase 1 (this requirement) makes a new account need no brand Rust: tokens,
faces, sizes and boxes are data, and the primitives that read them are
neutral. Phase 2 makes the existing brands expressible as data, so their
Rust can go:

1. **Logo as data.** `[marca.logo]` holds the mark as vector paths (SVG path
   data or polygon lists, low poly or not), with its watermark variant and
   size; `Marca::logo(x, y, alto)` draws it as `Prim2::Path` (no raster), and
   the profile's `safe.except` is derived from where the watermark goes.
   Rotor's R F R̃ disc and PMMO's medallion become data files.
2. **Components as data.** The level chip, a formula band (`[formula]` of the
   gate, with the mono face), a facet style (N-gon count, the 135° light),
   card layouts: each a parameterised neutral primitive.
3. **Instruments as data.** Presets built on guion-audio's synth (drone,
   struck tone, noise burst, gallop, taiko, riser), parameterised in
   `[[marca.instrumento]]`, so `sonido.toml` names the account's instruments
   without a motor binary.
4. **Migration.** fbf first (it already adopts Rotor's design: it points at
   Rotor's contract, no Rust); then Rotor (`rf-marca` → contract + logo
   file); PMMO last (its incised Greek letters and papyrus need the
   letters-as-outlines of motoreel R-0014). Each brand repo runs `guion
   cuenta perfil --check` in its CI.
5. **Agents from the contract.** The domain reviewer and the designer are
   re-rendered from the generic templates when the contract changes
   (`guion cuenta agentes --check`), so the house's prompt improvements reach
   every account.

## 6. Open questions

- Q1. Should a scaffolded account start as its own repo or inside
  rotorf-sico (as PMMO did)? Both work (`--repo`); recommend its own repo,
  sibling of the others, so its history and its permissions are its own.
- Q2. Should `guion cuenta nueva` create the GitHub repo and the first
  commit? Recommend no: the owner creates the repo; the scaffold stays local
  and deterministic.

## 7. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-10-18 | Two new crates: guion-cuenta (the contract, the profile, the scaffold; no engine) and guion-marca (the neutral primitives and instruments; the engine coupling) | CLAUDE.md §2: the coupling to motoreel lives in one crate; the contract stays readable without an engine |
| 2026-10-18 | The gate profile is derived from the contract, never written by hand in a new account | One source for the tokens: the scene and the gate read the same values |
| 2026-10-18 | A scaffolded scene depends on guion-marca alone (which re-exports guion-reel and motoreel), by path to the sibling guion-video-creator | The house's repos are siblings already; one dependency keeps the account's manifest trivial |
| 2026-10-18 | Templates use `{%clave%}`; `{{…}}` is left for `guion reel nuevo` | The account's template is itself a template for posts |
| 2026-10-18 | The neutral kit is always compiled in guion-marca (guion-audio with `score` only) | A feature would make `cargo test -p guion-marca` differ from the workspace build; the score adds no heavy crate |
| 2026-10-18 | `[sound] command` may say `{guion}`: the binary running the pipeline | A new account's sound is `guion reel sonido` without depending on `guion` being in PATH |
| 2026-10-19 | `[marca.rejilla]`: an optional background grid, data in the contract; its token must be a `fondo` role | The fourth account's direction (terminal / circuit, a PCB grid) is the first real case; a grid is generic (Rotor's retícula, a ruled page), so it is data and a neutral primitive, not account Rust |
| 2026-10-18 | The user's repo list is append-only TOML (`[[repo]]`) | `--al-tablero` appends without rewriting a file the owner may have commented |

## Changelog

- 2026-10-18 — created; realized by SPEC-0014 in the same change.
- 2026-10-19 — `[marca.rejilla]` and `Marca::rejilla`; the first real account («El extraño arte de computar», terminal / circuit) scaffolded locally with it: its template reel passes the gate 18/18.
