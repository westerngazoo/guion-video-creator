# Recorrido del código — guion-video-creator

Guía orientada a desarrolladores para entender dónde vive cada responsabilidad.

### 1. Spec

ENCARGO.md defines mandate, decisions, acceptance criteria.

### 2. Physics

FISICA.md — exact physics to reproduce, not reinvent.

### 3. Verify

dorados.json golden samples, 2% tolerance gate.

### 4. Pipeline

guion-cli check → assemble → motoreel → ffmpeg → mp4.

## Punto de entrada recomendado

Empieza por el README del proyecto y el módulo/crate principal listado en la documentación de arquitectura.
