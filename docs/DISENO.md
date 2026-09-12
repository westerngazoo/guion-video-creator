# El diseño: por qué no es "un archivo por reel"

Este documento existe por una corrección del dueño (12 sep 2026):

> *«necesito que uses ese estudio y esas herramientas… no quiero python y
> quiero una herramienta que se pueda reusar por otros creadores después,
> no algo hardcodeado que de momento funciona pero no tiene abstracción
> ni diseño»*

Tiene razón, y el problema **no es el lenguaje**. Hoy existe en los dos:

| dónde | qué hay |
|---|---|
| `fisicobuenfisico/tools/` | `reel37.py`, `reel38.py`, `reel39.py`, `reel40.py` — ~400 líneas cada uno |
| `guion-fisica/src/` (Rust) | `reel04.rs`, `reel26.rs`, `reel29.rs`, `reel39.rs` — un struct por reel |
| `guion-models/src/lever.rs` | *"ported from `tools/reel09.py`"* |
| `reel39.rs` | `enum Variante { Militar, Smith }` + constantes duras |

**Portar el Python a Rust uno por uno no arregla nada: mueve el hardcodeo
de lenguaje.** Un `enum Variante { Militar, Smith }` es el nombre de dos
ejercicios concretos dentro del motor. El creador siguiente querrá comparar
otros dos, y tendrá que escribir Rust.

## Lo único que compraron los reels a mano

Descubrieron la interfaz. Los cuatro tienen **exactamente la misma forma**,
escrita cuatro veces sin nombrarla: `TARGET`, `pose(cual, u)`,
`torques(p)`, `tau_vivo`, `trabajo`, `balance`, `pico`, `curva`,
`MEDICIONES`, `filas_final`. Eso no es coincidencia: es el diseño pidiendo
salir. Aquí está nombrado.

## Las capas

```
guion (TOML)          lo que escribe el creador. Sin código.
      │
      ▼
Comparacion           N opciones + articulación OBJETIVO + criterios
      │               deriva: eje común, picos, trabajo, cruces, tarjeta
      ▼
Ejercicio             camino (postura ← progreso u) + cargas
      │               deriva: τ(articulación, u), W, pico, balance
      ▼
MaquinaHumana         articulaciones con nombre, fracciones de Winter
      │               lo ÚNICO que sabe qué es una "cadera"
      ▼
Mecanismo + Carga     árbol cinemático y fuerzas con línea de acción
                      física de propósito general, sin gimnasio
```

Las dos de abajo ya están decididas en R-0001 (Q1: `Mecanismo` en el kernel
de `garust`; Q2: árbol por arreglo de padres; `trait Carga` con `Peso`,
`Cable`, `Banda`, `Leva`). **`Ejercicio` y `Comparacion` son lo que falta**,
y son justo las dos que hoy se escriben a mano por reel.

## `Ejercicio` — la pieza que falta

```rust
pub trait Ejercicio {
    /// La postura en el instante `u` de la repetición. u=0 inicio, u=1 cierre.
    fn postura(&self, u: f64) -> Postura;
    /// Las fuerzas externas en esa postura, cada una con su línea de acción.
    fn cargas(&self, p: &Postura) -> Vec<(Punto, Fuerza)>;
    fn rango(&self) -> (f64, f64) { (0.0, 1.0) }
}
```

Todo lo demás son **funciones libres sobre el trait**, escritas UNA vez:

```rust
fn tau(e: &dyn Ejercicio, art: Articulacion, u: f64) -> f64;
fn pico(e: &dyn Ejercicio, art: Articulacion) -> (f64, f64);
fn trabajo(e: &dyn Ejercicio) -> f64;
fn balance_energia(e: &dyn Ejercicio) -> (f64, f64);   // músculos vs carga
```

**Dos implementaciones, no una.** Un ejercicio puede venir de:

