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
| AC2 | El cambio tiene DIRECCIÓN: la tinta de `pista` es la clara y el cuadro sale netamente más claro | `guion-cli/tests/marcas.rs::la_tinta_de_pista_es_la_clara_y_se_nota_en_el_cuadro` |
| AC3 | Una marca que no existe falla, nombra las que hay, y no escribe cuadros | `guion-cli/tests/marcas.rs::una_marca_que_no_existe_no_renderiza_nada` |
| AC4 | `pista` es otra marca de verdad (papel oscuro, tinta clara, otro acento) | `guion-brand/tests/marcas.rs::pista_es_otra_marca_no_la_de_siempre` |
| AC5 | Cada tinta se mide contra la superficie en la que se pinta | `guion-brand/tests/marcas.rs::cada_tinta_contra_la_superficie_en_la_que_se_pinta` |
| AC6 | La rampa de calor se lee como escala en las dos marcas | `guion-brand/tests/marcas.rs::la_rampa_de_calor_se_lee_como_escala` |
| AC7 | `skin` es igual en las dos marcas | `guion-brand/tests/marcas.rs::la_piel_no_es_identidad_de_marca` |
| AC8 | El acento de `pista` es frío | `guion-brand/tests/marcas.rs::el_acento_de_pista_es_frio_porque_el_calor_ya_ocupa_lo_calido` |

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

- **OQ-1 — el motor no pinta el papel.** Medido en esta rama: renderizando
  el mismo guion bajo las dos marcas, el fondo del cuadro sale
  **negro en las dos**; sólo cambian los trazos. `paper` se usa como color
  nombrado pero nadie lo pinta como suelo de la página. En `fbf` (papel
  crema) la diferencia es enorme. Hasta que se cierre, ninguna pieza sale
  del motor con su fondo de marca.
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

## Changelog

- 2026-09-24 — creado, implementado y en revisión en la misma rama.
