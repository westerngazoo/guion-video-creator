# R-0008 — La segunda marca

- **Status:** In review
- **Milestone:** M3
- **Owner:** físico buen físico (see project-specifics.md)
- **Created:** 2026-09-24
- **Depends on:** R-0003 (CLI render), M3 (guion-brand)
- **Realized by:** esta misma rama (cambio chico, sin SPEC aparte)

## 1. Statement

El motor deja de tener UNA identidad adentro. `guion-brand` pasa de una
marca embebida (`fbf`) a una **tabla de marcas** resuelta por nombre desde
`meta.theme`, y un nombre que no está en la tabla es un **error**, nunca un
regreso callado a la marca por omisión.

La segunda marca es `pista`: automovilismo, suelo azul marino, acento frío.

## 2. Rationale

Hay dos razones, y la segunda es la que manda.

**La primera es que hace falta.** Se abre una cuenta de karting, que es
otro público y otra identidad. `guion-brand` ya cargaba paletas desde TOML
(`themes/<marca>/palette.toml`) y ya tenía `load_theme`: la mitad del
trabajo estaba hecha y sin conectar.

**La segunda es un defecto que ya cobró.** El 2026-09-24, en la fábrica
vieja, se renderizó una pieza completa con la identidad equivocada y
ninguna de sus 41 comprobaciones lo dijo — porque ninguna preguntaba por
la marca. Se descubrió mirando el mp4. El binario de guion tenía el mismo
silencio, escrito más explícito:

```rust
match sp.meta.theme.as_deref() {
    Some("fbf") | None => default_theme(),
    _ => default_theme(),          // <- `theme = "pista"` cae aquí
}
```

y `render` ni siquiera miraba `meta.theme`. O sea que `render` y `encode`
podían sacar la misma pieza con dos identidades distintas, las dos con
código de salida 0.

Un render con la marca equivocada **se ve terminado**. Ése es justo el
fallo que tiene que gritar, y es lo que exige `CLAUDE.md` §6: fallar
fuerte ante lo desconocido, nombrando lo que sí existe.

## 3. Acceptance criteria

| # | Criterio | Prueba |
|---|----------|--------|
| AC1 | La misma pieza bajo dos marcas no da el mismo cuadro | `guion-cli/tests/marcas.rs::la_misma_pieza_bajo_dos_marcas_no_da_el_mismo_cuadro` |
| AC2 | El cuadro se limpia con el **papel de la marca** (cierra OQ-1) | `guion-cli/tests/marcas.rs::el_cuadro_se_limpia_con_el_papel_de_la_marca` |
| AC3 | Una marca que no existe falla, nombra las que hay, y no escribe cuadros | `guion-cli/tests/marcas.rs::una_marca_que_no_existe_no_renderiza_nada` |
| AC4 | `pista` es otra marca de verdad (papel oscuro, tinta clara, otro acento) | `guion-brand/tests/marcas.rs::pista_es_otra_marca_no_la_de_siempre` |
| AC5 | Cada tinta se mide contra la superficie en la que se pinta | `guion-brand/tests/marcas.rs::cada_tinta_contra_la_superficie_en_la_que_se_pinta` |
| AC6 | La rampa de calor se lee como escala en las dos marcas | `guion-brand/tests/marcas.rs::la_rampa_de_calor_se_lee_como_escala` |
| AC7 | `skin` es igual en las dos marcas | `guion-brand/tests/marcas.rs::la_piel_no_es_identidad_de_marca` |
| AC8 | El acento de `pista` es frío | `guion-brand/tests/marcas.rs::el_acento_de_pista_es_frio_porque_el_calor_ya_ocupa_lo_calido` |
| AC9 | Las dos marcas son opuestas: una escribe oscuro sobre claro y la otra al revés | `guion-cli/tests/marcas.rs::una_marca_escribe_oscuro_sobre_claro_y_la_otra_al_reves` |
| AC10 | Lo que se escribe en un PPM es lo que se lee, byte por byte | `guion-brand/src/ppm_io.rs::tests::lo_que_se_escribe_es_lo_que_se_lee` |
| AC11 | El post-fx no corre la imagen ni rota los canales | `guion-brand/src/ppm_io.rs::tests::el_postfx_no_corre_la_imagen` |

## 4. Constraints & non-goals

- **No se toca `fbf`.** 47 piezas publicadas dependen de esos valores. El
  único cambio a su paleta es *nombrar* un color que ya usaba (`bar`).
- **No se inventan campos de marca que el motor no dibuje.** La `prenda`
  (lo que viste el muñeco) existe en la fábrica vieja y NO entra aquí:
  el motor todavía no pinta muñeco. Queda en preguntas abiertas.
- **No es un selector de marca por línea de comandos.** La marca la dice
  el guion, que es el contrato. Una bandera que la sobreescriba sería otra
  manera de renderizar con la identidad equivocada.

