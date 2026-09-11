# Números dorados

`dorados.json` — **135 muestras de 12 piezas ya publicadas** de
@fisicobuenfisico. Generado por `fisicobuenfisico/tools/fixtures.py`; no
editar a mano.

Cada pieza trae:

- `constantes` — los parámetros del modelo tal como están publicados
  (carga, estatura, geometría de la máquina, `TARGET`, `MEDICIONES`).
- `muestras` — `entrada → salida` de su propia función. Según la pieza:
  `pose` (posiciones articulares), `fuerza_unitaria` (dirección de la línea
  de acción), `hombro` / `codo` / `cadera` / `rodilla` / `l5s1` (torques en
  N·m), ángulos en grados, `trabajo`, `recorrido`, y
  `trabajo_musculos` vs `trabajo_carga` (balance de energía).
- `descripcion`, `tema`.

**Las convenciones no están homologadas a propósito.** Cada pieza mide
desde su propio cero, y descubrir eso es parte del trabajo del motor
nuevo. Si se normalizara al extraer, el fixture dejaría de probar nada.

**Unidades:** torque N·m, fuerza N, largos m, ángulos grados (salvo donde
el nombre diga otra cosa), trabajo J.

**Tolerancia: 2 %** (R-0001 Q5). No se sube.

## Regenerar

```bash
cd fisicobuenfisico && .venv/bin/python3 tools/fixtures.py
cp fixtures/dorados.json ../guion-video-creator/fixtures/
```

Cada pieza corre en su propio proceso porque el tema de color se resuelve
al importar y en un proceso sólo cabe uno.
