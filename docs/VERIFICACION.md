# Las compuertas

En la fábrica de Python son `tools/revisa.py` (los números) y
`tools/choques.py` (lo que se ve). **No son consejos: son compuertas.** Si
una falla no hay mp4, y el error dice qué falló. Hay que portarlas a Rust
como tests, no como documentación.

De dónde salieron: durante meses "revisé" a ojo. Tres piezas salieron sin
una sola comprobación y una llevaba un error del 11 %. Se descubrió sólo
porque el dueño preguntó «¿estás seguro de esos 360?». La disciplina que
vive en un documento para que otro la implemente no es disciplina.

## A · Los números (`revisa.py`)

Cada una es una **propiedad física que tiene que valer siempre**, no una
prueba de regresión: no compara contra un valor guardado, compara contra
lo que la física obliga.

1. **Límites.** Lo que la física obliga pase lo que pase con el código.
   Palanca en cero → torque cero. Torso vertical → momento en L5/S1 ≈ 0.
   La barra a plomo del hombro → τ hombro = 0 al arrancar.
2. **Suma de partes.** El atajo cuadra con la suma explícita segmento por
   segmento (±0.5 %). Ésta cachó el error del 11 %.
3. **Convergencia.** El mismo número con paso de integración
   150/300/600/1200/2400 (±0.5 %). Uno que cambia con el paso no es un
   número, es ruido.
4. **Continuidad.** Nada brinca entre muestras vecinas. Un salto grande es
   un cruce de rama de IK o un ángulo que dio la vuelta, no un movimiento.
5. **Monotonía.** El ángulo va en la dirección que la física manda, sin
   devolverse.
6. **Balance de energía.** `FISICA.md` §6, ±0.5 %.
7. **Independencia del corte.** Un número que cambia según dónde cortes la
   integral está mal condicionado y no se publica (así se vetó un
   porcentaje de rodilla: el bloqueo metía +230 W).
8. **`MEDICIONES` declaradas.** Cada reel declara, por opción, `τ` y `W`
   con unidades que tengan sentido (τ de 5 a 3000 N·m, W de 5 a 5000 J).
   Un reel que no declara qué mide no sabe qué mide.
9. **`TARGET` se mueve.** El τ de la articulación objetivo cambia ≥30 %
   del pico durante la repetición, **en al menos una** de las opciones
   comparadas. (En el press el Smith es parejo a propósito — ésa es la
   noticia — y el militar es el que se mueve.)
10. **El τ impreso es el pico del objetivo** (±1 %).
11. **El área dibujada es el número impreso.** Se reintegra la curva que
    de verdad va a pantalla y se compara con la cifra (±1 %). Nació de un
    rótulo que decía "235 J en las dos" cuando el área dibujada de una era
    198: la cuenta verificada no era la dibujada.
12. **Ningún número con unidad escrito a mano.** Se recorre el AST del
    módulo: toda cadena literal que no sea f-string ni docstring y traiga
    dígito + unidad (`cm`, `m`, `J`, `N·m`, `kg`, `%`, `°`…) falla. Nació
    de un reel que decía «la barra viaja **5 cm** más» con el escalón ya
    en 10: la portada se había corregido, el cuerpo del reel no. Las
    invariantes físicas no lo veían porque no es física, es un texto.
13. **Lo que se afirma en pantalla, comprobado.** "Casi igual" = picos a
    menos del 15 %. "La diferencia" = al menos 1.3×. Si la frase no se
    puede comprobar, no se escribe.
14. **Sensibilidad a los supuestos de máquina.** `FISICA.md` §9.

## B · Lo que se ve (`choques.py`)

Se le pasa a la pieza un `ImageDraw` **espía** en lugar del real: cada vez
que alguien dibuja texto, el espía anota la caja y deja pasar la llamada.
No hace falta tocar la pieza. Corre el `draw_frame` **de verdad** en
5 instantes (más `base_frame` y la tarjeta final), porque muchos rótulos se
mueven con la animación y uno limpio a media repetición se encima al final.

Detecta:

1. **Texto sobre texto** (>22 % del área de la caja más chica).
2. **Texto tapado por un rectángulo relleno posterior.** Texto contra
   texto no lo ve, porque un panel no es texto: hay que apuntar quién
   pinta encima de quién y en qué orden.
3. **Texto fuera del cuadro.** Un cintillo terminaba en "…ya es otr" con
   el resto fuera del lienzo; nadie lo tapaba, así que (1) y (2) no lo
   veían.
4. **Letra por debajo del mínimo legible** (§C).
5. **Las láminas del carrusel** con el mismo espía (`--laminas`).

Lo que **no** ve, y conviene saberlo en vez de creer que cubre todo: los
títulos compuestos por rotación se dibujan como imagen aparte, no por
`d.text`.

> Primera versión de esta herramienta: reconstruía el dibujo a mano
> llamando a `grafica` y `figura` por separado, adivinando sus firmas.
> Encontró un choque y se perdió otros dos porque nunca llegó a los
> paneles. **Una prueba que dice "sin choques" cuando ni siquiera miró es
> peor que no tenerla.** Correr el render de verdad, siempre.

## C · Letra mínima — como fórmula, no como constante

El texto tiene que medir **≥10 px en el ancho de reproducción más chico
previsto**:

```
fs_min = ceil(10 · ancho_render / ancho_reproducción_mínimo)
```

| formato | render | reproducción | fs_min |
|---|---|---|---|
| reel vertical | 1080 | 390 px (teléfono) | **28 px** |
| YouTube 16:9 | 1920 | 390 px (teléfono en vertical) | **50 px** |

Los 28 px no se eligieron: se derivaron, y luego los confirmó la
audiencia — «las leyendas son difíciles de leer por la letra pequeña».
Para YouTube el número es otro; **recalcúlalo, no lo copies.**

## D · La regla que manda sobre todas

**Nada sin verificar.** A la cuenta la siguen fisioterapeutas y la gente
hace lo que ve. Antes de afirmar cualquier cosa de fisiología o anatomía
hay que buscar fuente; si no hay, no se dice. Los tests de este repo son
el lugar donde eso vive, no un README.
