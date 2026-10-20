---
name: hacer-video
description: Hacer un video de cualquier cuenta con guion-video-creator — reel de Instagram, TikTok, YouTube Short o video largo de YouTube — de la idea al paquete para subir, con la compuerta automática antes de cualquier revisión. Úsala para todo video nuevo o rehecho, en cualquier repo de cuenta (rotorf-sico, fisicobuenfisico, las que crea `guion cuenta nueva`).
---

# Hacer un video, a la primera

Un video es un **post**: una carpeta con su fuente (voz, escena, sonido, caption,
README). La cuenta pone la marca y las reglas; guion-video-creator pone todo lo
demás. Lo genérico vive aquí; lo de cada cuenta, en su `cuenta.toml`, su
`CLAUDE.md` y su `docs/estilo.md`. Proceso completo: `docs/proceso-reel.md`.

## 0. Lo que no se rompe (en todas las cuentas)

1. **Nada se publica sin el dueño.** El agente deja el post `listo` y entrega el
   paquete; el dueño sube. Se marca `publicado` solo cuando el dueño lo dice.
2. **Todo número tiene procedencia:** calculado (qué modelo), medido (cuándo,
   con qué), reportado (fuente con enlace, verificada en línea) o supuesto. En
   pantalla y en la voz se dice en palabras.
3. **Privacidad:** nada de caras, nombres o datos de terceros (menores: de sus
   tutores) sin permiso escrito.
4. **Primero el revisor de contenido de la cuenta** (fisico, computista,
   helenista…), **después designer**. Un FIX detiene el video.
5. **Determinismo:** misma fuente, mismos bytes. Un cambio en el MP4 sale de un
   cambio en la fuente.
6. Nada interno en lo publicado (IDs de requisitos, repos, notas del proceso).

## 1. Elige el formato

| Formato | Lienzo | Duración | Entrega | Notas |
|---|---|---|---|---|
| Reel de Instagram | 9:16, 1080×1920, 30 fps | 60–90 s (meta) | MP4 + portada + caption + alt text | ≤ 5 hashtags, caption ≤ 2 200 caracteres; la portada se recorta a 3:4 |
| TikTok | 9:16, 1080×1920 | igual que el reel | el mismo MP4 | sin portada propia: el primer cuadro manda |
| YouTube Short | 9:16, 1080×1920 | hasta 3 min (reportado, ayuda de YouTube, 2024-10) | MP4 + título | el mismo MP4 del reel sirve si dura ≤ 3 min |
| YouTube largo | 16:9, 1920×1080, 30 fps | 6–15 min | MP4 + miniatura 1280×720 (≤ 2 MB) + título (≤ 100) + descripción (≤ 5 000) con capítulos + subtítulos .srt | capítulos: el primero en 0:00, al menos tres, de ≥ 10 s cada uno (reportado, ayuda de YouTube; verifícalo antes de depender de ello) |

Audio en todos: voz sobre la cama a **−14 LUFS**, pico real ≤ −1 dBTP.

**Hoy (2026-10-20) el motor y la compuerta están hechos para 9:16.** El 16:9,
la miniatura, los capítulos y el .srt son el requisito R-0016 (en curso): hasta
que esté, un video largo de YouTube se arma como reels encadenados por capítulo
y se avisa al dueño. No inventes un perfil 16:9 a mano en la cuenta.

## 2. Crea el post

```bash
guion reel nuevo "<cuenta>" <slug> --titulo "<pregunta>"   # de la plantilla de la cuenta (guion-cuentas.toml)
```

- **Gancho = la pregunta física/técnica**, nunca clickbait («¿La polea te da
  tensión constante?»). Una idea por video.
- `README.md`: estado, nivel o serie, formato y la **tabla de números con su
  procedencia**.

## 3. Escribe la voz (`voz.toml`)

- Voz de la cuenta (`cuenta.toml`); la casa usa `es-carlfm-x-low` (es) y
  `en-gb-alan-low` (en). Tuteo en español.
