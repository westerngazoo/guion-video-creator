# SPEC-0013 — The accounts dashboard

- **Status:** In review
- **Realizes:** R-0013
- **Created:** 2026-10-18
- **Module(s):** `guion-tablero` (new crate), `guion-cli` (`guion tablero`, `guion reel nuevo`), `guion-encode` (`ffmpeg::duration`; the mix's cut)

## 1. Design

```
guion-tablero/src/
├── lib.rs        # the crate's map; re-exports run, run_nuevo, opciones, plan_lote
├── cuentas.rs    # guion-cuentas.toml (deny_unknown_fields), validation, segment glob (AC1)
├── post.rs       # Estado, the README's estado line and title, check.json, one Post (AC2)
├── tablero.rs    # Tablero::leer over workspaces, Conteo, durations in threads, JSON (AC3)
├── ui.rs         # App (state, keys → Efecto) and dibujar (any ratatui backend) (AC4)
├── tui.rs        # the terminal loop: 100 ms poll, job channel, background re-read
├── acciones.rs   # Accion → Comando (guion reel …), abrir, marcar_publicado (AC5–AC7)
├── trabajos.rs   # a queue, N worker threads, line-streamed stdout/stderr (AC5)
├── encargos.rs   # encargos.toml: load, append-only add (AC8)
├── nuevo.rs      # guion reel nuevo: copy, substitute, borrador, registro (AC10)
├── fecha.rs      # today (`date +%F`, else UTC by Hinnant's algorithm)
└── cli.rs        # guion tablero [--json] | lote …; guion reel nuevo (AC3, AC9, AC10)
```

### `guion-cuentas.toml`

```toml
[[cuenta]]
nombre = "Cuenta A"
posts = ["content/posts/*"]          # `*`, `?` inside a segment
perfil = "brand/reel-perfil.toml"    # → guion reel check --brand
publicar = ["publicar/nivel-*"]      # <match>/<post>/ holds the deliverables
plantilla = "plantillas/reel"        # guion reel nuevo copies it
nuevos = "content/posts"             # default: the first posts glob ending in /*

[[cuenta.registro]]                  # appended (once) on guion reel nuevo
archivo = "motor/Cargo.toml"
texto = "[[bin]]\nname = \"{{slug}}\"\npath = \"../content/posts/{{slug}}/escena.rs\"\n"
```

### Use

```bash
guion tablero [--workspace DIR...] [--paralelo N]
guion tablero --json [--workspace DIR...]
guion tablero lote <render|check|voz> [--cuenta X] [--estado Y] [--paralelo N] [--dry-run] [--workspace DIR...]
guion reel nuevo <cuenta> <slug> [--titulo "…"] [--workspace DIR...]
```

Without `--workspace`, the current folder. Keys: ↑↓/jk move, ←→/hl or Tab
switch pane, space mark, `a` mark all, `f` filter, `r` render, `c` check,
`v` voice, `o` open, `p` published, `e` encargo (+ optional slug: a new
post), `g` re-read, `q` quit. In a prompt: Enter next/accept, Esc cancel,
Tab cycles the account.

### Jobs

`trabajos::lanzar` puts the jobs in a queue and starts `min(N, jobs)`
threads; each pops a job, spawns it with piped stdout and stderr, streams
their lines (`\r` progress keeps its last state) and sends `Fin` with the
exit code. The TUI drains the channel every tick and re-reads the state in a
thread when a job ends; `lote` drains it until every job ended.

## 2. Acceptance-criteria checklist

| AC | Test (`crates/guion-tablero/tests/r0013_tablero.rs` unless noted) |
|----|------|
| AC1 | `ac1_the_accounts_file_is_the_repos_and_fails_by_name`; unit `cuentas::tests::*` |
| AC2 | `ac2_a_post_is_read_from_its_folder`; unit `post::tests::the_estado_word_ends_at_punctuation`; `guion-encode` unit `ffmpeg::tests::la_duracion_sale_del_encabezado` |
| AC3 | `ac3_the_whole_state_as_json` (snapshot `tests/snapshots/estado.json`) |
| AC4 | `ac4_the_screen_three_panes_a_log_and_the_keys` (TestBackend snapshot `tests/snapshots/tablero.txt`) |
| AC5 | `ac5_actions_are_the_existing_commands` (command lines, legacy skip, a real job's lines and exit code) |
| AC6 | `ac6_open_the_sheet_or_say_where_it_is` |
| AC7 | `ac7_mark_publicado_rewrites_the_estado_line` |
| AC8 | `ac8_an_encargo_is_appended` |
| AC9 | `ac9_lote_runs_an_action_over_a_filtered_set` |
| AC10 | `ac10_a_new_post_from_the_accounts_template`; out of tree: the Rotor template, made with `guion reel nuevo` and rendered with `guion reel`, passed the gate 18/18 (rotorf-sico, 2026-10-18); the mix fix: `guion-encode` `ac6_la_mezcla_conserva_todos_los_cuadros` |

## 3. Decision log

See R-0013 §6.

## Changelog

- 2026-10-18 — created with R-0013.
