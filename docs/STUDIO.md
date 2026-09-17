# guion studio — quickstart para creadores

YouTube-first creator workflow: escribe un screenplay TOML, simula, renderiza frames verticales (9:16), genera narración y exporta un reel `.mp4`.

## Requisitos

| Herramienta | Para qué |
|-------------|----------|
| Rust 1.75+ | CLI y studio |
| `ffmpeg` | `guion encode` (mux video + audio) |
| `piper` (opcional) | TTS real con `--engine piper` |
| WebKit/GTK (Linux) | Ventana Tauri del studio |

Instala ffmpeg:

```bash
sudo apt install ffmpeg   # Debian/Ubuntu
```

## CLI — pipeline completo

Desde la raíz del workspace `guion/`:

```bash
# 1. Validar screenplay
cargo run -p guion-cli -- check templates/reel-39-shoulder-press.screenplay.toml

# 2. Renderizar frames PPM (1080×1920)
cargo run -p guion-cli -- render templates/reel-39-shoulder-press.screenplay.toml

# 3. Generar narración WAV (scaffold = tonos temporizados por cue)
cargo run -p guion-cli -- narrate templates/reel-39-shoulder-press.screenplay.toml

# Con cama musical 8-bit + voz mezclada en un solo WAV:
cargo run -p guion-cli -- narrate templates/reel-39-shoulder-press.screenplay.toml --with-bed

# Piper (offline) cuando tengas el binario y modelo ONNX:
cargo run -p guion-cli -- narrate templates/reel-39-shoulder-press.screenplay.toml \
  --engine piper --piper-model /path/to/es_ES-sharvina-medium.onnx

# 4. Exportar MP4 (auto-narra si falta out/<slug>/narration.wav)
cargo run -p guion-cli -- encode templates/reel-39-shoulder-press.screenplay.toml --narrate
```

Salidas típicas:

```
out/reel-39-shoulder-press/frames/frame_00000.ppm …
out/reel-39-shoulder-press/narration.wav
out/reel-39-shoulder-press/mixed.wav      # cama + narración
out/reel-39-shoulder-press/reel.mp4
```

## Screenplay — audio y narración

```toml
[audio]
generator = "fbf-default"          # cama 8-bit (Rust, sin Python)
narration_script = "templates/foo.narration.txt"  # opcional
path = "assets/custom-bed.wav"     # opcional, tiene prioridad si existe

[[narration]]
text = "Tu línea de voz en off…"
at   = { start = 0.4, end = 3.8 }
```

- `[[narration]]` define cues con timestamps en segundos.
- `guion narrate` lee esos bloques (o un `.narration.txt` con `[0.4–3.8]`).
- El motor por defecto (`scaffold`) coloca tonos en cada ventana — útil para editar timing antes de conectar Piper.
- `guion encode --narrate` genera la narración si falta y mezcla con `generator`.

## guion studio (Tauri)

Interfaz oscura con preview vertical, scrubber y botones Check / Preview / Render / Narrate / Export.

```bash
cd guion
cargo run -p guion-studio
```

Build de distribución (requiere deps de sistema para WebKit):

```bash
cargo install tauri-cli --version "^2"
cd guion/apps/guion-studio
cargo tauri build
```

## Motores TTS (`NarrationEngine`)

| `--engine` | Descripción |
|------------|-------------|
| `scaffold` | MVP sin deps: tonos alineados a cada cue |
| `piper` | Subprocess a `piper` + modelo ONNX |

Para añadir otro motor, implementa `guion_audio::NarrationEngine` en un crate nuevo y regístralo en `engine_by_name`.

## MVP vs próximos pasos

**MVP (esta entrega)**

- `guion narrate` con timeline alineado a `[[narration]]`
- Cama `fbf-default` en Rust (port de `tools/audio8.py`)
- `guion encode` mezcla narración + generator
- Studio Tauri mínimo llamando los mismos crates

**Siguiente**

- Preview con frame PPM embebido en la UI
- `piper-rs` embebido (sin subprocess)
- Duración de reel basada en narración (hoy `duration()` usa el hook)
- Subtítulos quemados opcionales en encode
