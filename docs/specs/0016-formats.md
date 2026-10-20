# SPEC-0016 — Formats

- **Status:** In review
- **Realizes:** R-0016
- **Created:** 2026-10-20
- **Module(s):** `guion-reel` (`formato`, `youtube`, `Canvas::HORIZONTAL_VIDEO`, `Command::Miniatura`, `Reel::miniatura`, `gate::profile` `[formato.*]`, `gate::youtube`), `guion-publish` (`youtube`), `guion-gadget` (`Escena::formato`, `Tiempos::con_eventos`), `guion-estudio` (`Plantilla::para_formato`, `--miniatura`), `guion-cli` (`--formato`, `[youtube]`, the YouTube steps)

## 1. Design

```
guion-reel/src/
├── formato.rs        # Formato {Reel, Tiktok, Short, Youtube}: de, desde_entorno (GUION_FORMATO), canvas, bandas (defaults), duracion_maxima
├── youtube.rs        # cues/srt/parse_srt (voz.json → .srt), resample (area, deterministic), png_size, check_thumbnail
├── canvas.rs         # + Canvas::HORIZONTAL_VIDEO (1920 × 1080, 30 fps, 600 px/unit)
├── studio.rs         # + Command::Miniatura(t) (--miniatura T), Reel::miniatura → <dir>/miniatura.png
└── gate/
    ├── profile.rs    # + [formato.<name>] (FormatoSpec), Profile::para(F), Profile::max_duration(F)
    ├── youtube.rs    # srt, chapters, texts (yt-title, yt-description), thumbnail findings
    └── mod.rs        # Inputs.formato + the YouTube paths; run checks with p.para(formato)
guion-publish/src/youtube.rs   # youtube.md → Fuente; export → titulo.txt + descripcion.txt; chapter rules
guion-gadget/src/escena.rs     # + formato (written back only when set)
guion-gadget/src/linea.rs      # + Tiempos::con_eventos (chapters on beats)
guion-estudio/src/plantilla.rs # + Plantilla::para_formato (canvas and safe area of the format)
guion-cli/src/reel/youtube.rs  # formato (flag > reel.toml > escena.toml), textos, subtitulos
```

### The post

```toml
# reel.toml
[reel]
formato = "youtube"            # reel (default) | tiktok | short | youtube
max_duration = 900.0

[scene]
miniatura_flag = "--miniatura" # default

[youtube]
texto = "youtube.md"           # default
miniatura = 12.0               # the thumbnail's frame, s
```

```toml
# escena.toml (gadget posts)
version = 1
formato = "youtube"
```

```markdown
<!-- youtube.md -->
## Título
¿Por qué el kart levanta una rueda?
## Descripción
…
## Capítulos
- inicio | La pregunta
- modelo | El modelo
- beat-3 | En la pista
## Procedencia
Las cifras son calculadas con el modelo, no medidas.
```

### The brand profile

```toml
[formato.youtube]                 # every key optional
canvas = { width = 1920, height = 1080, fps = 30.0 }
subtitle = { band = [940.0, 1025.0], baseline = 980.0, center_x = 960.0, max_width = 1400.0 }
safe = { x = [96.0, 1824.0], y = [54.0, 1025.0], free = [], except = [] }
formula_band = [600.0, 800.0]
max_duration = 900.0
```

`Profile::para(F)`: the top level when F has the orientation of
`[canvas]`; otherwise the defaults below; then `[formato.<F>]`.

| Defaults (house, *supuesto*) | 9:16 | 16:9 |
|---|---|---|
| canvas | 1080 × 1920 | 1920 × 1080 |
| subtitle band / baseline / centre / widest | 1460–1545 / 1500 / 540 / 840 | 940–1025 / 980 / 960 / 1400 |
| safe x / y | 90–990 / 250–1545 | 96–1824 / 54–1025 |
| formula band | none | none |

### Use

```bash
guion reel <post> [--twice] [--no-check] [--formato reel|tiktok|short|youtube]
guion reel check <post> [--formato F] [...]
<scene-bin> --miniatura 12.0            # any scene on guion-reel's Command
guion studio <post> --miniatura 12.0
```

The pipeline sets `GUION_FORMATO` for the scene and the sound; exports the
caption when the format has one (reel, TikTok) or `caption.md` exists; for
`short` and `youtube`, `titulo.txt` + `descripcion.txt` from `youtube.md`
(anchors resolved with `Tiempos::de_voz(voz.json).con_eventos(eventos.json)`);
for `youtube`, `<name>.srt` and `miniatura.png` (`<bin> --miniatura t`), all
in `[publish] dir` or the post; then the gate with the format.

### What the gate checks, by format

| Check | reel | tiktok | short | youtube |
|---|---|---|---|---|
| R-0012's 18 (subtitles, colour, layout, motion, duration, audio, determinism) | ✓ | ✓ | ✓ | ✓ (bands of 16:9) |
| `caption`, `units`, `internal` | ✓ | ✓ | if `caption.md` | if `caption.md` |
| `duration` ≤ platform | — | — | 180 s | — |
| `yt-title`, `yt-description` | | | ✓ | ✓ |
| `srt`, `chapters`, `thumbnail` | | | | ✓ |

