# El plan, por fases

Cada fase termina en algo que **corre y se puede comprobar**, no en un
documento. El orden no es negociable en un punto: la cinemática se verifica
antes de calcular un solo torque, porque un torque correcto con una postura
equivocada es una coincidencia.

## Fase 0 — hecha

`guion-comparacion`: un TOML produce los números del reel 40 al 2%, sin
Rust por ejercicio. Cinco tests. `Ejercicio` en su forma cerrada
(`mecanica::Lift`), `Comparacion` con criterios y veredictos.

De paso salieron dos defectos reales: el eje de progreso espejeado entre
módulos del motor, y `IGUALES` rotulando dos números que difieren 19%.

## Fase 1 — `Mecanico`: el ejercicio general

Hoy el catálogo sólo sabe de formas cerradas (`τ = mgL·sen φ`). La mayoría
de los ejercicios no tienen forma cerrada: hay que resolver la postura y
sumar momentos.

1. **Cinemática.** Árbol por arreglo de padres en `garust-geo/tree.rs`
   (R-0001 Q2), IK de dos círculos con **rama por vector explícito** — no
   por "la solución más alta", que ya produjo un codo al revés en
   producción. Se verifica contra el campo `pose` de los dorados de
   reel38/39 **antes** de tocar un torque.
2. **Cargas.** `trait Carga` con `Peso` (vertical) y `Cable` (línea de
   acción a la polea). R-0001 §6.1.
3. **Torques.** Dinámica inversa recorriendo el árbol: el momento en una
   articulación es la suma de lo distal a ella. Se verifica contra
   `hombro`/`codo`/`cadera`/`l5s1` de los dorados.
4. **Balance de energía.** La compuerta que no se puede fingir: cierra
   sólo si (1), (2) y (3) están bien.

**Termina cuando** reel38 y reel39 salen del catálogo, por TOML, al 2%.

## Fase 2 — las compuertas como propiedades

Las ~30 comprobaciones que hoy `revisa.py` repite por reel se vuelven
propiedades sobre `Comparacion` y `Ejercicio`, escritas una vez:
convergencia, continuidad, monotonía, límites, balance, el τ del objetivo
se mueve, el τ impreso es el pico, la palanca dibujada es el τ del modelo.

Y la prueba de que las compuertas sirven: **meterles un error a propósito
y ver que fallan.** Una compuerta que nunca ha fallado no está probada.

**Termina cuando** un ejercicio nuevo hereda las compuertas por existir.

## Fase 3 — el render

Un solo renderizador parametrizado por la `Comparacion`, no uno por reel:
figuras con su fuerza y su palanca, paneles vivos, la gráfica del eje
compartido con el cruce, la tarjeta final, el cintillo y la tira de
fórmula. Encima de `motoreel` y `guion-encode`, que ya existen.

Aquí entran los dos formatos (9:16 y 16:9) y la letra mínima **como
fórmula**, no como constante: `ceil(10 · ancho_render / ancho_mínimo)`.

**Termina cuando** un reel del molde sale por el motor y es
indistinguible del publicado, con las compuertas visuales verdes.

## Fase 4 — voz y subtítulos

`guion-narrate` ya existe y funciona: seis proveedores, incluida la voz
propia grabada, con `.srt` y `.vtt`. Falta conectarlo a la comparación:
que el guion declare los *beats* y la duración hablada mande sobre la
animación.

**Se prueba solo**, como una variable aparte, cuando el molde ya esté
estabilizado. No antes: el formato apenas empezó a funcionar.

## Fase 5 — retirar el Python y limpiar

En este orden, y no antes:

1. El motor reproduce las 13 piezas de `fixtures/dorados.json` al 2%.
2. Se publica un reel hecho con el motor.
3. `fisicobuenfisico/tools/` se borra de `main`. La rama
   `fabrica-python-respaldo` se queda como testigo.
4. `fisicobuenfisico` se queda sólo con contenido: `out/`, `brand/`,
   `fotos_in/`, `estado.json`, `PENDIENTES.md`.
5. Se borra [`COORDINACION.md`](COORDINACION.md).

## Lo que no está en el plan

Y no por olvido:

- **Fuerza muscular y activación.** Necesitan brazos de momento
  anatómicos con fuente, y EMG. No se insinúan.
- **Dinámica, 3D, estabilidad.** El modelo es cuasi-estático 2D y lo dice.
- **Cinemática de la sentadilla.** Tiene un grado de libertad libre; hace
  falta elegir y justificar una estrategia de descenso, con sus propias
  afirmaciones. Es un requerimiento aparte, no un efecto secundario de un
  reel.
