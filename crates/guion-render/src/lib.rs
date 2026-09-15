//! La comparación, dibujada.
//!
//! Aquí está la diferencia con animar matemáticas. En una herramienta de
//! animación la curva se mueve porque el autor escribió la solución en la
//! animación: **la animación ES la fórmula**, y si el autor se equivocó el
//! video sale precioso e igual de equivocado. Este módulo no sabe qué
//! forma tiene ninguna curva. Le pregunta a la
//! [`Comparacion`](guion_comparacion::Comparacion) —que le pregunta a
//! `mecanica`— y dibuja lo que le contesten.
//!
//! El resultado: **el dibujo no puede contradecir a la física, porque no
//! tiene su propia copia de la física.**
//!
//! Lo que hay hoy es la gráfica: los ejes, una curva por opción sobre el
//! eje compartido de progreso, y el cruce marcado cuando lo hay. Sin
//! tipografía todavía, y esa ausencia es una decisión pendiente y no un
//! olvido — ver `TIPOGRAFIA` abajo.

#![forbid(unsafe_code)]

use guion_comparacion::Comparacion;
use garust::{pga, Motor3};
use motoreel::{Camera, Object, Rgb, Scene, Shape, Style, Track};

/// **Por qué todavía no hay texto.**
///
/// La cara tipográfica de motoreel es un mapa de bits de 5×7 que cubre
/// ASCII imprimible y nada más. Un reel de esta marca necesita `τ`, `·`,
/// `°`, `¿`, `á` y `ñ` — y [`motoreel::Label`] normaliza a ASCII al
/// evaluar, así que no los pierde de forma visible: los muerde. Además
/// una cara de 5×7 no se parece a la tipografía de display con la que
/// está construida la marca.
///
/// Son dos caminos y la decisión es del dueño, no mía: ampliar la tabla
/// de mapa de bits a Latin-1 más los símbolos (barato, cambia el look), o
/// meterle a motoreel un primitivo de texto con fuente real (más trabajo,
/// conserva la marca). Mientras tanto la geometría —que es lo que la
/// física decide— ya se puede comprobar.
pub const TIPOGRAFIA: &str = "pendiente: ver la nota de este módulo";

/// La ventana de imagen, centrada en el origen.
pub const VISTA: (f64, f64) = (2.0, 1.2);

/// La escena vive en `z = −1`: la cámara mira a lo largo de −z y descarta
/// lo que esté EN su plano, así que un dibujo plano en z = 0 sale negro y
/// sin un solo error. Costó un cuadro en blanco averiguarlo.
const PROFUNDIDAD: f64 = -1.0;

/// El papel de la marca, para que el fondo no sea negro de fábrica.
pub const PAPEL: Rgb = Rgb { r: 0xe8, g: 0xe2, b: 0xd4 };

fn punto(x: f64, y: f64) -> pga::Point {
    pga::Point::new(x, y, PROFUNDIDAD)
}

fn trazo(color: Rgb, ancho: f64) -> Style {
    Style { stroke: color, width: ancho, alpha: 1.0 }
}

/// Los colores de una opción, por orden de declaración.
///
/// El primero es la tinta y el segundo el acento, igual que en las piezas
/// publicadas: la opción 0 se lee como "la de referencia" y la 1 como "la
/// que se compara". Más de dos y se reparten tonos intermedios, porque
/// una comparación de tres no es más difícil de dibujar — es más difícil
/// de leer, y eso lo decide quien escribe el guion.
fn color_de(i: usize, n: usize) -> Rgb {
    const TINTA: Rgb = Rgb { r: 0x1a, g: 0x18, b: 0x14 };
    const ACENTO: Rgb = Rgb { r: 0xc4, g: 0x45, b: 0x1e };
    if i == 0 {
        return TINTA;
    }
    if i + 1 == n {
        return ACENTO;
    }
    let t = i as f64 / (n - 1).max(1) as f64;
    Rgb {
        r: (TINTA.r as f64 + (ACENTO.r as f64 - TINTA.r as f64) * t) as u8,
        g: (TINTA.g as f64 + (ACENTO.g as f64 - TINTA.g as f64) * t) as u8,
        b: (TINTA.b as f64 + (ACENTO.b as f64 - TINTA.b as f64) * t) as u8,
    }
}

/// Cómo se mapea un punto `(progreso, τ)` a la imagen.
struct Ejes {
    tau_max: f64,
}

impl Ejes {
    /// La escala vertical sale del **pico más alto de la comparación**, no
    /// de un número elegido a mano: una gráfica cuya escala no depende de
    /// lo que grafica miente por omisión el día que alguien cambia la
    /// carga.
    fn de(c: &Comparacion, n: usize) -> Self {
        let mut tau_max: f64 = 0.0;
        for i in 0..c.opciones.len() {
            for (_, t) in c.curva(i, n) {
                tau_max = tau_max.max(t.abs());
            }
        }
        // 10% de aire arriba para que el pico no toque el marco
        Ejes { tau_max: if tau_max > 0.0 { tau_max * 1.10 } else { 1.0 } }
    }

    fn en(&self, s: f64, tau: f64) -> pga::Point {
        punto(
            -VISTA.0 / 2.0 * 0.86 + s * VISTA.0 * 0.86,
            -VISTA.1 / 2.0 * 0.80 + (tau / self.tau_max) * VISTA.1 * 0.80,
        )
    }
}