## 2. Platform limits: sources and verification

Every limit is *reportado*. Verified online on 2026-10-20. This session's
network policy refused direct connections to `support.google.com` and
`developers.google.com` (proxy: `connect_rejected`), so each was verified
through web-search results that quote the official page, not by opening the
page itself. Re-open each page before relying on a limit that matters.

| Limit | Value used | Source | Verification |
|---|---|---|---|
| Chapters: first at 00:00, ≥ 3 ascending, each ≥ 10 s | `MIN_CHAPTERS = 3`, `MIN_CHAPTER_S = 10` | YouTube Help, «Video chapters», <https://support.google.com/youtube/answer/9884579> («Make sure that the first timestamp you list starts with 00:00. Your video should have at least three timestamps listed in ascending order. The minimum length for video chapters is 10 seconds.») | search result quoting the page; consistent secondary: Dickinson College media center, 2026-01-12 |
| Thumbnail 1280 × 720 (min width 640), under 2 MB, JPG/GIF/PNG, 16:9 | `THUMB_W/H = 1280/720`, `THUMB_MAX_BYTES = 2 000 000`, PNG | YouTube Help, «Add video thumbnails», <https://support.google.com/youtube/answer/72431> («Have a resolution of 1280x720 (with minimum width of 640 pixels)… Remain under 2MB for videos or 10MB for podcasts») | search result quoting the page |
| Shorts up to 3 minutes (uploads on or after 2024-10-15, square or vertical) | `SHORT_MAX_S = 180` | YouTube Help, «Understand three-minute YouTube Shorts», <https://support.google.com/youtube/answer/15424877> | search result quoting the page |
| Title ≤ 100 characters, no `<` `>` | `MAX_TITLE_CHARS = 100`, `FORBIDDEN` | YouTube Data API, «Videos» resource, `snippet.title`, <https://developers.google.com/youtube/v3/docs/videos> | search result quoting the reference (zh-CN locale page); the Help Center page on titles was **not** found |
| Description ≤ 5 000 (characters in the Help Center, **bytes** in the Data API), no `<` `>` | `MAX_DESCRIPTION = 5000`, checked as characters **and** UTF-8 bytes | same reference, `snippet.description` | search result quoting the reference; the Help Center wording («5,000 characters») only through secondary sources — **not verified** on the Help Center itself |

Not verified, not checked: TikTok's and Instagram's own length limits (the
post's `max_duration` governs a reel and a TikTok), the 15-minute limit for
unverified YouTube accounts, chapter timestamps past one hour (R-0016 Q3).

## 3. Acceptance-criteria checklist

| AC | Test |
|----|------|
| AC1 | `guion-reel tests/r0016_formats.rs::ac1_the_format_sets_the_canvas`; unit `formato::tests::*`; `guion-estudio tests/r0016_formato.rs::ac1_escena_formato_sets_the_studio_canvas_and_the_gadgets_context`; unit `guion-cli reel::config::tests::the_format_and_youtube_are_optional` |
| AC2 | `ac2_bands_per_format_defaults_here_overrides_in_the_profile` |
| AC3 | `ac3_a_vertical_reel_renders_the_same_bytes_through_the_format`; `guion-estudio …::ac3_a_scene_without_formato_saves_the_same_bytes`; R-0012's tests unchanged and green; out of tree, below |
| AC4 | `ac4_the_srt_is_timed_by_voz_json`; the `srt` failure in `ac7_…` |
| AC5 | `guion-publish tests/r0016_youtube.rs::ac5_chapters_follow_youtubes_rules_and_fail_naming_the_rule`, `ac5_timestamps_as_youtube_reads_them`; `guion-reel …::ac5_the_gate_reads_chapters_back_from_the_description`; unit `guion-gadget linea::tests::los_eventos_tambien_son_anclas` |
| AC6 | `ac6_the_thumbnail_is_1280_by_720_and_light`; `guion-publish …::ac6_*` (parse, description, limits) |
| AC7 | `ac7_a_16_9_reel_renders_and_passes_the_gate_as_youtube` (and fails where it should: a moved cue, a lost chapter; the 16:9 MP4 refused as a Short) |

### AC3 out of tree (2026-10-20)

Scene binaries built from guion-video-creator before (d44fde2) and after
this change, into a scratch `CARGO_TARGET_DIR`, run with `GUION_FORMATO=reel`;
nothing committed in the account repos, their files restored.

| Output | sha256 before = after |
|---|---|
| rotorf-sico `frenar-y-girar.mp4` (default features) | `db29be15…8512380c` |
| rotorf-sico `frenar-y-girar.mp4` (`--features guion-reel`) | `db29be15…8512380c` |
| rotorf-sico `eventos.json` (both) | `e310a2a5…891237c1de8` |
| fisicobuenfisico `como-leer.mp4` | `5a67d7a2…cd85a955b1` |
| fisicobuenfisico `eventos.json` | `c657f032…f8fceb042` |

`guion reel check` on both: the same 18 findings (PASS), `check.json` equal
but for its run time.

## 4. Decision log

See R-0016 §6.

## Changelog

- 2026-10-20 — created with R-0016.
