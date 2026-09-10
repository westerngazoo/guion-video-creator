# guion video creator

La fábrica de video de **@fisicobuenfisico**, en Rust, manejada por datos.

Hoy los reels los produce una fábrica de Python (`fisicobuenfisico/tools/`)
que dibuja con Pillow y calcula la mecánica a mano, un módulo por pieza.
Funciona y está publicada, pero cada pieza reimplementa lo mismo con su
propia convención. Este repo es el destino: **un motor, dos formatos**
(reel vertical 1080×1920 e YouTube horizontal 1920×1080), con la física
en un solo lugar y verificada.

## Por dónde empezar

| archivo | qué es |
|---|---|
| [`docs/ENCARGO.md`](docs/ENCARGO.md) | **el encargo completo**: qué construir, en qué orden, y cuándo está terminado |
| [`docs/FISICA.md`](docs/FISICA.md) | la física exactamente como se calcula hoy. El motor tiene que reproducirla, no reinventarla |
| [`docs/VERIFICACION.md`](docs/VERIFICACION.md) | las compuertas que hay que portar. Ninguna pieza sale sin pasarlas |
| [`fixtures/dorados.json`](fixtures/dorados.json) | 135 muestras de 12 piezas **ya publicadas**. Es el listón: si el motor no las reproduce al 2 %, dos piezas se contradicen en público |

## La regla que manda sobre todas

Nada sin verificar. A esta cuenta la siguen fisioterapeutas y la gente
hace lo que ve. Un número que no pasa las compuertas no se dibuja, y una
afirmación que el modelo no sostiene no se escribe — aunque quede mejor.
