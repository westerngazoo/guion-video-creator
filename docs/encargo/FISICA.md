# La física, exactamente como se calcula hoy

Esto no es una propuesta: es lo que está publicado. El motor nuevo tiene
que **reproducirlo**, no mejorarlo. Si algo aquí parece mal, dilo antes de
cambiarlo — cada número de estos ya salió en un video.

## 1. El modelo

- **2D, plano sagital.** Nada de rotación fuera del plano.
- **Cuasi-estático.** Las repeticiones son lentas; no hay término
  inercial. Lo que se calcula es el momento que la articulación tiene que
  **balancear**, no el que la acelera.
- **Eslabones rígidos**: tibia, fémur, tronco, húmero, antebrazo. Largos
  por fracción de estatura; masas por fracción de masa corporal
  (fracciones segmentales de Winter). Las dos piernas o los dos brazos se
  suman en un eslabón.
- **Articulaciones bilaterales sumadas en una.** El τ que se imprime es el
  total, no por lado.

## 2. La pose

Un ejercicio es **un camino de la carga** parametrizado por `u ∈ [0, 1]`:
`u=0` es el inicio de la repetición, `u=1` el cierre. La pose se obtiene
por cinemática inversa desde ese camino.

- **IK por intersección de dos círculos** (hombro→codo→mano, o
  cadera→rodilla→tobillo).
- **La rama se elige con un vector de dirección explícito**, nunca con
  una regla implícita tipo "la solución más alta". Ese fue un defecto
  real: en el remo con barra la mano cuelga justo debajo del hombro, la
  línea hombro–mano es vertical, las dos soluciones son espejo con la
  misma altura, y la que salía era ruido numérico — el codo se veía al
  revés. **Portar esto como parámetro, no como heurística.**
- **Diferencias de ángulo normalizadas, no los ángulos.**
  `(b − a + π) mod 2π − π`. Normalizar el valor en vez de la diferencia
  infló el trabajo de rodilla de 134 J a 571 J en una versión anterior.

## 3. Las cargas

Cada carga externa es `(F⃗, punto de aplicación)`:

- **Peso** (carga y segmentos): vertical, `(0, −m·g)`, en el centro de
  masa del segmento.
- **Cable**: a lo largo de la **línea de acción** hacia la polea,
  `F · û(polea − mano)`. No es vertical y eso es justamente lo que cambia
  el resultado.

## 4. Torque

```
τ(pivote) = | Σ (r_i − pivote) × F⃗_i |     sobre todo lo DISTAL al pivote
```

Con signo para integrar; en valor absoluto para imprimir.

Qué entra en cada articulación:

| articulación | qué se cuenta |
|---|---|
| codo | sólo la carga |
| hombro | sólo la carga |
| cadera, L5/S1 | la carga **y** el cuerpo que cuelga (tronco, cabeza, brazos) |

El peso del propio brazo pone ≤20 N·m en hombro y codo, y en la polea
**cambia de signo** a media repetición (al arrancar el brazo quiere caer,
o sea ayuda): meterlo hace que la curva toque cero sin que la carga haga
nada. Se excluye y **se dice en el caption**. En cadera y L5/S1 no se
puede excluir: ahí el cuerpo es el 36 % del total, y ése es el dato.

El momento se suma **segmento por segmento**. Meter tronco + cabeza +
brazos como una sola masa al centro de masa del tronco subestimaba un
11 % (399 N·m salieron 360): la cabeza cae más allá del hombro y los
brazos cuelgan *del* hombro, los dos mucho más lejos del eje.

## 5. Trabajo

```
W = F × (lo que la carga viaja EN SU línea de acción)
```

- barra: lo que sube (vertical).
- cable: lo que se acorta o alarga el tramo polea–mano.

Y se comprueba contra la integral articular (§6). Un área sombreada en
una gráfica **sólo** se pinta si se verificó que es trabajo: torque contra
altura da N·m², no joules, y eso ya se dibujó mal una vez.

## 6. Balance de energía — la prueba fuerte

```
W_músculos = − ∮ [ τ_hombro·dφ_hombro + τ_codo·d(φ_codo − φ_hombro) ]
```

tiene que dar el `W` de §5, dentro del 0.5 %. Cierra sólo si la
cinemática, los torques y el recorrido cuentan la misma historia; es la
comprobación que ninguna de las tres pasa sola.

## 7. Lo que medimos, y nada más

Por cada opción comparada:

| | qué es | unidad |
|---|---|---|
| **τ** | torque en la articulación **objetivo** | N·m |
| **W** | trabajo por repetición | J |
| **P̄** | potencia media `W/t` — *sólo* si el reel es de tempo | W |

La **articulación objetivo** es la que el ejercicio entrena: curl → codo,
peso muerto → cadera, remo y press → hombro. Es la que va viva en pantalla
y **tiene que moverse con la repetición**. Un τ fijo en pantalla, aunque
sea correcto, no cuenta nada — el reel del remo medía la espalda baja (el
*costo*, no el objetivo) y no se movía: «se quedó fijo, no hay algo
dinámico». El costo se cuenta aparte.

## 8. Lo que NO calculamos

Y no se insinúa que sí:

- **Fuerza muscular.** Necesita brazos de momento anatómicos con fuente.
- **Activación.** Eso es EMG, no mecánica. Un renglón que diga "mayor
  activación de dorsal" es mentira; el que sí se puede escribir es "carga
  en el hombro, que es lo que jala el dorsal".
- **Dinámica** (inercia, aceleración), **3D**, **estabilidad**, y qué
  construye más músculo.

## 9. Supuestos de máquina

Dónde está la polea, cuánto se inclina el torso, dónde toca la barra: eso
no es física, es la máquina y la técnica. Van **declarados como constantes
del módulo**, **escritos en el caption**, y **con prueba de sensibilidad**:
se mueven ±20 cm y lo que se afirma en pantalla tiene que seguir siendo
cierto. Si no lo es, la afirmación estaba de más.
