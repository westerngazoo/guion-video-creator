# El proceso de un reel narrado (rápido y barato), para toda cuenta

Este es el proceso genérico de la casa (R-0014). Las cuentas lo enlazan desde
su `CLAUDE.md` y su skill; **no lo copian**. Lo que es de una cuenta (su
materia, su marca, su voz, su revisor) está en su `cuenta.toml` y sus agentes;
lo que es de todas, aquí.

Nació de una decisión del dueño (2026-10-16, en rotorf-sico): «no puede ser
que tarde siempre tanto y queme tanto token; una vez que el diseño, el
template y la voz están definidos, va en una capa de abstracción». Contado en
los reels de entonces: de 3 a 5 rondas autor → revisor → designer por reel, y
la mayoría de los FIX eran reglas ya escritas o medibles. Lo medible lo hace
una máquina; lo que es juicio, una sola ronda de personas.

## 1. Lo que una máquina mide antes que nadie

`guion reel check` (R-0012), con el perfil de la cuenta
(`brand/reel-perfil.toml`, derivado de `cuenta.toml`):

| Qué | Check |
|---|---|
| subtítulo: ancho, palabras, cifras y signos, líneas largas, uno a la vez | `sub-*` |
| un acento por cuadro, un foco por beat | `red`, `focus` |
| área segura y franja del subtítulo | `safe-area`, `sub-band` |
| ningún cuadro quieto de más de ~4 s con voz; cierre quieto ≥ 1.5 s | `still-voice`, `closing-hold` |
| duración máxima | `duration` |
| −14 LUFS, pico real ≤ −1 dBTP; la cama no sube en huecos cortos | `loudness`, `bed-gaps` |
| caption: 2 200 caracteres, 5 hashtags, U+202F, nada interno | `caption`, `units`, `internal` |
| dos renders, mismos bytes | `determinism` |
| YouTube (Short y largo): título ≤ 100, descripción ≤ 5 000, sin `<` ni `>`, nada interno | `yt-title`, `yt-description` |
| YouTube largo: el .srt igual a `voz.json`; capítulos (00:00, ≥ 3, ≥ 10 s); miniatura 1280×720 ≤ 2 MB | `srt`, `chapters`, `thumbnail` |

**El formato (R-0016)** sale de `--formato`, de `[reel] formato` en
`reel.toml` o de `formato` en `escena.toml`: `reel` (por omisión), `tiktok` y
`short` (9:16, 1080×1920; el Short hasta 3 min) o `youtube` (16:9,
1920×1080). La compuerta toma las franjas del formato del perfil de la cuenta
(`[formato.<nombre>]`) o, si no las escribe, las de la casa. El mismo MP4 de un
reel se comprueba como Short con `guion reel check <post> --formato short`.

**Un reel no va a revisión con la compuerta en rojo.** Su informe
(`check.json`, corto) es lo que leen los revisores, no cuarenta cuadros.

## 2. Los pasos

1. **Plantilla.** `guion reel nuevo <cuenta> <slug> --titulo "…"` copia la
   plantilla de la cuenta y registra la escena. Un reel es datos más una
   escena corta: `reel.toml`, `voz.toml`, `sonido.toml` y `escena.rs` con
   solo sus beats. El reloj de la voz, los subtítulos, la cama, la mezcla, el
   caption y las comprobaciones viven en guion-video-creator; la cuenta pone
   sus tokens (y, si los tiene, sus instrumentos y primitivas propias).
2. **El autor escribe bien a la primera.** Un solo agente por reel, con la
   skill de la cuenta (las reglas de juicio en una lista) y un encargo de
   menos de 40 líneas. No relee manuales enteros: la skill, `cuenta.toml` y
   el último post `listo` de la misma cuenta bastan.
3. **Un comando.** `guion reel <post> --twice`: voz si falta → escena (dos
   veces) → sonido → mezcla → caption y alt text → **la compuerta**. En
   YouTube, además: `titulo.txt` y `descripcion.txt` desde `youtube.md` (con los
   capítulos anclados a la voz o a los beats) y, en el video largo, el `.srt`
   y la miniatura (`[youtube] miniatura = <s>`). El autor mira la hoja de
   contacto (`cuadros/hoja.png`) él mismo antes de pedir nada.
4. **Una sola revisión, en paralelo.** El revisor de la materia (solo
   `voz.toml`, el caption, los asserts, la tabla de números y la hoja) y el
   designer (la hoja, `check.json` y los cuadros que la compuerta marque) leen
   lo mismo **a la vez** y devuelven **ediciones listas para aplicar**, en 30
   líneas o menos. «La materia antes que el diseño» se cumple porque un FIX
   del revisor detiene la entrega aunque el designer haya dicho SHIP.
5. **Confirmar barato.** Tras aplicar, se confirma solo lo marcado (los
   tiempos y los archivos tocados) con un modelo chico y la compuerta otra
   vez; nada se revisa entero dos veces.
6. **En lote.** Varios reels de la misma plantilla van juntos (autor →
   compuerta → revisión → confirmación); `guion tablero lote check --cuenta X`
   corre la compuerta sobre todos, y el administrador solo ve el resultado.
7. **Commit solo de tus rutas.** Cada agente agrega por ruta lo que tocó
   (`git add posts/<slug> motor/Cargo.toml`), nunca `git add -A`: en el mismo
   repo trabajan otras sesiones y otras cuentas.

## 3. Las reglas de juicio (para el autor y el revisor)

1. Lo que solo vale en un modelo se dice como del modelo («en el modelo»,
   «si…», «en el caso ideal»).
2. Cada número dice su procedencia en palabras: de ejemplo, calculado (no
   medido), medido (cuándo) o reportado (fuente en el README, verificada).
3. Nada de causas circulares: se dice la causa.
4. La animación no dibuja nada falso para la materia.
5. Lo que la voz nombra se ve mientras se dice; «adentro», «arriba» necesitan
   una referencia en pantalla.
6. Los asserts prueban algo: atan la cifra que dice la voz al dibujo.
7. Los símbolos se dicen como se leen.

## 4. Lo que no cambia, en ninguna cuenta

Las reglas del `CLAUDE.md` de cada cuenta (§2): nada se publica sin el dueño;
todo número tiene procedencia; privacidad y consentimiento (menores, con sus
tutores); la materia se revisa antes que el diseño y su FIX detiene el post.

## 5. Ahorro

- Informes de 30 líneas o menos: duración, LUFS, la compuerta, lo que el
  dueño decide.
- Lo genérico (reloj, subtítulos, cama, mezcla, checks, tablero, este
  proceso, los revisores genéricos) se arregla aquí, en guion-video-creator,
  nunca en una cuenta.
- Una cuenta nueva no copia este proceso ni los agentes de otra:
  `guion cuenta nueva` los genera de `templates/cuenta/`.
