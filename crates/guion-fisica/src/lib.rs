//! `guion-fisica` — el puente entre los valores dorados y el motor.
//!
//! **Aquí no se hace física.** Cada función toma una muestra dorada, la
//! traduce a una llamada de `mecanica` y devuelve lo que el motor
//! contesta. Si algo no cuadra, el que está mal es el motor o el dorado,
//! y las dos cosas son noticias útiles.
//!
//! No siempre fue así. Este crate tuvo un archivo por reel con su propia
//! `G`, su propio producto cruz y su propia geometría, y sólo una de las
//! cuatro copias era física de verdad: el jalón era un ajuste de tres
//! coeficientes y el press una tabla de cuatro nudos interpolados, los
//! dos ajustados a estos mismos valores dorados. Pasaban la prueba al 2 %
//! por construcción. Un ajuste a los dorados pasa los dorados siempre,
//! así que la compuerta estaba midiendo cero.
//!
//! Lo que se perdió al quitarlos no fue nada: las constantes que parecían
//! irreducibles se derivan todas. El arranque del jalón, que vivía como
//! `1.080_211_278_642_769`, es el brazo al 97 % de extensión proyectado
//! desde el hombro; el tope de la Smith, que vivía como `alcance - 0.008`,
//! es `sqrt(alcance² − x²)`.

mod dorados;

pub use dorados::{
    default_dorados_path, load as load_dorados, within_tol, Dorados, Muestra, Pieza,
};

use mecanica::curl::{Agarre, Curl};
use mecanica::jalon::Jalon;
use mecanica::maquina_humana::Persona;
use mecanica::press::{Press, Variante};
use mecanica::Lift;

/// Tolerancia de la casa contra los dorados (ENCARGO §3).
pub const DORADOS_TOL: f64 = 0.02;

/// Las piezas que ya pasan por el motor.
pub const CON_PUENTE: &[&str] = &["reel04", "reel26", "reel29", "reel39"];

