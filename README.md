# guion video creator

El productor de video del **motor de mecánica**: `garust` (álgebra
geométrica) + `physics-lab/mecanica` (la máquina humana y sus
afirmaciones). En Rust, manejado por datos, con dos formatos de salida
(reel vertical 1080×1920 e YouTube horizontal 1920×1080).

## Esto no es un proyecto aparte — léelo antes que nada

> **UN SOLO MOTOR.**
>
> `garust` + `physics-lab/mecanica` es el motor, y es **el mismo** que
> consume la app **Goose Physics** (`westerngazoo/sargentAI`, requerimiento
> R-0045: modelo biomecánico de levantamientos, torque articular y
> comparación de variantes).
>
> **Instagram no es el producto: es el banco de pruebas público y el
> embudo.** Decisión del dueño, 6 sep 2026, registrada en el `ROADMAP.md`
> de esa app: *«la física es el embudo; la app es el destino»*. Los 7 mil
> seguidores de [@fisicobuenfisico](https://instagram.com/fisicobuenfisico)
> llegaron por mecánica de levantamientos, no por un registrador de
> series.
>
> De ahí sale todo lo demás de este repo. Cada reel es **una verificación
> del motor frente a una audiencia que corrige** —y que incluye
> fisioterapeutas— y al mismo tiempo lo que trae a esa audiencia a la app.
>
> Por eso **ninguna pieza puede calcular su física por su cuenta.** Si el
> reel y la app dan números distintos, uno de los dos miente, y el que
> queda en evidencia en público es el reel.

Hoy los reels los produce una fábrica de Python
(`fisicobuenfisico/tools/`) que dibuja con Pillow y calcula la mecánica a
mano, un módulo por pieza — exactamente el problema que esto resuelve.
Esa fábrica **se retira**; el motor no se reinventa aquí, se consume.

## Por dónde empezar

| archivo | qué es |
|---|---|
| [`docs/ENCARGO.md`](docs/ENCARGO.md) | **el encargo completo**: qué construir, en qué orden, y cuándo está terminado |
| [`docs/DISENO.md`](docs/DISENO.md) | **el diseño**: por qué NO es un archivo por reel. `Ejercicio` y `Comparacion`, y el examen que dice si ya está |
| [`docs/FISICA.md`](docs/FISICA.md) | la física exactamente como se calcula hoy. El motor tiene que reproducirla, no reinventarla |
| [`docs/VERIFICACION.md`](docs/VERIFICACION.md) | las compuertas que hay que portar. Ninguna pieza sale sin pasarlas |
| [`docs/PLAN.md`](docs/PLAN.md) | **el plan por fases**: qué corre al final de cada una |
| [`docs/COORDINACION.md`](docs/COORDINACION.md) | **temporal**: qué árbol está congelado mientras se hace la mudanza |
| [`fixtures/dorados.json`](fixtures/dorados.json) | 135 muestras de 12 piezas **ya publicadas**. Es el listón: si el motor no las reproduce al 2 %, dos piezas se contradicen en público |

## La regla que manda sobre todas

Nada sin verificar. A esta cuenta la siguen fisioterapeutas y la gente
hace lo que ve. Un número que no pasa las compuertas no se dibuja, y una
afirmación que el modelo no sostiene no se escribe — aunque quede mejor.
