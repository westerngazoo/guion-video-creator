# {{titulo}}

- **estado:** borrador ({{fecha}})
- **cuenta:** {{cuenta}}
- **formato:** reel 9:16, 1080×1920, 30 fps, narrado
- **voz:** `voz.toml` → `voz.wav` y `voz.json` (`guion reel voice`); la escena saca todos sus instantes de `voz.json`. El sonido es `sonido.toml` (`guion reel sonido`). `reel.toml`, para `guion reel`
- **creado:** {{fecha}} con `guion reel nuevo` desde `plantillas/reel`

## La idea (una sola)

La pregunta y su respuesta, en un párrafo. La fórmula acompaña a la intuición.

**Lo que este reel no dice** (a propósito, una idea por reel):
- …

## Guion (narrado)

| Beat | Voz (`voz.toml`) | En pantalla |
|---|---|---|
| 1 · Gancho | `gancho` | El título |
| … | … | … |
| Cierre | `cierre` | La tarjeta de la cuenta, quieta 2.3 s al final |

## Números

| Cifra | Valor | Procedencia | Fuente |
|---|---|---|---|
| … | … | medido / calculado / reportado / de ejemplo | cuándo · modelo · enlace |

## Pendiente

- [ ] `voz.toml` escrito; `guion reel voice posts/{{slug}}`
- [ ] los beats en `escena.rs`; `guion reel posts/{{slug}} --twice`: mismos bytes
- [ ] la compuerta en verde (`check.json`); la hoja de contacto (`cuadros/hoja.png`)
- [ ] `{%revisor%}` y `{%designer%}` en paralelo sobre la hoja, el informe y `voz.toml`
- [ ] caption, alt text y hashtags; estado `listo`
