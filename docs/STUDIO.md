# guion studio — cómo correrlo en local

Guía práctica para creadores y desarrolladores: instalar dependencias, clonar el
repo, ejecutar el pipeline CLI completo (`check` → `render` → `narrate` →
`encode`) y abrir **guion studio** (interfaz Tauri).

YouTube-first: escribes un screenplay TOML, simulas, renderizas frames verticales
(9:16), generas narración y exportas un reel `.mp4`.

---

## Requisitos

| Herramienta | Versión | Para qué |
|-------------|---------|----------|
| **Rust** (stable) | 1.75+ (el repo fija toolchain en `rust-toolchain.toml`) | CLI, crates y studio |
| **ffmpeg** | cualquier 5.x/6.x reciente | `guion encode` (mux video + audio) |
| **WebKit/GTK** (solo Linux) | `libwebkit2gtk-4.1-dev` | Ventana Tauri de guion studio |
| **Xcode CLT** (solo macOS) | Command Line Tools | Compilar Rust y Tauri (WebKit nativo) |
| **piper** (opcional) | binario + modelo ONNX | TTS real con `--engine piper` |

Elige tu plataforma para instalar dependencias de sistema. El CLI (`guion-cli`)
no necesita GTK ni Xcode; solo **guion studio** (Tauri) sí.

---

## macOS

En Mac, Tauri usa **WebKit nativo** del sistema. No hace falta instalar GTK,
`libwebkit2gtk` ni paquetes apt.

### 1. Xcode Command Line Tools

```bash
xcode-select --install
```

Si ya los tienes, `xcode-select -p` debe mostrar una ruta bajo
`/Library/Developer/CommandLineTools` o Xcode.

### 2. Homebrew: ffmpeg y Rust

```bash
brew install ffmpeg rust
```

Alternativa para Rust (recomendada si quieres `rustup` y el toolchain del repo):

```bash
brew install ffmpeg
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
```

Al entrar en `guion/`, `rustup` lee `rust-toolchain.toml` e instala el canal
stable con `rustfmt` y `clippy`.

```bash
rustc --version
ffmpeg -version
```

### 3. Clonar y ejecutar guion studio

