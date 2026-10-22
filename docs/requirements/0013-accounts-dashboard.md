# R-0013 — The accounts dashboard: every post of every account, and the actions on them

- **Status:** In review (implemented locally, awaiting the owner's sign-off)
- **Milestone:** M2
- **Owner:** físico buen físico (see project-specifics.md)
- **Created:** 2026-10-18
- **Depends on:** R-0005 (ffmpeg discovery; the mix), R-0012 (`guion reel`, its check, its voice, `check.json`, `reel.toml`)
- **Realized by:** SPEC-0013
- **QA:** `qa` agent run scoped to this requirement

## 1. Statement

The owner of several Instagram accounts (today three, in two repos) must see,
in one terminal screen, every post of every account with its estado, its last
gate result, its duration and whether it is narrated, and must be able to run
the existing commands on one post or on a batch without an agent: render,
check, voice, open the contact sheet, mark it published, hand the account a
one-line idea (an *encargo*), and start a new post from the account's
template. The same state and the same batch actions must be available to
scripts and agents without the screen. Nothing about a brand lives in this
repo: each repo declares its accounts in a small file.

## 2. Rationale

The posts live in folders across repos, each with a README whose `estado:`
line is the source of truth, and since R-0012 a `reel.toml` and a
`check.json`. Knowing what is ready, what failed its gate and what is still
legacy meant opening dozens of READMEs; running the gate over a set meant a
shell loop. The owner asked (2026-10-18) for a dashboard, generic, with
batch actions; and, the same day, for the template of a new reel to be "as
automatic as possible": one command that copies the account's template and
registers the scene.

## 3. Acceptance criteria

- **AC1. The accounts file.** A repo declares its accounts in
  `guion-cuentas.toml` at its root: name, post globs, gate profile,
  deliverable folders, template, where new posts go, and snippets to append
  on a new post. Unknown keys fail by name; names are unique
  (case-insensitive); paths are relative. A workspace without the file is an
  error naming it. The crate holds no brand value.
- **AC2. A post.** A folder matched by a glob that holds `README.md`. From
  it: the title (first `# `), the `estado:` line (first one, list marker and
  emphasis allowed; the first word, accent-insensitive: idea, borrador,
  revisión, listo, publicado; else none), `reel.toml` (else «legado»),
  `voz.toml` (narrated), `check.json` (pass, totals, one line per check),
  the MP4 (`<name>-audio.mp4` with `reel.toml`'s name, else the only
  `*-audio.mp4`, else `<name>.mp4`) and its duration through guion-encode's
  ffmpeg, `cuadros/hoja.png`, and the deliverables (`reel.toml`'s
  `[publish] dir`, else `<publicar glob>/<post>`).
- **AC3. The state as JSON.** `guion tablero --json` prints every account
  (repo, profile, template, counts per estado plus «sin estado» and
  «legado», posts) and every encargo, in a fixed order.
- **AC4. The screen.** Accounts (with counts) on the left, posts in the
  middle (estado in colour, check PASS/FAIL n/—, duration, narrated, type),
  the post's detail on the right (title, estado and note, check lines,
  paths, the account's pending encargos), a log pane and a key line. A
  filter by estado cycles; space marks posts; `a` marks all visible.
- **AC5. Actions as subprocesses.** `r` runs `guion reel <post>`, `c`
  `guion reel check <post> --brand <profile>`, `v` `guion reel voice
  <post>`, on the marked posts (else the cursor's), a few at a time, in the
  background, their output streamed to the log; the screen never waits on
  them and re-reads the state when one ends. A legacy post is skipped for
  render and voice, saying why.
- **AC6. Open.** `o` opens the contact sheet (else the MP4) with `open`
  (macOS) or `xdg-open` (with a display); headless, it prints the path.
- **AC7. Published.** `p` asks a date (today by default) and a link and
  rewrites the README's `estado:` line as `publicado (date, link)`, keeping
  what it said (`Antes: …`). A bad date or a README without the line is an
  error.
- **AC8. Encargo.** `e` asks an account and a one-line idea and appends
  `[[encargo]]` (id `E-NNNN`, fecha, cuenta, idea, estado = pendiente, and
  `post` if one was made) to that repo's `encargos.toml`, never rewriting
  what is there.
- **AC9. Batch for scripts.** `guion tablero lote <render|check|voz>
  [--cuenta X] [--estado Y] [--paralelo N] [--dry-run]` runs an action over
  the filtered set; exit 0 when all pass, 1 when one fails, 2 on a usage or
  input error.
- **AC10. A new post from a template.** `guion reel nuevo <cuenta> <slug>
  [--titulo "…"]` copies the account's template folder into a new post
  folder, replacing `{{slug}}`, `{{titulo}}`, `{{fecha}}` and `{{cuenta}}`
  in file names and UTF-8 contents (keeping file modes), sets the README's
  estado to `borrador`, appends the account's `registro` snippets, and the
  dashboard lists the post at once. The `e` prompt takes an optional slug
  and does both. An existing folder, a bad slug or an account without a
  template is an error.
- **AC11. An encargo with details.** Besides the idea, an encargo may carry
  `formato` (reel, short, youtube), `prioridad` (alta, normal, baja), `para`
  (`YYYY-MM-DD`), `nivel`, `variantes` (2 to 4 short lines: what the reel
  compares), `remake` (the old post it remakes) and `notas`. They are
  trimmed and checked before anything is written; an empty one is no field;
  an encargo without them writes none of their keys. `guion encargo nuevo`
  takes them as flags.
- **AC12. An encargo follows its post.** `guion encargo tomar <id> <post>`
  (and `guion reel nuevo … --encargo <id>`) sets `estado = "en curso"` and
  links the post, which must be one of the encargo's account; `guion
  encargo marcar <id> <estado>` sets pendiente, en curso, hecho or
  descartado. Both rewrite only those two lines of that block: comments,
  other keys and other blocks stay. What the dashboard shows (`vivo` in the
  JSON) is the linked post's estado when the account has it (`listo` and
  `publicado` as they are, anything before them `en curso`), `descartado`
  when discarded, else the stored estado; with the post's deliverables'
  folder (`paquete`). No one writes the encargo again when the post moves on.
- **AC13. An account's picture and colour.** `[[cuenta]]` may declare
  `avatar` (a png, jpg, webp or svg, relative to the repo, never outside
  it) and `color` (`#RRGGBB`), so a dashboard can show which account is
  which at a glance. A bad one is an error naming the account; a declared
  avatar that is not there yet is simply left out of the state.

## 4. Constraints & non-goals

- New crates in the lock: `ratatui` 0.30 (no default features, its
  `crossterm` backend, re-exported as `ratatui::crossterm`), by the owner's
  request. No date, glob or temp-dir crate.
- Not here: publishing to Instagram (OQ-4 of the brand repos); the agents'
  reviews; editing posts beyond the `estado:` line.

## 5. Open questions

- Q1. Should the dashboard watch the folders and refresh on its own? Today
  it re-reads when a job ends and on `g`.
- Q2. ~~Should encargos move through their estados from the dashboard?~~
  Closed 2026-10-22 by AC12: the agent takes an encargo once, and from then
  on it follows its post's README.

## 6. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-10-18 | A new crate, `guion-tablero`; `guion tablero` and `guion reel nuevo` dispatch to it from guion-cli | The dashboard composes no guion engine: it reads folders and runs `guion` itself as a subprocess (`GUION_BIN`, else this binary), so the actions are exactly the commands the owner runs by hand |
| 2026-10-18 | The estado word is the first run of letters of the line's value | Real READMEs write `listo (2026-10-16): …`, `revisión — falta …`, `Publicado.` |
| 2026-10-18 | `publicado` keeps the old value after `Antes:` | The READMEs carry their history in that line; a rewrite must not lose it |
| 2026-10-18 | Templates and the scene's registration are the brand's (`plantilla`, `registro`) | The scene binary lives in each brand's motor manifest; a generic append with placeholders registers it without this repo knowing the motor |
| 2026-10-22 | An encargo's shown estado is derived from its post, not stored | One fact in one place: the post's README already says where it is; storing it again in the encargo is what left every encargo «pendiente» |
| 2026-10-18 | `mux_at_loudness` cuts with `-t <video duration>` instead of `-shortest` | The gate (R-0012 AC5) caught a template reel muxed with 213 of its 217 frames: with `-c:v copy`, `-shortest` dropped the video's tail; `-t` kept all 217 on the same inputs |

## Changelog

- 2026-10-22 — AC13 (an account's picture and colour), for the owner's accounts UI.
- 2026-10-22 — AC11 (an encargo's details) and AC12 (it follows its post); Q2 closed. The owner asked for more options when requesting a reel, and the encargos never left «pendiente» (10 of 10 in the first repo that used them, one of them already delivered).
- 2026-10-18 — created; realized by SPEC-0013 in the same change.