- Lo que solo vale en el modelo se dice como del modelo («en el modelo», «si…»,
  «calculado, no medido»). Nada de causas circulares.
- Símbolos dichos como palabras («tau», «efe por ele») con verbo.
- **Subtítulo:** una línea, ≤ 8 palabras, ≤ 840 px, sin cifras ni signos; no
  repite lo que ya dice la pantalla (si una glosa o fórmula lo dice, la línea va
  sin subtítulo). Línea de más de 6 s → partirla y usar `desde`.
- Los tiempos salen de `voz.json`, nunca a mano.

## 4. Arma la escena

Dos caminos, el mismo contrato (R-0015):

- **Con gadgets (preferido para lo nuevo):** `escena.toml` con gadgets de la
  biblioteca (palanca, medidor… y los de la cuenta en `cuenta.toml [estudio]`).
  Ábrelo con `guion studio <post>`: sliders con procedencia, cursor de tiempo,
  el cuadro real; «guardar» escribe `escena.toml`. Los clips se anclan a líneas
  de la voz (`<id>`, `<id>.fin` ± s), nunca a un número suelto. Sin interfaz:
  `guion studio <post> --cuadro 3.2 --out f.png`, `--check`, `--sheet`.
- **Con código (`escena.rs`):** solo los beats; cada número que dice la voz,
  atado al modelo con un `assert` (bisección, no la misma cuenta dos veces).

La física y las fórmulas vienen de **physics-lab** (con oráculos); el motor las
consume, no las copia. Lo genérico que te falte va a guion-video-creator, nunca
al repo de la cuenta.

**Diseño que cumple el autor solo** (además de `docs/estilo.md` de la cuenta):
un foco por beat, un solo rojo/acento fuerte por cuadro, oro (o el papel de
"fórmula" de la cuenta) solo para lo que sale de una fórmula; nada quieto más de
~4 s con voz; etiquetas que no chocan (colocación por obstáculos) y a ≤ 56 px de
su línea o con guía; nada en la franja del subtítulo; cierre quieto ≥ 1.5 s;
signo menos como barra, no guion.

## 5. Sonido (`sonido.toml`)

Instrumentos registrados por la cuenta; la cama no sube en huecos < 0.5 s
(`[agachar] unir = 0.5`). Un solo reloj: los golpes salen de `eventos.json`.

## 6. Render y compuerta, antes de cualquier revisión

```bash
guion reel <post> --twice        # voz → escena → sonido → mezcla → caption → compuerta
guion reel check <post>          # debe salir 18/18 con el perfil de la cuenta
```

Mismos bytes en las dos corridas. Mira la hoja de contacto tú mismo. Solo
entonces: **una ronda de revisión en paralelo** (revisor de contenido + designer)
sobre la hoja, el informe de la compuerta y `voz.toml`. Junta todos los FIX en
una sola pasada; las confirmaciones, con un modelo barato.

## 7. Caption y entrega

- Caption: U+202F antes de unidades y %, ≤ 5 hashtags, ≤ 2 200 caracteres,
  procedencia en palabras, nada interno. El bin `caption` lo comprueba y escribe
  `caption.txt` y `alt-text.txt` (no se versionan en el post).
- Paquete en `publicar/<cuenta>/<tema>/`: MP4 con audio, portada sin
  subtítulo (legible en la cuadrícula y en el recorte 3:4), `caption.txt`,
  `alt-text.txt`; su entrada en `ORDEN.md` con caption y alt text para pegar.
- Estado `listo` en el README; al dueño, el video y una línea de qué decide.

## Ahorro

No releas manuales: esta skill, el `CLAUDE.md` de la cuenta y el último post
`listo` de la misma cuenta bastan. Informa en ≤ 30 líneas: duración, LUFS,
compuerta, cuadros a mirar, lo que decide el dueño.
