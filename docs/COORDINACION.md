# Coordinación — congelado hasta la mudanza

**Estado: 12 sep 2026.** Este documento existe para que dos agentes no
trabajen sobre el mismo árbol mientras se mueve de lugar. Es temporal: se
borra cuando la mudanza termine.

## La decisión

`westerngazoo/guion-video-creator` es **la única casa** de la herramienta.
Una herramienta que otros creadores van a reusar no puede vivir dentro del
repo de contenido personal del dueño, y hoy vive ahí.

## Qué está congelado, y por qué

| árbol | estado | razón |
|---|---|---|
| `fisicobuenfisico/guion/**` | **CONGELADO** | se mueve completo a este repo. Lo que se escriba ahí mientras tanto hay que moverlo dos veces |
| `fisicobuenfisico/tools/**` | **CONGELADO** | es la fábrica que publica hoy. No se borra hasta que el motor reproduzca `fixtures/dorados.json` |
| `fisicobuenfisico/{out,brand,fotos_in}/**` | libre | contenido, se queda en ese repo para siempre |
| este repo | **activo** | aquí se trabaja |
| `physics-lab` rama `ejercicio-comparacion` | **activa** | el motor. No es de nadie más ahora mismo |

## Lo que se mueve, cuando se mueva

De `fisicobuenfisico/guion/` → aquí:

- `apps/guion-studio` (Tauri), `crates/guion-audio`, `guion-brand`,
  `guion-encode`, `guion-motion`, `guion-assemble`, `guion-core`.
- **`crates/guion-fisica` NO se mueve tal cual.** Son `reel04.rs`,
  `reel26.rs`, `reel29.rs`, `reel39.rs`: un struct por reel. Eso es el
  antipatrón que [`DISENO.md`](DISENO.md) nombra. Lo que sirve de ahí son
  los **números**, y ya están en `fixtures/dorados.json`. Se reescribe
  como modelos del catálogo, no se copia.
- `crates/guion-models` (`lever.rs`, `press.rs`) se revisa igual: son
  modelos, que es la forma correcta, pero anunciados como *"ported from
  tools/reel09.py"*. Entran al catálogo con sus afirmaciones o no entran.

De `westerngazoo/guion` → aquí: `guion-narrate` (seis proveedores de voz,
incluida la voz propia grabada, con subtítulos) y lo que dependa de él.
**Ojo: ese árbol tiene 23 archivos sin commitear.** Hay que commitearlos
antes de mover nada, o se pierden.

## Respaldos, ya subidos

Ninguna de estas dos existía fuera del disco del dueño. Ahora sí:

- `westerngazoo/fisicobuenfisico` rama **`fabrica-python-respaldo`**
  (`12ead0f`) — la fábrica de Python completa y funcionando, rescatada de
  un *stash*. La rama `cursor/guion-playground-f88e` la borró siguiendo
  ADR-0002, que es la dirección correcta, pero el único respaldo era un
  stash: un `git stash drop` y se iba sin que nadie se enterara.
- `westerngazoo/physics-lab` rama **`ejercicio-comparacion`** (`78c4fc6`)
  — el eje de progreso promovido al raíz del crate y el defecto latente
  que eso destapó. 31 afirmaciones pasando.

## Qué sí se puede hacer mientras tanto

Sin tocar los árboles congelados:

1. **Commitear** los 23 archivos sueltos de `westerngazoo/guion`.
2. Leer [`DISENO.md`](DISENO.md) y decir si el diseño está mal — es el
   momento de discutirlo, no después de construirlo.
3. Trabajo de motor en `physics-lab`: modelos nuevos para el catálogo, con
   sus afirmaciones. Eso nunca estorba.

## Cómo se sabe que terminó

`fisicobuenfisico` se queda **sólo con contenido** — sin `guion/`, sin
`garust/`, sin `motoreel/` dentro. La herramienta compila y pasa sus tests
desde este repo, sola. Ahí se borra este archivo.
