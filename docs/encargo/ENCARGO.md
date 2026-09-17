# Encargo

Construir la fábrica de video de **@fisicobuenfisico** en Rust, manejada
por datos, con **dos formatos de salida**: el reel vertical de Instagram
(1080×1920) y el video horizontal de YouTube (1920×1080, de varios
minutos). La física en un solo lugar, y verificada.

Antes de escribir código, lee [`FISICA.md`](FISICA.md) y
[`VERIFICACION.md`](VERIFICACION.md). Este archivo dice **qué** hacer y
**cuándo está terminado**; esos dos dicen **qué tiene que ser cierto**.

## 1. Lo que ya existe — reutilizar, no reescribir

| repo | qué aporta | estado |
|---|---|---|
| `garust` | álgebra geométrica: motores PGA 3D, `Chain::ik_dls` (IK amortiguada) | maduro, en uso |
| `physics-lab/mecanica` | modelos de gym con afirmaciones re-derivadas (`sentadilla.rs`, `gluteo.rs`, `patada.rs`); rama `rfc-002-mecanica` | 29 afirmaciones; le encontró un defecto al reel 20 |
| `guion` | TOML de guion → escena de `motoreel` → cuadros numerados; `guion-core`, `guion-assemble`, `guion-cli` | M1 funcionando: `guion-cli check screenplays/reel09-lever.toml` → *ok* |
| `fisicobuenfisico/tools/` | la fábrica de Python que produce los reels hoy | **congelada como fuente de fixtures.** El código es desechable; los números publicados no |

**No tocar `fisicobuenfisico/tools/`** salvo para leer. Y no editar
archivos de otros repos sin avisar: ya pasó una vez que un agente
sobrescribió `reel10.py` y `reel11.py`.

## 2. Decisiones ya tomadas (R-0001, aceptado)

Están en `fisicobuenfisico/docs/R-0001-maquina-humana.md`. No se
reabren; si alguna estorba, dilo y se discute, pero no la cambies solo.

- **Q1** — el `Mecanismo` va en el kernel de `garust`: es física de
  propósito general, no autoría.
- **Q2** — árbol cinemático por **arreglo de padres**, en
  `garust-geo/tree.rs` **nuevo**; `chain.rs` **no se toca**. Se pidió la
  opción más genérica: el arreglo de padres subsume la cadena abierta y es
  la representación estándar de multicuerpos.
- **Q3** — la máquina humana va en `physics-lab/mecanica`: es un modelo
  con afirmaciones, no kernel.
- **Q4** — Python se queda **como fuente de fixtures**, no como destino.
- **Q5** — tolerancia de energía **2 %**. Si no cierra, se entiende por
  qué; **no se sube la tolerancia**.

## 3. El listón

[`../fixtures/dorados.json`](../fixtures/dorados.json) — **135 muestras de
12 piezas ya publicadas**. Para cada pieza: sus constantes, y entradas →
salidas de su propia función tal como está publicada.

Las convenciones **no están homologadas a propósito**. Cada pieza mide
desde su propio cero y con su propia firma (`tau(phi, modo)`,
`tau(psi, r, P)`, `tau(beta_g, u)`, `resistencia(estilo, th)`).
Homologarlas es trabajo del motor nuevo; si se normalizaran al extraer, el
fixture dejaría de probar nada.

Las cuatro piezas del molde actual traen, además de torques: la **pose
completa** (posiciones articulares) en 5 instantes, el **vector unitario
de la fuerza**, los ángulos de hombro y codo, el **balance de energía**, y
las **variantes de sensibilidad** de los supuestos de máquina. La pose
está ahí para que la cinemática se pueda verificar **independiente** de la
aritmética de torques: si el motor acierta el torque con una pose
distinta, acertó por casualidad.

**Una pieza está migrada cuando el motor reproduce sus muestras al 2 % y
pasa las compuertas de `VERIFICACION.md`.** Ni antes, ni por parecido
visual.

## 4. Orden sugerido

1. **Cinemática.** Árbol por arreglo de padres + IK de dos círculos con
   **rama por vector de dirección** (`FISICA.md` §2). Verificar contra el
   campo `pose` de los fixtures antes de calcular un solo torque.
2. **Cargas y torques.** `trait Carga` (`Peso`, `Cable`) y dinámica
   inversa recorriendo el árbol. Verificar contra `hombro`/`codo`/`l5s1`.
3. **Trabajo y balance de energía.** Es la compuerta que no se puede
   fingir: cierra sólo si (1) y (2) están bien.
4. **Las compuertas como tests**, no como documento. `VERIFICACION.md` §A.
5. **Render.** Encima de `guion`. Aquí entra el segundo formato.
6. **Las compuertas visuales.** `VERIFICACION.md` §B y §C.

Después de (3) ya se puede migrar un reel y compararlo contra el
publicado. Antes, no.

## 5. Los dos formatos

Lo que **no** cambia entre formatos: la física, las compuertas, y que cada
número venga de una constante por f-string (nunca escrito a mano).

Lo que **sí** cambia:

| | reel (IG) | video (YouTube) |
|---|---|---|
| lienzo | 1080×1920 | 1920×1080 |
| duración | ~6–10 s, una repetición | minutos, varias secciones |
| letra mínima | 28 px | **recalcular** — `VERIFICACION.md` §C da ~50 px |
| estructura | molde mínimo: figuras, paneles vivos, gráfica, tarjeta final | por secciones, con narración |
| tarjetas | **cero**, salvo la tarjeta final de comparación | no aplica la misma restricción |

El molde mínimo del reel (una repetición que traza la gráfica, números
vivos, cero tarjetas, ~6 s, y una **tarjeta final** de comparación a dos
columnas con palomita por renglón) es una restricción de Instagram,
**no** una ley de diseño. En YouTube hay tiempo para explicar; lo que se
mantiene es la disciplina, no el formato.

## 6. Lo que hace fácil el trabajo, y lo que lo arruina

**Un objetivo declarado del proyecto:** que alguien **sin programar**
pueda usar todo el framework. Un guion en TOML que diga el ejercicio, las
dos opciones, la carga y la articulación objetivo, y de ahí salga el
video. Cada cosa que se pueda derivar del modelo — títulos, rótulos,
captions, láminas del carrusel, la portada — se deriva; **nada se escribe
dos veces**. Ésa es la razón de la compuerta 12 de `VERIFICACION.md`.

**Lo que lo arruina:** un número bonito que nadie comprobó. Ver §D de
`VERIFICACION.md`.

## 7. Terminado cuando

1. Los fixtures de las 12 piezas se reproducen al 2 %.
2. Las compuertas de `VERIFICACION.md` §A corren como tests y **fallan**
   cuando se les mete un error a propósito (probarlo: una compuerta que
   nunca ha fallado no está probada).
3. Un reel del molde actual sale por el motor nuevo y es indistinguible
   del publicado, con las compuertas visuales verdes.
4. Un video 16:9 sale del mismo guion, con la letra mínima recalculada.
5. Nada de lo anterior necesita tocar Python.