/// La escena de la gráfica: ejes, una curva por opción, y el cruce.
///
/// `hasta` es cuánto de la repetición se ha trazado, de 0 a 1 — la curva
/// se dibuja **con** el movimiento y no aparece hecha, que es lo que hace
/// que se lea como una medición y no como un adorno.
#[must_use]
pub fn escena_grafica(c: &Comparacion, hasta: f64, muestras: usize) -> Scene {
    let ejes = Ejes::de(c, muestras);
    let mut objects = Vec::new();

    // el marco: sólo los dos ejes, que es lo que se usa para leer
    let gris = Rgb { r: 0x78, g: 0x6e, b: 0x5e };
    objects.push(Object {
        shape: Shape::Polyline(vec![
            ejes.en(0.0, ejes.tau_max),
            ejes.en(0.0, 0.0),
            ejes.en(1.0, 0.0),
        ]),
        style: trazo(gris, 0.006),
        track: Track::hold(Motor3::identity()),
    });

    let n = c.opciones.len();
    for i in 0..n {
        let pts: Vec<pga::Point> = c
            .curva(i, muestras)
            .into_iter()
            .filter(|(s, _)| *s <= hasta + 1e-12)
            .map(|(s, t)| ejes.en(s, t))
            .collect();
        if pts.len() >= 2 {
            objects.push(Object {
                shape: Shape::Polyline(pts.clone()),
                style: trazo(color_de(i, n), 0.014),
                track: Track::hold(Motor3::identity()),
            });
            // el punto que marca dónde va la animación
            objects.push(Object {
                shape: Shape::Point(*pts.last().expect("no vacío")),
                style: trazo(color_de(i, n), 0.03),
                track: Track::hold(Motor3::identity()),
            });
        }
    }

    // el cruce, cuando el trazo ya llegó a él: suele ser el hallazgo
    if n >= 2 {
        if let Some(s) = c.cruce(0, 1) {
            if s <= hasta {
                let tau = c.valor(0, guion_comparacion::Medida::EnProgreso(s));
                objects.push(Object {
                    shape: Shape::Segment(ejes.en(s, 0.0), ejes.en(s, tau)),
                    style: trazo(Rgb { r: 0xe0, g: 0x8a, b: 0x3c }, 0.008),
                    track: Track::hold(Motor3::identity()),
                });
            }
        }
    }

    Scene {
        objects,
        camera: Camera::orthographic(Motor3::identity()),
        duration: 1.0,
        labels: Vec::new(),
        view: VISTA,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use guion_comparacion::guion;

    fn comparacion() -> Comparacion {
        let ruta = concat!(env!("CARGO_MANIFEST_DIR"),
                           "/../../guiones/reel40-gluteo.toml");
        guion::cargar(&std::fs::read_to_string(ruta).expect("guion")).expect("carga")
    }

    /// **Lo dibujado es lo calculado.** Se toma un punto de la polilínea y
    /// se deshace el mapeo: tiene que dar el τ que el modelo dice para ese
    /// progreso. Si el dibujo tuviera su propia copia de la curva, esto
    /// fallaría — y ésa es toda la tesis del proyecto en un test.
    #[test]
    fn la_curva_dibujada_es_la_que_calcula_el_modelo() {
        let c = comparacion();
        let n = 200;
        let ejes = Ejes::de(&c, n);
        let escena = escena_grafica(&c, 1.0, n);
        // el primer objeto es el marco; el segundo, la curva de la opción 0
        let Shape::Polyline(pts) = &escena.objects[1].shape else {
            panic!("la opción 0 no es una polilínea");
        };
        for (k, p) in pts.iter().enumerate().step_by(37) {
            let (_, y, _) = p.to_euclidean();
            // deshacer el mapeo vertical
            let tau_dibujado = (y + VISTA.1 / 2.0 * 0.80) / (VISTA.1 * 0.80)
                * ejes.tau_max;
            let s = k as f64 / (pts.len() - 1) as f64;
            let tau_modelo = c.valor(0, guion_comparacion::Medida::EnProgreso(s));
            assert!(
                (tau_dibujado - tau_modelo).abs() < 0.5,
                "en s={s:.2} se dibujó {tau_dibujado:.1} y el modelo dice \
                 {tau_modelo:.1}"
            );
        }
    }

    /// La escena va DELANTE de la cámara. Un dibujo plano en z = 0 cae en
    /// el plano de la cámara y se descarta entero, sin un solo error: el
    /// primer render salió negro por esto.
    #[test]
    fn la_escena_no_esta_en_el_plano_de_la_camara() {
        let escena = escena_grafica(&comparacion(), 1.0, 40);
        for o in &escena.objects {
            if let Shape::Polyline(pts) = &o.shape {
                for p in pts {
                    let (_, _, z) = p.to_euclidean();
                    assert!(z < -1e-9, "z = {z}: la cámara lo descartaría");
                }
            }
        }
    }

    /// La escala vertical sale del pico de la comparación, no de un número
    /// elegido a mano: si alguien sube la carga, la gráfica sigue cabiendo.
    #[test]
    fn la_escala_sigue_a_los_datos() {
        let c = comparacion();
        let ejes = Ejes::de(&c, 200);
        let pico = (0..c.opciones.len())
            .map(|i| c.valor(i, guion_comparacion::Medida::Pico))
            .fold(0.0_f64, f64::max);
        assert!(ejes.tau_max > pico, "el pico tiene que caber");
        assert!(ejes.tau_max < pico * 1.5, "y no sobrar media gráfica");
    }
}