Misma regla de **motoreel hermano** que en Linux (ver [Clonar y preparar el workspace](#clonar-y-preparar-el-workspace)).
Desde la raíz `guion/`:

```bash
cd guion
cargo run -p guion-studio
```

La primera compilación tarda varios minutos; las siguientes reutilizan el cache
de Cargo.

### 4. Build de distribución (`.app` / `.dmg`)

```bash
cargo install tauri-cli --version "^2"
cd apps/guion-studio
cargo tauri build
```

Artefactos en `guion/apps/guion-studio/target/release/bundle/macos/`:

- `guion-studio.app` — aplicación lista para abrir
- `guion-studio_0.1.0_aarch64.dmg` (o `_x64_`) — instalador, según arquitectura

### Solución de problemas (macOS)

| Problema | Qué hacer |
|----------|-----------|
| **`pathspec 'main' did not match`** | Clone antiguo solo con `master` — ver [Clone antiguo en `master`](#clone-antiguo-en-master-sin-main-ni-studio) |
| **`apps/guion-studio/Cargo.toml`: No such file** | Repo/rama sin studio o cwd equivocado — ver [Carpeta ya existente](#carpeta-fisicobuenfisico-ya-existente-no-volver-a-clonar) y [Clone antiguo en `master`](#clone-antiguo-en-master-sin-main-ni-studio) |
| **`destination path 'fisicobuenfisico' already exists`** | Ya tienes el clone — no vuelvas a `git clone`; ver [Carpeta ya existente](#carpeta-fisicobuenfisico-ya-existente-no-volver-a-clonar) |
| **`zoxide: no match found`** | Usa `cd` normal (no `z`); la ruta puede no estar en la base de zoxide — ver [Carpeta ya existente](#carpeta-fisicobuenfisico-ya-existente-no-volver-a-clonar) |
| **`fatal: not a git repository`** | No estás dentro del clone — `cd` a `~/projects/fisicobuenfisico` antes de `git checkout` |
| **`guion-studio` not found in workspace** | Rama/repo sin studio — ver [Solución de problemas](#package-s-guion-studio-not-found-in-workspace) |
| **`zsh: command not found: #`** | No pegues líneas que empiezan con `#` en la terminal (son comentarios de esta guía) |
| **`zoxide` / `xoxide` not found** al abrir Terminal | No es de guion — plugin de shell en `~/.zshrc`; ver [zoxide no encontrado](#zoxide-not-found-command-not-found-en-macos) |
| **Gatekeeper** bloquea la app | Clic derecho → **Abrir**, o `xattr -cr guion-studio.app` |
| **Apple Silicon (arm64)** | Usa toolchain nativo: `rustc -vV` debe mostrar `host: aarch64-apple-darwin`. No necesitas Rosetta salvo que fuerces binarios x86_64 |
| Avisos **EGL** / `/dev/dri` | No aplican en Mac; ignóralos si los ves copiando logs de Linux |
| `failed to load … dependency motoreel` / `garust` | Clona los repos hermanos (ver [Setup completo](#setup-completo--clones-obligatorios)) |

---

## Linux

### Instalar Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
rustc --version
```

Al entrar en `guion/`, `rustup` lee `rust-toolchain.toml` e instala el canal
stable con `rustfmt` y `clippy`.

### Instalar ffmpeg

```bash
# Debian / Ubuntu
sudo apt update
sudo apt install ffmpeg

ffmpeg -version
```

### Dependencias de sistema para guion studio

Solo necesarias si vas a compilar o ejecutar la app Tauri.

```bash
# Debian / Ubuntu (Tauri 2 — WebKitGTK 4.1)
sudo apt update
sudo apt install \
  libwebkit2gtk-4.1-dev \
  build-essential \
  curl \
  wget \
  file \
  libxdo-dev \
  libssl-dev \
  libgtk-3-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev
```

En Ubuntu 24.04 y Debian recientes usa **4.1**, no `libwebkit2gtk-4.0-dev` (eso
es Tauri 1 y ya no está en los repos).

### Build de distribución (Linux)

```bash
cargo install tauri-cli --version "^2"
cd apps/guion-studio
cargo tauri build
```

Artefactos en `guion/apps/guion-studio/target/release/bundle/` (`.deb`,
`.AppImage`, etc., según la plataforma).

---

## Windows (nota breve)

Tauri 2 en Windows pide **Visual Studio Build Tools** con el workload de C++
y **WebView2** (suele venir en Windows 10/11). Instala Rust con
[rustup](https://rustup.rs/) y ffmpeg (p. ej. `winget install Gyan.FFmpeg`).
Los mismos comandos aplican desde `guion/`: `cargo run -p guion-studio` y
`cargo tauri build` en `apps/guion-studio/`.

---

## Clonar y preparar el workspace

`guion` usa **path dependencies** a repos hermanos del monorepo (no vienen en el
`git clone` de `fisicobuenfisico`):

```
directorio-padre/          # p. ej. ~/projects/fisicobuenfisico
├── guion/                 ← aquí corres todos los comandos cargo
├── motoreel/              ← motor de render (obligatorio)
└── garust/                ← álgebra geométrica para guion-assemble (obligatorio)
```

Rutas en los `Cargo.toml` del workspace:

| Crate | Path desde `guion/` | Repo |
|-------|---------------------|------|
| `motoreel` | `../motoreel/crates/motoreel` | [westerngazoo/motoreel](https://github.com/westerngazoo/motoreel) |
| `garust` | `../garust` (vía `crates/guion-assemble`) | [westerngazoo/garust](https://github.com/westerngazoo/garust) |

> **Nota:** `physics-lab/` aparece en tests opcionales de `guion-motion` (WASM);
> no es path dependency de Cargo y puede faltar sin romper `cargo check`.

### Setup completo — clones obligatorios

Ejecuta **un comando por línea** (no pegues líneas que empiezan con `#`). Ajusta
`~/projects/fisicobuenfisico` a tu ruta real:

```bash
cd ~/projects/fisicobuenfisico
git clone https://github.com/westerngazoo/motoreel.git motoreel
git clone https://github.com/westerngazoo/garust.git garust
ls motoreel/crates/motoreel/Cargo.toml
ls garust/Cargo.toml
cd guion
cargo check --workspace --exclude guion-studio
```

Alternativa automática (desde `guion/`):

```bash
./scripts/bootstrap-siblings.sh
cargo check --workspace --exclude guion-studio
```

### Opción A — monorepo fisicobuenfisico (recomendado)

El monorepo trae `guion/`; **motoreel** y **garust** son repos aparte que debes
clonar como hermanos (CI hace lo mismo con motoreel en
`.github/workflows/guion-ci.yml`):

```bash
git clone https://github.com/westerngazoo/fisicobuenfisico.git
cd fisicobuenfisico

git fetch origin
git checkout cursor/guion-model-source-design

git clone https://github.com/westerngazoo/motoreel.git motoreel
git clone https://github.com/westerngazoo/garust.git garust

cd guion
ls apps/guion-studio/Cargo.toml
ls ../motoreel/crates/motoreel/Cargo.toml
ls ../garust/Cargo.toml
ls ../garust/Cargo.toml
```

**Importante:** en el monorepo, `guion-studio` está en la rama
`cursor/guion-model-source-design` (PR #5). La rama `main` del monorepo **no**
incluye la carpeta `guion/` todavía.

### Opción B — solo guion-video-creator

Si trabajas con el repo standalone de guion, clona **motoreel** y **garust** al
mismo nivel:

```bash
mkdir -p ~/src/guion-workspace && cd ~/src/guion-workspace

git clone https://github.com/westerngazoo/guion-video-creator.git guion
git clone https://github.com/westerngazoo/motoreel.git motoreel
git clone https://github.com/westerngazoo/garust.git garust

cd guion
git checkout cursor/guion-model-source-design   # o la rama que estés probando
```

### Verificar que los hermanos están en su sitio

```bash
ls ../motoreel/crates/motoreel/Cargo.toml
ls ../garust/Cargo.toml
ls ../garust/Cargo.toml
```

Si falta alguno, `cargo build` fallará con `failed to load manifest for
dependency` y la ruta del `Cargo.toml` ausente.

### Primera compilación

Desde la raíz del workspace `guion/`:

```bash
cargo test --workspace --exclude guion-studio
```

Excluimos `guion-studio` en CI headless porque necesita WebKit. En tu máquina
con escritorio gráfico puedes incluirlo:

```bash
cargo test --workspace
```

---

## Regla de oro: trabaja desde `guion/`

Todos los comandos de esta guía asumen que tu directorio actual es la raíz del
workspace Rust (`guion/`), no `target/debug/` ni `apps/guion-studio/`.

```bash
cd guion    # o fisicobuenfisico/guion
pwd         # debe terminar en .../guion
```

Los screenplays en `templates/` se resuelven aunque invoques el binario vía
`cargo run`, pero la carpeta de salida `out/` se crea relativa al **cwd**. Si
corres desde `target/debug/`, los frames caen en `target/debug/out/` y el studio
no los encuentra.

---

## Flujo de creador (YouTube-first)

Orden recomendado para un reel nuevo. La física va **antes** del screenplay; el
studio solo orquesta lo que ya está definido en TOML.

1. **Modelo de física** en `crates/guion-fisica/` (o port desde `tools/reelNN.py`).
   Los reels de gym usan **estática cerrada** (torque, brazo de palanca, IK 2D) —
   no Lagrangiano ni integración numérica. Verifica con tests dorados:
   `cargo test -p guion-fisica`.
2. **Screenplay TOML** en `templates/` referenciando el modelo, p. ej.
   `source = "guion-fisica:reel39"` en `[[model]]`. Define objetos (`@press.elbow`),
   motion (`drive = "@press.u"`), labels y hook.
3. **Narración** (opcional): bloques `[[narration]]` en el TOML y/o
   `templates/foo.narration.txt` enlazado desde `[audio]`.
4. **Studio o CLI** (siempre desde `guion/`):
   **Check** → **Preview** → **Render** → **Narrate** → **Export MP4**.

En guion studio, el campo **Ruta screenplay** pide el archivo `.screenplay.toml`,
no un `.mp4` ni la carpeta `out/`. Atajo: clic en una plantilla de la columna
izquierda (lista `guion/templates/*.screenplay.toml`).

Ejemplo para probar reel 39:

```
templates/reel-39-shoulder-press.screenplay.toml
```

---

## CLI — pipeline completo

Plantilla de ejemplo: `templates/reel-39-shoulder-press.screenplay.toml`.

### 1. Validar screenplay

```bash
cargo run -p guion-cli -- check templates/reel-39-shoulder-press.screenplay.toml
```

Salida esperada:

```
ok: Press militar vs Smith: ¿cuál entrena el hombro? (reel-39-shoulder-press)
```

### 2. Renderizar frames PPM (1080×1920)

```bash
cargo run -p guion-cli -- render templates/reel-39-shoulder-press.screenplay.toml
```

Flags útiles:

| Flag | Efecto |
|------|--------|
| `--out DIR` | Directorio de frames (default: `out/<slug>/frames`) |
| `--fps N` | Sobrescribe el fps del screenplay |
| `--no-brand` | Sin post-fx de marca (grain, halftone) |

### 3. Generar narración WAV

Motor por defecto (`scaffold`): tonos temporizados por cue — ideal para
ajustar timing antes de conectar voz real.

```bash
cargo run -p guion-cli -- narrate templates/reel-39-shoulder-press.screenplay.toml
```

Con cama musical 8-bit mezclada en un solo WAV:

```bash
cargo run -p guion-cli -- narrate templates/reel-39-shoulder-press.screenplay.toml --with-bed
```

Con Piper (offline), cuando tengas el binario y modelo ONNX:

```bash
cargo run -p guion-cli -- narrate templates/reel-39-shoulder-press.screenplay.toml \
  --engine piper --piper-model /ruta/a/es_ES-sharvina-medium.onnx
```

### 4. Exportar MP4

```bash
cargo run -p guion-cli -- encode templates/reel-39-shoulder-press.screenplay.toml
```

Si aún no existe `out/<slug>/narration.wav`, añade `--narrate` para generarla
automáticamente:

```bash
cargo run -p guion-cli -- encode templates/reel-39-shoulder-press.screenplay.toml --narrate
```

### Salidas típicas

```
out/reel-39-shoulder-press/
├── frames/frame_00000.ppm … frame_00464.ppm
├── narration.wav
├── mixed.wav          # solo con --with-bed en narrate
└── reel.mp4
```

### Instalar el binario `guion` en PATH (opcional)

```bash
cargo install --path crates/guion-cli
guion check templates/reel-39-shoulder-press.screenplay.toml
```

---

## Screenplay — audio y narración

```toml
[audio]
generator = "fbf-default"          # cama 8-bit (Rust, sin Python)
narration_script = "templates/foo.narration.txt"  # opcional
path = "assets/custom-bed.wav"     # opcional; tiene prioridad si existe

[[narration]]
text = "Tu línea de voz en off…"
at   = { start = 0.4, end = 3.8 }
```

- `[[narration]]` define cues con timestamps en segundos.
- `guion narrate` lee esos bloques (o un `.narration.txt` con `[0.4–3.8]`).
- `guion encode --narrate` genera la narración si falta y mezcla con el
  generator configurado en `[audio]`.

---

## guion studio (Tauri)

Interfaz oscura con preview vertical, scrubber y botones **Check / Preview /
Render / Narrate / Export**. Usa los mismos crates que el CLI.

### Modo desarrollo

Desde `guion/` (macOS, Linux o Windows con escritorio):

```bash
cargo run -p guion-studio
```

- **macOS**: WebKit nativo; sin paquetes GTK (ver [macOS](#macos)).
- **Linux**: primera compilación más lenta (WebKitGTK + GTK); ver [Linux](#linux).
- **Templates**: el studio lista `guion/templates/` automáticamente; no hace
  falta configurar rutas.

### Build de distribución

Ver la sección de tu plataforma: [macOS](#4-build-de-distribución-app--dmg),
[Linux](#build-de-distribución-linux). Comandos comunes:

```bash
cargo install tauri-cli --version "^2"
cd apps/guion-studio
cargo tauri build
```

### Requisitos de la UI

- **macOS**: sesión de escritorio normal; no aplica SSH headless sin display.
- **Linux**: Wayland o X11. Sin display (SSH sin forwarding, contenedor
  headless) la ventana no abre.

---

## Motores TTS (`NarrationEngine`)

| `--engine` | Descripción |
|------------|-------------|
| `scaffold` | MVP sin deps: tonos alineados a cada cue |
| `piper` | Subprocess a `piper` + modelo ONNX |

Para añadir otro motor, implementa `guion_audio::NarrationEngine` en un crate
nuevo y regístralo en `engine_by_name`.

---

## Solución de problemas

### Carpeta `fisicobuenfisico` ya existente (no volver a clonar)

Si al seguir la guía de clonado obtuviste una cadena como:

```
fatal: destination path 'fisicobuenfisico' already exists and is not an empty directory.
zoxide: no match found
fatal: not a git repository (or any of the parent directories): .git
"apps/guion-studio/Cargo.toml": No such file or directory (os error 2)
```

**Diagnóstico:** ya tienes `~/projects/fisicobuenfisico` (o similar), pero algo falló en el
camino:

1. **`git clone` falló** porque la carpeta ya existe — no hace falta clonar de nuevo.
2. **`zoxide: no match found`** — el alias `z` (zoxide) no conoce esa ruta todavía; usa
   `cd` normal.
3. **`not a git repository`** — ejecutaste `git checkout` fuera del directorio del repo
   (p. ej. en `~/projects` o en una subcarpeta que no es la raíz del clone).
4. **`apps/guion-studio/Cargo.toml` no existe** — estás en la raíz del monorepo sin
   `cd guion`, estás en una rama sin studio (`main`), o el clone está incompleto/corrupto.

**Estructura del monorepo** (rama `cursor/guion-model-source-design`):

```
fisicobuenfisico/          ← raíz git (.git aquí)
├── guion/                 ← workspace Rust; todos los `cargo` van aquí
│   └── apps/guion-studio/Cargo.toml
├── motoreel/              ← hermano de guion (obligatorio; clone aparte, no en git)
│   └── crates/motoreel/Cargo.toml
├── garust/                ← hermano de guion (obligatorio; clone aparte, no en git)
│   └── Cargo.toml
├── physics-lab/           ← opcional (tests WASM; no es path dep de Cargo)
└── …
```

Desde la raíz del monorepo el studio está en `guion/apps/guion-studio/`, **no** en
`apps/guion-studio/` directamente bajo `fisicobuenfisico/`.

#### Recuperar un clone existente (sin volver a clonar)

Ejecuta **un comando por línea** (no pegues líneas que empiezan con `#`). Usa `cd`, no
`z`, si zoxide falla.

```bash
cd ~/projects/fisicobuenfisico
```

```bash
git status
```

```bash
git remote -v
```

```bash
git branch -a
```

Comprueba dónde está `guion` y si existe el studio:

```bash
ls guion/apps/guion-studio/Cargo.toml
```

Si ese `ls` falla, busca la carpeta `guion` en el árbol:

```bash
find . -maxdepth 3 -name Cargo.toml -path '*/guion-studio/*' 2>/dev/null
```

Actualiza refs y cambia a la rama con studio:

```bash
git fetch origin
```

```bash
git checkout cursor/guion-model-source-design
```

Si Git dice que la rama no existe localmente pero sí en `git branch -a` como
`remotes/origin/cursor/guion-model-source-design`:

```bash
git checkout -b cursor/guion-model-source-design origin/cursor/guion-model-source-design
```

Verifica **motoreel** y **garust** como hermanos de `guion` (desde la raíz del monorepo):

```bash
ls motoreel/crates/motoreel/Cargo.toml
ls garust/Cargo.toml
```

O desde `guion/`:

```bash
cd guion
```

```bash
ls ../motoreel/crates/motoreel/Cargo.toml
ls ../garust/Cargo.toml
ls ../garust/Cargo.toml
```

Si falta algún hermano (no vienen en el `git clone` de `fisicobuenfisico`):

```bash
cd ~/projects/fisicobuenfisico
```

```bash
git clone https://github.com/westerngazoo/motoreel.git motoreel
git clone https://github.com/westerngazoo/garust.git garust
```

```bash
ls motoreel/crates/motoreel/Cargo.toml
ls garust/Cargo.toml
```

Compila y abre studio (siempre desde `guion/`):

```bash
cd ~/projects/fisicobuenfisico/guion
```

```bash
ls apps/guion-studio/Cargo.toml
```

```bash
cargo run -p guion-studio
```

#### Interpretación rápida

| Síntoma | Causa probable | Qué hacer |
|---------|----------------|-----------|
| `git status` → *not a git repository* | No estás en la raíz del clone | `cd ~/projects/fisicobuenfisico` |
| `remote` no es `fisicobuenfisico` | Carpeta equivocada o renombrada | Confirma `pwd`; puede ser un clone antiguo de otro repo |
| `ls guion/...` falla en `main` | `main` no incluye `guion/` aún | `git checkout cursor/guion-model-source-design` |
| `ls guion/apps/guion-studio/...` OK en raíz pero `cargo` falla | cwd es la raíz, no `guion/` | `cd guion` antes de `cargo run` |
| `motoreel/...` o `garust/...` no existe | Repo hermano no clonado | Ver [Setup completo](#setup-completo--clones-obligatorios) |

#### Si el clone está corrupto o irreparable

Solo entonces renombra y clona de nuevo:

```bash
cd ~/projects
```

```bash
mv fisicobuenfisico fisicobuenfisico.bak
```

```bash
git clone https://github.com/westerngazoo/fisicobuenfisico.git
```

```bash
cd fisicobuenfisico
```

```bash
git fetch origin
```

```bash
git checkout cursor/guion-model-source-design
```

```bash
cd guion
```

```bash
ls apps/guion-studio/Cargo.toml
```

```bash
ls ../motoreel/crates/motoreel/Cargo.toml
ls ../garust/Cargo.toml
```

```bash
cargo run -p guion-studio
```

### Clone antiguo en `master` (sin `main` ni studio)

Si en `~/projects/guion` ejecutaste algo como:

```bash
git fetch origin && git checkout main && git pull
ls apps/guion-studio/Cargo.toml && cargo run -p guion-studio
```

y obtuviste:

- `error: pathspec 'main' did not match any file(s) known to git`
- `apps/guion-studio/Cargo.toml: No such file or directory`

**Diagnóstico:** casi seguro tienes un clone **antiguo** del repo standalone
[`guion-video-creator`](https://github.com/westerngazoo/guion-video-creator) (o
un `guion` viejo) en la rama `master`. Ese repo solo tiene docs/fixtures; **no**
tiene `apps/guion-studio`. `guion-studio` vive en el
[monorepo fisicobuenfisico](https://github.com/westerngazoo/fisicobuenfisico)
bajo `guion/apps/guion-studio/` en la rama `cursor/guion-model-source-design`.

**No pegues líneas que empiezan con `#` en la terminal** — en zsh/bash son
comentarios y verás `zsh: command not found: #` si copias una línea de comentario
de esta guía.

#### Paso 0 — comprobar qué tienes

```bash
cd ~/projects/guion
pwd
git remote -v
git branch -a
ls apps/guion-studio/Cargo.toml
```

Interpretación rápida:

| `git remote -v` | `git branch -a` | Qué tienes |
|---------------|-----------------|------------|
| `guion-video-creator` | solo `master` | Opción A o B (abajo) |
| `fisicobuenfisico` | `main` pero sin `guion/` en cwd | Estás en la raíz del monorepo, no en `guion/` — Opción C |
| `fisicobuenfisico` | `cursor/guion-model-source-design` | Opción C — solo falta `cd guion` y clonar hermanos (motoreel, garust) |

#### Opción A — clonar el monorepo (recomendado)

```bash
cd ~/projects
git clone https://github.com/westerngazoo/fisicobuenfisico.git
cd fisicobuenfisico
git fetch origin
git checkout cursor/guion-model-source-design
git clone https://github.com/westerngazoo/motoreel.git motoreel
git clone https://github.com/westerngazoo/garust.git garust
cd guion
ls apps/guion-studio/Cargo.toml
ls ../motoreel/crates/motoreel/Cargo.toml
ls ../garust/Cargo.toml
ls ../garust/Cargo.toml
cargo run -p guion-studio
```

#### Opción B — reutilizar la carpeta `~/projects/guion`

Si quieres conservar la ruta `~/projects/guion`:

```bash
cd ~/projects
mv guion guion-old-backup
git clone https://github.com/westerngazoo/fisicobuenfisico.git fisicobuenfisico-tmp
cd fisicobuenfisico-tmp
git fetch origin
git checkout cursor/guion-model-source-design
cd ..
ln -s fisicobuenfisico-tmp/guion guion
cd guion
ls apps/guion-studio/Cargo.toml
ls ../motoreel/crates/motoreel/Cargo.toml
ls ../garust/Cargo.toml
ls ../garust/Cargo.toml
cargo run -p guion-studio
```

O añade el monorepo como segundo remote en tu carpeta existente:

```bash
cd ~/projects/guion
git remote add fisicobuenfisico https://github.com/westerngazoo/fisicobuenfisico.git
git fetch fisicobuenfisico
git checkout -b cursor/guion-model-source-design fisicobuenfisico/cursor/guion-model-source-design
```

Eso **reemplaza** el contenido del directorio de trabajo por la rama del
monorepo. Si faltan hermanos al lado de `guion/`:

```bash
cd ~/projects/fisicobuenfisico
git clone https://github.com/westerngazoo/motoreel.git motoreel
git clone https://github.com/westerngazoo/garust.git garust
cd guion
ls ../motoreel/crates/motoreel/Cargo.toml
ls ../garust/Cargo.toml
ls ../garust/Cargo.toml
```

#### Opción C — tu remote ya es fisicobuenfisico

```bash
cd ~/projects/guion
git remote -v
git fetch origin
git branch -a
git checkout cursor/guion-model-source-design
cd guion
ls apps/guion-studio/Cargo.toml
ls ../motoreel/crates/motoreel/Cargo.toml
ls ../garust/Cargo.toml
ls ../garust/Cargo.toml
cargo run -p guion-studio
```

Si clonaste el monorepo entero pero tu shell está en la raíz
(`fisicobuenfisico/`), el path correcto es `guion/apps/guion-studio/Cargo.toml`
desde ahí, o `cd guion` antes de `cargo run`.

### `package(s) guion-studio not found in workspace`

Cargo muestra algo como:

```
error: package(s) `guion-studio` not found in workspace `/Users/…/guion`
```

**Causa:** estás en una copia de `guion` que aún no incluye `apps/guion-studio`
— típicamente `master` en un clone local antiguo, o el repo standalone
[`guion-video-creator`](https://github.com/westerngazoo/guion-video-creator) (solo
docs/fixtures, sin código Rust). `guion-studio` vive en el
[monorepo fisicobuenfisico](https://github.com/westerngazoo/fisicobuenfisico)
bajo `guion/apps/guion-studio` en la rama
`cursor/guion-model-source-design` (PR #5). Ver también
[Clone antiguo en `master`](#clone-antiguo-en-master-sin-main-ni-studio).

**Verificar** (desde la raíz del repo donde corres `cargo`):

```bash
ls apps/guion-studio/Cargo.toml
grep guion-studio Cargo.toml
```

Si `ls` falla o `grep` no devuelve `"apps/guion-studio"`, esa copia no tiene
studio.

#### Fix A — monorepo fisicobuenfisico (recomendado)

Si clonaste el monorepo pero estás en una rama vieja:

```bash
cd ~/projects/fisicobuenfisico
git fetch origin
git checkout cursor/guion-model-source-design

cd guion
ls apps/guion-studio/Cargo.toml
grep guion-studio Cargo.toml
ls ../motoreel/crates/motoreel/Cargo.toml
ls ../garust/Cargo.toml
cargo run -p guion-studio
```

#### Fix B — solo tienes un directorio `guion` en `master`

El workspace Rust completo (CLI + studio) está en el monorepo. Opciones:

1. **Clonar el monorepo** y los hermanos **motoreel** + **garust**:

```bash
cd ~/projects
git clone https://github.com/westerngazoo/fisicobuenfisico.git
cd fisicobuenfisico
git clone https://github.com/westerngazoo/motoreel.git motoreel
git clone https://github.com/westerngazoo/garust.git garust
cd guion
ls ../motoreel/crates/motoreel/Cargo.toml
ls ../garust/Cargo.toml
ls ../garust/Cargo.toml
cargo run -p guion-studio
```

2. **Reutilizar tu carpeta `guion`** apuntando al monorepo (conserva el path
   `/Users/goose/projects/guion`):

```bash
cd ~/projects
mv guion guion-old-backup
git clone https://github.com/westerngazoo/fisicobuenfisico.git fisicobuenfisico
ln -s fisicobuenfisico/guion guion
cd guion
cargo run -p guion-studio
```

3. **Si tu `guion` es un clone de fisicobuenfisico mal nombrado**, actualiza rama:

```bash
cd ~/projects/guion
git remote -v
git fetch origin
git branch -a
git checkout cursor/guion-model-source-design
ls apps/guion-studio/Cargo.toml
ls ../motoreel/crates/motoreel/Cargo.toml
ls ../garust/Cargo.toml
ls ../garust/Cargo.toml
cargo run -p guion-studio
```

**Nota:** el repo `guion-video-creator` en GitHub no contiene el crate
`guion-studio`; no uses `cargo run -p guion-studio` ahí hasta que se publique un
mirror del workspace completo.

### `failed to load manifest for dependency motoreel` / `garust`

Cargo no encuentra un `Cargo.toml` de path dependency — el repo hermano no está
clonado al lado de `guion/` (no vienen en el `git clone` de `fisicobuenfisico`).

Ejemplo típico:

```
failed to load manifest for dependency `garust`
failed to read `…/garust/Cargo.toml`: No such file or directory
```

**Fix inmediato** (desde la raíz del monorepo, p. ej. `~/projects/fisicobuenfisico`):

```bash
cd ~/projects/fisicobuenfisico
git clone https://github.com/westerngazoo/motoreel.git motoreel
git clone https://github.com/westerngazoo/garust.git garust
ls motoreel/crates/motoreel/Cargo.toml
ls garust/Cargo.toml
cd guion
cargo check --workspace --exclude guion-studio
```

O desde `guion/`:

```bash
./scripts/bootstrap-siblings.sh
cargo check --workspace --exclude guion-studio
```

### Los frames o el MP4 no aparecen donde espero

Probablemente ejecutaste el comando fuera de `guion/`. Vuelve a la raíz del
workspace y repite:

```bash
cd /ruta/a/guion
cargo run -p guion-cli -- render templates/reel-39-shoulder-press.screenplay.toml
ls out/reel-39-shoulder-press/frames/ | head
```

### `guion: no encuentra el screenplay`

Pasa una ruta relativa a `guion/` o una absoluta. El CLI también busca en el
workspace si no encuentra el archivo en el cwd.

### Errores de WebKit/GTK al compilar `guion-studio` (Linux)

Instala las dependencias de la sección [Linux → Dependencias de sistema](#dependencias-de-sistema-para-guion-studio). En Ubuntu 24.04 asegúrate de usar
`libwebkit2gtk-4.1-dev`. En **macOS** no uses paquetes GTK; solo Xcode CLT +
Homebrew (ver [macOS](#macos)).

### Gatekeeper no deja abrir `guion-studio.app` (macOS)

Clic derecho en la app → **Abrir**, o quita el atributo de cuarentena:

```bash
xattr -cr path/to/guion-studio.app
```

### Avisos EGL / GPU (Linux; no aplica en macOS)

Al ejecutar Tauri sin GPU o sin display en **Linux** verás mensajes como:

```
libEGL warning: failed to open /dev/dri/renderD128
```

En un escritorio normal son inofensivos. En un servidor sin gráficos, usa solo el
CLI (`guion-cli`); el studio necesita ventana. En macOS estos avisos no aparecen.

### `zoxide: command not found` en macOS

Al abrir Terminal o ejecutar `source ~/.zshrc` ves algo como:

```
zoxide: command not found
```

(o lo lees mal como **«xoxide not found»** — no existe ningún comando `xoxide` en
guion ni en el build de Rust/Tauri).

**Causa:** tu `~/.zshrc` (u otro dotfile de shell) inicializa **zoxide**, un
plugin opcional de Homebrew para saltar entre directorios. No forma parte de
guion-studio; solo aparece porque el shell carga esa config antes de que corras
`cargo run -p guion-studio`.

**Opción A — instalar zoxide** (si lo quieres usar):

```bash
brew install zoxide
echo 'eval "$(zoxide init zsh)"' >> ~/.zshrc
source ~/.zshrc
```

**Opción B — ignorarlo** (suficiente para guion):

Comenta o borra la línea de zoxide en `~/.zshrc` (suele ser
`eval "$(zoxide init zsh)"`):

```bash
nano ~/.zshrc   # añade # al inicio de la línea de zoxide
source ~/.zshrc
```

Si el mensaje menciona **xcode** o **xcodebuild** (no «xoxide»), instala las
Command Line Tools: `xcode-select --install` (ver [macOS → Xcode CLT](#1-xcode-command-line-tools)).

### `ffmpeg` no encontrado en `guion encode`

```bash
# macOS
which ffmpeg || brew install ffmpeg

# Linux
which ffmpeg || sudo apt install ffmpeg
```

### Compilación lenta en el primer `render`

El primer `cargo run` compila todo el workspace (motoreel, assemble, brand…).
Los siguientes renders solo reutilizan el binario ya compilado (~1 minuto de
render para ~465 frames en debug).

---

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

---

## Referencias

- Visión del framework: [RFC-0001](RFC-0001-guion-framework.md)
- CI del monorepo: `.github/workflows/guion-ci.yml` (checkout de motoreel)
- Plantillas: `guion/templates/`
