---
name: {%skill%}
description: Escribir o rehacer un reel narrado de «{%nombre%}» a la primera, desde su plantilla, y pasarlo por la compuerta automática antes de cualquier revisión. Úsala para todo reel nuevo o rehecho de esta cuenta.
---

# Reel narrado de «{%nombre%}», a la primera

El proceso es el genérico: `{%proceso%}`. Esto es lo que el autor necesita.

## Archivos del post

`guion reel nuevo "{%nombre%}" <slug> --titulo "…"` crea `posts/<slug>/` con
`reel.toml`, `voz.toml`, `sonido.toml`, `escena.rs` (solo los beats; los
tiempos salen de `voz.json`, nunca a mano), `caption.md` y `README.md`
(estado y números con procedencia). La marca es `cuenta.toml`: las escenas
usan las primitivas de `guion-marca` y no escriben colores ni tamaños.

## Reglas de juicio

1. Lo que solo vale en un modelo se dice como del modelo.
2. Cada número dice su procedencia en palabras.
3. Nada de causas circulares.
4. La animación no dibuja nada falso para {%materia%}.
5. Los asserts prueban algo: ata la cifra que dice la voz al dibujo.
6. Los símbolos se dicen como se leen.

## Antes de pedir revisión

1. `guion reel posts/<slug> --twice`: mismos bytes y la compuerta en verde.
2. Mira la hoja de contacto (`cuadros/hoja.png`) tú mismo.
3. Una sola revisión: `{%revisor%}` y `{%designer%}` en paralelo, sobre la
   hoja, `check.json` y `voz.toml`. Aplica sus ediciones y confirma barato lo
   marcado.

## Ahorro

- No releas manuales: esta skill, `cuenta.toml` y el post de referencia
  bastan.
- Informa en 30 líneas o menos: duración, LUFS, la compuerta, lo que el dueño
  decide.
- Commit solo de tus rutas. Lo genérico va a guion-video-creator.
