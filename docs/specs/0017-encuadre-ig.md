# SPEC-0017 — Cover framing (`encuadre`)

- **Status:** In review
- **Realizes:** R-0017
- **Created:** 2026-11-14
- **Module(s):** `guion-reel` (`formato::Encuadre`, `ENCUADRE_VERTICAL`, `Bandas::encuadre`; `gate::encuadre`; `gate::profile` `[encuadre]`, `FormatoSpec::encuadre`, `Profile::encuadre`; `gate::Inputs::cover`), `guion-cli` (`[reel] cover` → `Inputs::cover`), `guion-cuenta` (scaffold title baseline)

## 1. Design

```
guion-reel/src/
├── formato.rs        # + Encuadre {min_top, max_bottom, center_y, max_offset}, ENCUADRE_VERTICAL, Bandas.encuadre
└── gate/
    ├── encuadre.rs   # measure (class → Rows), check (Rows vs Encuadre → Finding), skipped (horizontal)
    ├── profile.rs    # + [encuadre], [formato.<name>] encuadre (EncuadreBands), Profile::encuadre(F), load-time checks
    └── mod.rs        # + Inputs.cover; the cover frame classified once, during the one decode
guion-cli/src/reel/mod.rs  # inputs.cover = [reel] cover
guion-cuenta/src/nueva.rs  # [marca.titulo] y = 986 (was 760)
```

### The profile

```toml
[encuadre]                         # the brand's own (vertical) format; every key optional
min_top = 300.0
max_bottom = 1545.0
center_y = 960.0
max_offset = 60.0

[formato.short]
encuadre = { max_offset = 40.0 }   # one format, on top
```

`Profile::encuadre(F)`: `None` for a horizontal F. Otherwise
`ENCUADRE_VERTICAL` × (checked canvas height / 1920), then `[encuadre]` (when
F has the brand's orientation), then `[formato.<F>] encuadre`.

### The finding (`check.json`)

```json
{ "check": "encuadre", "pass": false,
  "detail": "cover at 2.00 s (f60): content y 290–1450, centre 870 (top ≥ 300, bottom ≤ 1545, centre 960 ± 60)",
  "at": ["top y 290 < 300 (above the 4:5 feed crop)",
         "centre y 870 is -90 from 960 (> ± 60, off-centre in the 3:4 grid crop)"] }
```

| Defaults (house, *reportado* estimates) | 9:16 | 16:9 |
|---|---|---|
| `min_top` / `max_bottom` | 300 / 1545 | — |
| `center_y` ± `max_offset` | 960 ± 60 | — |
| safe y (was 250–1545) | 300–1545 | 54–1025 |

## 2. Acceptance-criteria checklist

All in `guion-reel tests/r0017_encuadre.rs` unless noted.

| AC | Test |
|----|------|
| AC1 | `ac1_vertical_formats_default_to_instagrams_crops` |
| AC2 | `ac2_the_box_is_the_covers_ink_without_subtitle_watermark_or_strays` |
| AC3 | `ac3_a_centred_cover_passes` |
| AC4 | `ac4_content_above_the_feed_crop_fails_naming_top_and_limit` |
| AC5 | `ac5_content_under_the_reels_ui_fails_naming_bottom_and_limit` |
| AC6 | `ac6_an_off_centre_box_fails_naming_centre_and_limit` |
| AC7 | `ac7_an_account_overrides_the_thresholds_in_its_profile` |
| AC8 | `ac8_a_horizontal_format_is_not_checked`; `r0016_formats.rs::ac7_…` (a 16:9 reel still passes) |
| AC9 | `ac9_the_gate_checks_the_cover_frame_of_the_mp4` (end to end: frame 0 fails, the cover passes, a cover past the end fails; skipped without DejaVu); `guion-marca tests/r0014_plantilla.rs::ac7_the_template_reel_builds_renders_and_passes_the_gate` (19 checks) |

The `guion-cli` line that passes `[reel] cover` has no test of its own
(`gate_post` is private to the binary); AC9 covers `Inputs.cover`.

## 3. Decision log

See R-0017 §6.

## Changelog

- 2026-11-14 — created with R-0017.
