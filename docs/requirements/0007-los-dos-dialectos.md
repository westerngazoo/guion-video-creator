# R-0007 — El CLI entiende los dos dialectos

- **Status:** Done
- **Milestone:** M1 / el objetivo de `DISENO.md`
- **Owner:** físico buen físico (ver `project-specifics.md`)
- **Created:** 2026-09-21
- **Depends on:** R-0001 (esquema), R-0003 (CLI)
- **Realized by:** esta misma entrega (SPEC aparte no hizo falta: el
  diseño cabe en la sección 4)

## 1. Statement

`guion` tiene dos dialectos de archivo y los dos son legítimos:

| dialecto | tabla que lo marca | qué describe |
|---|---|---|
| **escena** | `[meta]` | lo que se ve, cuadro por cuadro |
| **comparación** | `[comparacion]` | lo que se **compara**, y de ahí se deriva |

El CLI debe **reconocer cuál tiene en las manos** y enviarlo a su lector,
en `check` y en `render`.

## 2. Rationale

El lector del dialecto alto (`guion_comparacion::guion::cargar`) y su
dibujo (`guion_render::escena_grafica`) existían desde antes, con sus
pruebas y sus números congelados en `fixtures/dorados.json`. Lo que
faltaba era el cable: **el binario no sabía que ese dialecto existe.**

La consecuencia era concreta y llevaba tiempo ahí: el primer guion
declarativo del proyecto, `guiones/reel40-gluteo.toml`, fallaba con

```
unknown field `comparacion`, expected one of `meta`, `camera`, `model`, …
```

Es decir, el archivo que `docs/DISENO.md` pone como el objetivo del
proyecto —*«que alguien sin programar pueda usar todo el framework»*— era
justo el que el CLI no podía leer.

## 3. Acceptance criteria

- **AC1.** `guion check <comparación.toml>` sale 0 e imprime los
  renglones con su **veredicto**, no sólo los valores. Una comparación
  sin veredicto es una tabla.
- **AC2.** El dialecto de escena sigue funcionando igual. El cable nuevo
  no puede costar el viejo.
- **AC3.** `guion render <comparación.toml> --out DIR` escribe una
  secuencia numerada de verdad — **un archivo por cuadro en un
  directorio**, no un directorio por cuadro, que es lo que hacía el
  ejemplo y sirve para mirar uno, no para encodear.
- **AC4.** `render` **dice lo que todavía no hace**. Hoy da la gráfica y
  no el molde: sin figura, sin paneles, sin tarjeta final. Un comando que
  escribe cuadros y calla el resto deja creer que ya está.
- **AC5.** Un archivo sin ninguna de las dos tablas se rechaza
  **nombrando las dos** que sí existen. No se adivina.
- **AC6.** El error que sale es **el del dialecto que el autor estaba
  escribiendo**. Un `[comparacion]` incompleto no puede reportar
  «unknown field `comparacion`» — el error del otro lector — que es
  precisamente el que confundió hasta hoy.

Las seis corren en `crates/guion-cli/tests/dialectos.rs`, **contra el
binario** y no contra las funciones por dentro: el defecto era que el
binario no sabía algo que sus crates sí sabían, y una prueba que llama a
la función se lo habría perdido.

## 4. Diseño

`guion_core::dialecto(texto) -> Result<Dialecto, NoSeSabe>` mira una
llave que distingue y devuelve la clase. Vive en `guion-core` porque es
una pregunta sobre el **formato**, y ése es el crate que posee el
contrato del formato (`CLAUDE.md` §2).

**Se detecta, no se adivina.** La alternativa barata —intentar un lector
y si falla intentar el otro— viola `CLAUDE.md` §6 («nombra la
ubicación»): un `[meta]` con una llave mal escrita acabaría reportando el
error del dialecto equivocado. Aquí se elige el lector primero y el error
que sale es el del dialecto correcto.

La sonda **ignora todo lo demás a propósito**: decide *quién* lee el
archivo, no si el archivo es válido. Un guion roto sigue teniendo
dialecto, y tiene que llegar a su lector para que ése dé el error bueno.

## 5. Constraints & non-goals

- **No es el molde.** `render` sobre una comparación da la **gráfica**:
  ejes, una curva por opción, y el cruce marcado. La figura, los paneles
  y la tarjeta final viven hoy en la fábrica de Python y su port es
  trabajo aparte. AC4 existe para que eso no se lea como terminado.
- **No se mezclan.** Un archivo con `[meta]` **y** `[comparacion]` es un
  error, no una preferencia. Elegir uno en silencio sería el *silently
  ignored* que §6 prohíbe.
- No se toca el esquema de ninguno de los dos dialectos.

## 6. Decision log

| Fecha | Decisión | Razón |
|---|---|---|
| 2026-09-21 | La detección vive en `guion-core`, no en el CLI | Es una pregunta sobre el formato, y `guion-core` posee el formato. El CLI sólo enruta |
| 2026-09-21 | Se detecta por llave, no por intento-y-error | Un intento fallido reporta el error del lector equivocado, que es el defecto que esto arregla |
| 2026-09-21 | `[meta]` + `[comparacion]` juntos es un error | Elegir uno callado es el «silently ignored» que la casa prohíbe |
| 2026-09-21 | `render` anuncia lo que falta | Escribir 60 cuadros sin decir que falta el molde entero es dejar creer que ya sale por aquí |
