# SPEC-0014 — Accounts

- **Status:** In review
- **Realizes:** R-0014
- **Created:** 2026-10-18
- **Module(s):** `guion-cuenta` (new crate), `guion-marca` (new crate), `guion-tablero` (`repos`, `--repo`, `--todas`), `guion-cli` (`guion cuenta`, `guion reel sonido`, `{guion}`), `templates/cuenta/`, `docs/proceso-reel.md`

## 1. Design

```
crates/guion-cuenta/src/          # no engine: serde + toml
├── lib.rs        # the crate's map
├── contrato.rs   # cuenta.toml: Cuenta and its sections, validation (AC1)
├── perfil.rs     # Cuenta → brand/reel-perfil.toml text (AC2)
├── texto.rs      # TOML/JSON strings by hand, {%clave%}, a_slug
├── plantillas.rs # templates/cuenta/* compiled in (include_str!)
├── nueva.rs      # Respuestas → Plan (every file as text) → crear (AC3–AC5)
└── cli.rs        # guion cuenta nueva | perfil; flags and the prompt

crates/guion-marca/src/           # the engine coupling
├── lib.rs        # re-exports guion-reel (as `reel`), motoreel, Marca
├── marca.rs      # Marca: tokens, faces in em, titulo, subtitulo, barra, cierre, reloj, reel (AC6)
└── sonido.rs     # the neutral kit (tono, pulso, roce) and render(post) (AC7)

crates/guion-tablero/src/repos.rs # the user's list: archivo, leer, agregar, elegir (AC8)

templates/cuenta/                 # the generic templates (AC3, AC9)
├── CLAUDE.md  gitignore
├── agentes/{revisor,designer}.md  skill/SKILL.md
├── docs/{decisions,fuentes,posts,publicar-perfil}.md
├── motor/{Cargo.toml,src/lib.rs}
└── plantilla/{README.md,caption.md,escena.rs,reel.toml,sonido.toml,voz.toml}
```

### A scaffolded account

```
<dir>/                              (with --repo: <root>/<folder>/)
├── cuenta.toml                     the contract
├── CLAUDE.md                       the rules (§2) and the manual; links docs/proceso-reel.md
├── brand/reel-perfil.toml          derived (guion cuenta perfil)
├── brand/fonts/README.md           the faces are the account's
├── docs/decisions.md               first row: the account is created
├── estado.json                     empty piezas
├── guion-cuentas.toml              its entry (with --repo: appended to the root's)
├── motor/Cargo.toml, src/lib.rs    the scenes' crate: depends on guion-marca only
├── plantillas/reel/                README, caption, escena.rs, reel.toml, sonido.toml, voz.toml
├── posts/README.md
├── .gitignore
├── .claude/agents/<revisor>.md     (root) generic reviewer + the account's rules
├── .claude/agents/designer.md      (root; designer-<slug>.md with --repo)
├── .claude/skills/reel-<slug>/     (root)
└── publicar/<slug>/perfil/         (root)
```

### Use

```bash
guion cuenta nueva ../extrano-arte \
  --nombre "El extraño arte de computar" --slug extrano-arte \
  --tema "Tecnología, IA y computación, desde cero" \
  --revisor computologo --materia "computación, IA y tecnología" \
  --regla "Un algoritmo se describe por lo que hace, no por la marca que lo vende." \
  [--idioma es] [--handle @…] [--paleta acento=#e0322a] [--fuente-texto F] \
  [--fecha 2026-10-18] [--guion ../guion-video-creator] [--repo ROOT] [--interactivo] [--al-tablero]
guion cuenta perfil ../extrano-arte [--check]
guion reel nuevo "El extraño arte de computar" que-es-un-bit --titulo "¿Qué es un bit?" --repo ../extrano-arte
guion reel ../extrano-arte/posts/que-es-un-bit --twice
guion tablero --todas            # or: --repo ../rotorf-sico --repo ../fisicobuenfisico --repo ../extrano-arte
```

`--guion` is guion-video-creator as seen from the account's root (default:
the sibling of the repo the account lives in); the motor's manifest points
at `<guion>/crates/guion-marca`.

### Determinism

`plan` is a pure function of the answers and the account's place in its
repo: no clock (the date is `--fecha`), no environment, files in path order,
TOML and JSON written by hand. `crear` checks every path before writing any.

## 2. Acceptance-criteria checklist

| AC | Test |
|----|------|
| AC1 | `guion-cuenta` `r0014_cuenta.rs` `ac1_the_contract_parses_and_fails_by_name`; unit `contrato::tests::slugs_and_dates` |
| AC2 | `ac2_the_gate_profile_is_derived_from_the_contract` (parsed by guion-reel's `Profile`); `ac2_perfil_check_catches_a_drift` |
| AC3 | `ac3_a_new_account_has_everything_the_contract_asks` (every file, no `{%` hole, the rules, the agents, the first decision, JSON status, the dashboard reads it, `guion reel nuevo` registers a scene); `ac3_same_answers_same_bytes`; `ac3_flags_and_the_prompt_give_the_answers`; unit `texto::tests::*`, `nueva::tests::*` |
| AC4 | `ac4_refuses_to_overwrite_and_writes_nothing` |
| AC5 | `ac5_inside_a_repo_the_entry_is_appended_and_shared_paths_go_to_the_root` |
| AC6 | `guion-marca` `r0014_plantilla.rs` `ac6_the_scene_checks_its_subtitles_with_the_gates_rules` |
| AC7 | `ac7_the_template_reel_builds_renders_and_passes_the_gate` (cargo builds the scaffolded motor in release into `target/cuenta-e2e`; two renders compared; the neutral kit; the mix; 18/18 checks). Out of tree, the same with `guion reel <post> --twice`: PASS 18/18 (2026-10-18) |
| AC8 | `guion-tablero` `r0014_repos.rs` `ac8_*` (three tests); unit `repos::tests::*`; out of tree: `guion tablero --repo rotorf-sico --repo fisicobuenfisico --json` lists Rotor Físico, PMMO and fisicobuenfisico |
| AC9 | `docs/proceso-reel.md`; the scaffolded `CLAUDE.md` and skill link it (checked in AC3) |

### What the end-to-end test stands in for

- **The voice.** Piper and a voice model are the account's (`GUION_VOCES`);
  the test writes `voz.json` (the template's two lines) and a `voz.wav` with
  a tone per line. Everything downstream (clock, subtitles, bed ducking,
  gap check) runs on them.
- **The faces.** Not shipped; DejaVu Sans (Bold for the title) stands in.
  Without DejaVu or without ffmpeg/libx264 both guion-marca tests are
  skipped, saying so.
- **The caption export** (`caption.txt`) is the pipeline's; the gate reads
  `caption.md` itself.

## 3. Decision log

See R-0014 §7.

## Changelog

- 2026-10-18 — created with R-0014.
