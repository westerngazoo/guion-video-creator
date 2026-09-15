# Flujos — guion-video-creator

## Flujo principal (Mermaid)

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
2. **Physics** — FISICA.md — exact physics to reproduce, not reinvent.
3. **Verify** — dorados.json golden samples, 2% tolerance gate.
4. **Pipeline** — guion-cli check → assemble → motoreel → ffmpeg → mp4.

## Secuencia (PlantUML)

Fuente: [`diagrams/flow-sequence.puml`](./diagrams/flow-sequence.puml)

```plantuml
@startuml
title guion-video-creator — secuencia principal

participant "Spec" as Spec0
participant "Physics" as Physics1
participant "Verify" as Verify2
participant "Pipeline" as Pipeline3

Spec0 -> Physics1: FISICA.md — exact physics to reproduce, not reinvent.
Physics1 -> Verify2: dorados.json golden samples, 2% tolerance gate.
Verify2 -> Pipeline3: guion-cli check → assemble → motoreel → ffmpeg → mp4.

@enduml
```

## Componentes / estados (PlantUML)

Fuente: [`diagrams/flow-architecture.puml`](./diagrams/flow-architecture.puml)

```plantuml
@startuml
title guion-video-creator — flujo de componentes
start
:TOMLscreenplay;
:GUIONguion;
:GUION;
:MRmotoreel;
:MR;
:FFffmpeg;
:FIXdorados.json;
:FF;
:PHYphysics-lab;
stop

@enduml
```

## Estados y casos borde

Consulta los tests de integración y los RFC/requirements del proyecto para flujos de error y recuperación.
