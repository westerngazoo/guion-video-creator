# Arquitectura — qué existe hoy, y dónde

[`DISENO.md`](DISENO.md) dice cómo tienen que ser las capas.
Este documento dice **qué está construido, en qué archivo, y con qué
afirmaciones**. Se actualiza cuando algo se construye, no cuando se
planea: si una fila dice "hecho" es porque hay tests corriendo.

## Por qué esto no existe ya

Manim anima matemáticas. El péndulo de un video de Manim se mueve porque
el autor escribió la solución en la animación — **la animación es la
fórmula**. Si el autor se equivocó, el video sale precioso e igual de
equivocado, y nadie lo nota.

Lo que falta es un motor donde **la animación no pueda contradecir a la
física, porque no tiene su propia copia de la física**. El número se
deriva de la geometría y las fuerzas; el dibujo lo lee. Cuando eso vale,
un video de conservación de energía no lleva barras decorativas: lleva la
**compuerta** dibujada, y si el modelo estuviera mal la barra se vería
crecer.

Ése es el producto, y es lo que hace que el mismo motor sirva para el
Instagram y para la app: si el reel y la app no comparten la física, una
de las dos le está mintiendo a alguien que paga.

## El mapa, capa por capa

| capa | dónde vive | estado |
|---|---|---|
| **`Mecanismo`** — árbol cinemático | `garust/crates/garust-geo/src/tree.rs` | **hecho** · 5 afirmaciones |
| **IK de dos eslabones** — la rama como dato | `garust/crates/garust-geo/src/twolink.rs` | **hecho** · 7 afirmaciones |
| **`Carga`** — fuerzas con línea de acción | `garust/crates/garust-physics/src/load.rs` | **hecho** · `Weight`, `Cable` |
| **Dinámica inversa** — τ de lo distal | `garust/crates/garust-physics/src/multibody.rs` | **hecho** · 5 afirmaciones |
| **`Lift`** — ejercicio de forma cerrada | `physics-lab/mecanica/src/lib.rs` | **hecho** · 31 afirmaciones |
| **`MaquinaHumana`** — articulaciones con nombre | `physics-lab/mecanica/src/maquina_humana.rs` | **hecho** · 4 afirmaciones + poses doradas |
| **`Ejercicio::Mecanico`** — el caso general | `physics-lab/mecanica/` | **falta** |
| **`Comparacion`** — lo que es un reel | `guion-video-creator/crates/guion-comparacion/` | **hecho** · 5 afirmaciones |
| **Guion TOML** — lo que escribe el creador | `guion-video-creator/guiones/` | **hecho** · un caso |
| **Render** | — | **falta** (fase 3) |
| **Narración y subtítulos** | `westerngazoo/guion` → `guion-narrate` | existe, **sin conectar** |

**247 afirmaciones corriendo** entre las capas construidas (157 `garust-geo`,
48 `garust-physics`, 35 `mecanica`, 7 `guion-comparacion`), y entre ellas la
que cierra el paso: el codo de los reels 38 y 39 cae **donde cayó en el
video publicado**, al 2%, resuelto por la máquina y no por el módulo que
los dibujó.

## Ramas vivas

| repo | rama | qué trae |
|---|---|---|
| `garust` | `tree-kinematico` | el árbol, las cargas y la dinámica inversa |
| `physics-lab` | `ejercicio-comparacion` | el eje de progreso al raíz + el defecto que destapó |
| `guion-video-creator` | `main` | `Comparacion`, el guion TOML, los contratos |

## Las costuras, y por qué están donde están

**`garust` no sabe qué es un gimnasio.** `Weight` y `Cable` son fuerzas
con punto de aplicación y dirección; nada ahí menciona una barra ni una
cadera. Un kernel que sabe qué es una sentadilla tiene mal el límite.

**`mecanica` es lo único que nombra articulaciones.** Ahí viven las
fracciones de Winter y el vocabulario del cuerpo. Y ahí van los modelos
**con sus afirmaciones**: un modelo sin claims no entra al catálogo.

**`guion-comparacion` no calcula un solo torque.** Le pregunta a
`mecanica`. Lo que sí decide es **política editorial** — qué se compara,
en qué sentido, y cuándo dos números se declaran iguales — y eso es un
dato en el TOML, no un `if` escondido en un reel.

**El catálogo es la costura entre los dos trabajos.**
`guion-comparacion/src/catalogo.rs` mapea un nombre de modelo a un
ejercicio del motor. Agregar un modelo es trabajo de motor y aparece aquí
en una línea; componer una comparación es trabajo de creador y no toca
Rust. El examen del diseño es justamente ése.

## Dos restricciones de diseño que no son negociables

### 1. No asumir cuasi-estático

Los reels van parametrizados por **progreso** `u ∈ [0,1]`; un péndulo va
por **tiempo** `t`. La escena tiene que aceptar los dos desde el
principio. Meter la estática en los cimientos y generalizar después es
cirugía, y la mitad del contenido de física que queremos hacer es
dinámico.

### 2. Dejar abierto el camino variacional

Hoy la mecánica publicada es estática: `τ = Σ (r − pivote) × F`. Hamilton
no aporta nada ahí y meterlo sería complejidad sin justificación. Pero
`garust-physics` **ya está formulado en variables canónicas** —el estado
es `(Motor, bivector de momento angular)`, no velocidades— y el
integrador es **simpléctico por partición**. Donde eso se vuelve
necesario:

- **La sentadilla.** Le queda un grado de libertad libre, y elegirlo a
  mano es inventar la cinemática. Un principio variacional la **deriva**:
  convierte "inventé una bajada" en "declaré un principio", que se puede
  discutir y refutar. Con la advertencia de siempre: un principio de
  optimalidad es una hipótesis de control motor, no una ley, y hay que
  validarlo contra datos medidos antes de que un número salga en pantalla.
- **Potencia media de verdad**, cuando el tiempo salga del modelo y no de
  una declaración.
- **Cadenas cerradas** (el hip thrust tiene dos apoyos), donde los
  multiplicadores hacen solo lo que hoy se resuelve a mano.

Por eso `trait Load` toma **todas** las poses y no sólo la suya: una
restricción y una fuerza que depende de dos extremos caben en la misma
interfaz sin cirugía.

## Lo que el motor se niega a hacer

Y es parte del diseño, no una carencia:

- **Repartir la carga entre los músculos que cruzan una articulación.**
  Es matemáticamente indeterminado (RFC-002 §7.3), no "todavía no
  implementado".
- **Activación muscular.** Eso es EMG.
- **La bajada de la sentadilla**, hasta que haya un principio declarado y
  validado.