/// Una salida con nombre: la que el dorado trae y el motor tiene que dar.
pub type Salida = (&'static str, f64);

/// Evalúa una muestra con el motor y devuelve sus salidas con nombre.
///
/// # Errors
/// Si la pieza no tiene puente o la muestra no trae sus entradas.
pub fn evalua(pieza: &str, m: &Muestra) -> Result<Vec<Salida>, String> {
    match pieza {
        "reel04" => reel04(m),
        "reel26" => reel26(m),
        "reel29" => reel29(m),
        "reel39" => reel39(m),
        otro => Err(format!("pieza sin puente al motor: {otro}")),
    }
}

// ------------------------------------------------------------- reel 04
fn reel04(m: &Muestra) -> Result<Vec<Salida>, String> {
    let phi = num(m, "phi_grados")?.to_radians();
    let agarre = match texto(m, "modo")?.as_str() {
        "barra" => Agarre::Barra,
        "polea" => Agarre::Polea((0.42, -0.85)),
        otro => return Err(format!("modo desconocido: {otro}")),
    };
    let curl = Curl {
        carga_kg: 20.0,
        antebrazo_m: 0.32,
        agarre,
        rango_rad: (0.0, core::f64::consts::PI),
    };
    Ok(vec![("tau", curl.tau(phi))])
}

// ------------------------------------------------------- reels 26 y 29
/// El jalón publicado. Un solo modelo para las dos piezas, porque es un
/// solo ejercicio: el 26 lo mira al cierre, el 29 a lo largo del tirón.
fn jalon(beta_grados: f64) -> Jalon {
    Jalon {
        carga_kg: 50.0,
        torso_m: 0.55,
        humero_m: 0.32,
        antebrazo_m: 0.28,
        esternon_fwd: 0.24,
        esternon_up: 0.15,
        polea: (0.24, 1.60),
        reclinado_rad: beta_grados.to_radians(),
        extension: 0.97,
    }
}

fn reel26(m: &Muestra) -> Result<Vec<Salida>, String> {
    // El reel 26 mide el CIERRE del jalón, que es `u = 1`.
    Ok(vec![("tau", jalon(num(m, "beta_grados")?).tau(1.0))])
}

fn reel29(m: &Muestra) -> Result<Vec<Salida>, String> {
    let j = jalon(num(m, "beta_grados")?);
    if let Ok(medida) = texto(m, "medida") {
        return match medida.as_str() {
            "recorrido" => Ok(vec![("tau", j.recorrido())]),
            otro => Err(format!("medida desconocida: {otro}")),
        };
    }
    let u = num(m, "u")?;
    Ok(vec![("tau", j.tau(u)), ("largo_cable", j.largo_cable(u))])
}

// ------------------------------------------------------------- reel 39
fn prensa(m: &Muestra) -> Result<(Press, Variante), String> {
    let variante = match texto(m, "variante")?.as_str() {
        "militar" => Variante::Militar,
        "smith" => Variante::Smith,
        otro => return Err(format!("variante desconocida: {otro}")),
    };
    let mut p = Press::reel39(
        Persona {
            estatura_m: 1.75,
            masa_kg: 80.0,
        },
        40.0,
    );
    // Las muestras de SENSIBILIDAD mueven la máquina: dónde te paras en
    // la Smith, y dónde arranca y cuándo se va atrás la barra libre. Que
    // el motor las reproduzca es lo que distingue un modelo de un ajuste
    // al caso central.
    if let Ok(x) = num(m, "x") {
        p.x_smith = x;
    }
    if let Ok(x0) = num(m, "x0") {
        p.x_militar = x0;
    }
    if let Ok(claro) = num(m, "claro") {
        p.claro = claro;
    }
    Ok((p, variante))
}

fn reel39(m: &Muestra) -> Result<Vec<Salida>, String> {
    let (p, variante) = prensa(m)?;

    if let Ok(medida) = texto(m, "medida") {
        if medida == "balance de energía" {
            let (musculos, carga) = p.balance(variante, 800).map_err(|e| format!("{e:?}"))?;
            return Ok(vec![
                ("trabajo_musculos", musculos),
                ("trabajo_carga", carga),
            ]);
        }
        return Err(format!("medida desconocida: {medida}"));
    }

    // Muestras de sensibilidad: no traen `u`, traen agregados.
    if !m.entrada.contains_key("u") {
        let fin = p.pose(variante, 1.0).map_err(|e| format!("{e:?}"))?;
        let medio = p.pose(variante, 0.5).map_err(|e| format!("{e:?}"))?;
        return Ok(vec![
            ("pico_hombro", p.pico_hombro(variante, 400)),
            ("tau_hombro_final", p.tau_hombro(&fin)),
            // El dorado lo llama "medio" y es el valor a mitad del
            // recorrido, no un promedio. Se reproduce lo que dice, no lo
            // que el nombre sugiere.
            ("tau_hombro_medio", p.tau_hombro(&medio)),
        ]);
    }

    let u = num(m, "u")?;
    let pose = p.pose(variante, u).map_err(|e| format!("{e:?}"))?;
    Ok(vec![
        ("hombro", p.tau_hombro(&pose)),
        ("codo", p.tau_codo(&pose)),
        ("l5s1", p.tau_l5s1(&pose)),
        ("hombro_grados", p.angulo_hombro(&pose)),
        ("codo_grados", p.angulo_codo(&pose)),
    ])
}

/// La postura que el motor calcula, para comparar punto por punto.
///
/// # Errors
/// Si la muestra no trae `variante` y `u`, o la mano cae fuera de alcance.
pub fn pose_reel39(m: &Muestra) -> Result<[(f64, f64); 4], String> {
    let (p, variante) = prensa(m)?;
    let u = num(m, "u")?;
    let pose = p.pose(variante, u).map_err(|e| format!("{e:?}"))?;
    Ok([pose.l5, pose.hombro, pose.codo, pose.mano])
}

// ------------------------------------------------------------ lectura
fn num(m: &Muestra, clave: &str) -> Result<f64, String> {
    m.entrada
        .get(clave)
        .and_then(serde_json::Value::as_f64)
        .ok_or_else(|| format!("falta la entrada {clave}"))
}

fn texto(m: &Muestra, clave: &str) -> Result<String, String> {
    m.entrada
        .get(clave)
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| format!("falta la entrada {clave}"))
}