- **`Mecanico`** — mecanismo + camino + cargas. Es el caso general (remo,
  press, peso muerto).
- **`FormaCerrada`** — un τ(φ) ya derivado y **ya verificado**. Es como
  entra `physics-lab/mecanica/src/gluteo.rs` sin reescribirlo: tiene seis
  afirmaciones probadas en Rust y sería un error volver a derivarlo.

Las dos exponen el mismo trait, así que `Comparacion` no distingue. Ésa es
la prueba de que la abstracción está en el lugar correcto.

## `Comparacion` — lo que es un reel

```rust
pub struct Comparacion {
    pub opciones: Vec<(String, Box<dyn Ejercicio>)>,
    pub objetivo: Articulacion,
    pub criterios: Vec<Criterio>,
}

pub enum Sentido { MayorEsMejor, MenorEsMejor, Informativo }

pub struct Criterio {
    pub etiqueta: String,
    pub medida: Medida,      // PicoDe(art) | TrabajoDe(art) | Rango | ...
    pub sentido: Sentido,
    pub empate_si: f64,      // 0.15 = "iguales si difieren menos del 15%"
}
```

De aquí sale **todo** lo que hoy se escribe a mano: los paneles vivos, el
eje común, la gráfica, los cruces, los renglones de la tarjeta final con su
palomita, las láminas del carrusel y el caption. El veredicto de cada
renglón lo decide `Sentido` + `empate_si`, que es **política editorial
declarada**, no un `if` escondido en un reel.

## Lo que esto le arregla a las compuertas

Hoy `revisa.py` tiene ~30 comprobaciones escritas **por reel**. Sobre el
trait son **propiedades sobre cualquier `Ejercicio`**, escritas una vez y
aplicadas a todo lo que exista:

- el balance de energía cierra al 2 %;
- el τ del objetivo se mueve ≥30 % en alguna opción;
- el τ impreso es el pico;
- la palanca **dibujada** por la fuerza es el τ del modelo;
- convergencia, continuidad, monotonía, límites.

Un reel nuevo hereda las compuertas por existir. Eso es lo que hoy no pasa:
cada reel nuevo necesita que alguien recuerde escribirle sus pruebas.

## El guion del creador

El objetivo declarado: **alguien que no programa arma un reel.**

```toml
[comparacion]
titulo    = "Glúteo: hip thrust vs rumano"
objetivo  = "cadera"
carga_kg  = 100

[[opcion]]
nombre       = "rumano"
modelo       = "bisagra_de_cadera"   # la palanca es el torso
torso_m      = 0.53
rango_grados = 72

[[opcion]]
nombre       = "hip thrust"
modelo       = "puente_de_cadera"    # la palanca es el fémur
femur_m      = 0.46
rango_grados = 40

[[criterio]]
etiqueta = "Pico en la cadera"
medida   = "pico"
sentido  = "informativo"
empate_si = 0.15
```

Nada de ese archivo menciona un reel, un color, un pixel ni un número de
salida. Ése es el examen: **si para un ejercicio nuevo hay que tocar Rust,
el diseño todavía no está.**

## El primer caso de prueba: reel 40, congelado

`reel40.py` (glúteo: hip thrust contra rumano) está calculado, verificado
contra `gluteo.rs` y **deliberadamente nunca publicado en Python**. Sus
números viven en `fixtures/dorados.json` como `reel40`:

| | rumano | hip thrust |
|---|---|---|
| τ cadera abajo | 494.5 N·m | 345.7 N·m |
| τ cadera arriba | 0.0 | 451.3 |
| W por repetición | 359.3 J | 290.1 J |
| cruce de las dos curvas | al **30.4 %** de la subida | ← el hallazgo |

El TOML de arriba tiene que producir exactamente eso. **Ése es el criterio
de que el diseño funciona**, y por eso el reel no se publicó a mano: un
caso de prueba que ya salió por el camino viejo deja de ser una prueba.