## 5. Open questions

- ~~**OQ-1 — el motor no pinta el papel.**~~ **Cerrada el 2026-09-25.**
  `PpmSink::with_background` existía desde siempre en motoreel y sólo la
  llamaba un *ejemplo*; ni `render` ni `encode` la usaban. Ahora las dos
  etapas limpian con `theme.paper()` y la página ocupa el 92.3 % del
  cuadro con el color exacto de la paleta (AC2). Al cerrarla salió a la
  luz el defecto del lector de PPM — ver el registro de decisiones.
- **OQ-4 — `guion-render` tiene su propia marca horneada.** `PAPEL`,
  `TINTA` y `ACENTO` son constantes dentro del crate del dialecto de
  comparación. Hoy sólo las usa su ejemplo, así que no se publica nada
  con ellas, pero es la tercera copia de la identidad en el árbol y hay
  que conectarla al tema antes de que ese dialecto salga por el CLI
  (R-0007, PR #5).
- **OQ-2 — `prenda`.** La fábrica vieja necesita un color que se despegue
  de `skin`: la familia mostaza entera cae a ΔE 19-25 de la piel y el
  muñeco se vuelve una mancha. Cuando el motor pinte muñeco, ese color
  tiene que existir en la paleta y tener su prueba de separación.
- **OQ-3 — el gris.** La fábrica vieja tiene cuatro grises calibrados
  contra su suelo (`gris`, `gris_f`, `rejilla_x`, `rejilla_y`) que esta
  paleta todavía no nombra. Son los que hacen legible una cuadrícula.

## 6. Decision log

- **2026-09-24 — `bar` es un campo, no un derivado de `ink`.**
  La primera versión de la prueba de contraste midió todo contra `paper`
  y reprobó tres colores de `fbf`, la marca que ya publica. No eran
  defectos: era el par equivocado — `accent2` no se pinta sobre el papel
  sino sobre el cintillo. La segunda versión supuso que toda marca tiene
  una superficie clara y una oscura y que `accent` va sobre la clara; en
  `pista` eso midió cian contra el color de TEXTO (1.54:1), porque en una
  marca de suelo oscuro `ink` no es fondo de nada. Las superficies son dos
  y ahora se llaman por su nombre: `paper` y `bar`. Una prueba que mide un
  par que no ocurre es peor que no tenerla, porque el primero que la vea
  reprobar le va a bajar el umbral.

- **2026-09-24 — el piso de la escala de calor es un trinquete, no un
  umbral cómodo.** La separación mínima entre tonos vecinos se fija en
  1.5:1 porque es lo que logra `fbf`, que ya publica. La regla es:
  *ninguna marca nueva puede leerse peor que la que ya funciona*. La
  primera rampa de `pista` no la pasaba — subí los cuatro tonos parejo y
  quedaron apretados (1.34:1, peor que `fbf`). La rampa que quedó mide
  1.71:1 de separación mínima y 3.63:1 contra su papel.

- **2026-09-24 — las marcas van embebidas, no leídas de disco.** Por la
  misma razón que `fbf` ya lo estaba: el render tiene que dar el mismo
  resultado desde cualquier directorio. Una marca que depende del cwd es
  una marca que algún día no se encuentra y se cae al tema por omisión sin
  avisar — que es exactamente el fallo que este requisito cierra.
  `load_theme` se queda para temas de fuera del repo.

- **2026-09-24 — la marca la dice el guion, no una bandera.** Se descartó
  `--tema` en el CLI. El guion es el contrato; una bandera que lo
  sobreescriba es otra manera de renderizar con la identidad equivocada,
  y esta vez sin que quede escrito en ningún archivo.

- **2026-09-25 — el lector de PPM se comía el valor máximo, y nadie lo
  había visto porque el fondo era negro.** Al pintar el papel, `fbf`
  salió (208,242,230) en vez de (242,230,208): los canales ROTADOS.
  `parse_p6_header` devolvía el final del renglón de dimensiones en vez
  del final del `255`, o sea 4 bytes de menos. Como 4 no es múltiplo de
  3, la imagen entera se corría un pixel Y cambiaba de canal, y como
  `postfx_dir` lee-procesa-escribe, el corrimiento se horneaba en el
  archivo. Llevaba ahí desde que existe el post-fx: (0,0,0) rotado sigue
  siendo (0,0,0), así que un fondo negro lo escondía entero.

  Lo que faltaba no era cuidado, era una prueba de ida y vuelta: escribir
  y volver a leer. Ahora existe (AC10), y con ella la de que el post-fx
  no corre la imagen (AC11).

## Changelog

- 2026-09-24 — creado, implementado y en revisión en la misma rama.
- 2026-09-25 — cerrada OQ-1 (el motor pinta el papel). En el camino salió
  el defecto del lector de PPM; AC2 se reescribió, y se agregaron AC9,
  AC10 y AC11.
