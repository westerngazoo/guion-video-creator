# Flujos — guion-video-creator

## Flujo principal

```mermaid
flowchart TB
TOML[screenplay TOML] --> GUION[guion workspace]
GUION --> MR[motoreel frames]
MR --> FF[ffmpeg mp4]
FIX[dorados.json] -->|2% gate| FF
PHY[physics-lab + garust] --> GUION
```

## Descripción paso a paso

1. **Spec** — ENCARGO.md defines mandate, decisions, acceptance criteria.
1. **Physics** — FISICA.md — exact physics to reproduce, not reinvent.
1. **Verify** — dorados.json golden samples, 2% tolerance gate.
1. **Pipeline** — guion-cli check → assemble → motoreel → ffmpeg → mp4.

## Diagrama PlantUML

Equivalente PlantUML del flujo principal (misma topología que el diagrama Mermaid):

```plantuml
@startuml
title guion-video-creator — flujo principal
note as N1
Ver flows.md Mermaid para detalle;
exportar con herramientas mermaid→plantuml si se prefiere editar en PlantUML.
end note
@enduml
```

## Estados y casos borde

Consulta los tests de integración y los RFC/requirements del proyecto para flujos de error y recuperación.
