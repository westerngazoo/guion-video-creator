//! La compuerta: cada pieza con puente al motor pasa TODAS sus muestras
//! doradas.
//!
//! Vale la pena decir qué mide esta prueba ahora y qué medía antes. Antes
//! el jalón y el press eran ajustes de curva sobre estos mismos valores,
//! así que la prueba comparaba los dorados contra funciones construidas
//! para reproducirlos. Ahora la geometría sale del motor y estos números
//! vienen de los reels publicados, que es lo que hace que la comparación
//! signifique algo.

use guion_fisica::{
    default_dorados_path, evalua, load_dorados, pose_reel39, within_tol, CON_PUENTE, DORADOS_TOL,
};

#[test]
fn las_piezas_con_puente_cuadran_con_sus_dorados() {
    let dorados = load_dorados(&default_dorados_path()).expect("dorados.json");
    let mut comprobadas = 0usize;

    for nombre in CON_PUENTE {
        let pieza = dorados
            .piezas
            .get(*nombre)
            .unwrap_or_else(|| panic!("{nombre} en dorados.json"));

        for (i, muestra) in pieza.muestras.iter().enumerate() {
            let salidas =
                evalua(nombre, muestra).unwrap_or_else(|e| panic!("{nombre} muestra {i}: {e}"));
            for (clave, obtenido) in salidas {
                let Some(esperado) = muestra
                    .salida
                    .get(clave)
                    .and_then(serde_json::Value::as_f64)
                else {
                    continue;
                };
                assert!(
                    within_tol(obtenido, esperado, DORADOS_TOL),
                    "{nombre} muestra {i} · {clave}: motor {obtenido}, \
                     dorado {esperado} (tol {DORADOS_TOL})"
                );
                comprobadas += 1;
            }
        }
    }
    // El piso es lo que HOY cubre, no un número redondo: si alguien quita
    // una pieza del puente, la compuerta lo dice en vez de pasar con
    // menos cobertura y verse igual de verde.
    assert!(
        comprobadas >= 101,
        "sólo se compararon {comprobadas} valores, y el puente cubre 101"
    );
}

/// Y la POSTURA, punto por punto. Un torque puede salir bien con una
/// postura equivocada si dos errores se cancelan; los puntos no mienten.
#[test]
fn la_postura_del_press_cuadra_punto_por_punto() {
    let dorados = load_dorados(&default_dorados_path()).expect("dorados.json");
    let pieza = dorados.piezas.get("reel39").expect("reel39");

    for (i, muestra) in pieza.muestras.iter().enumerate() {
        let Some(esperada) = muestra.salida.get("pose") else {
            continue;
        };
        let obtenida = pose_reel39(muestra).unwrap_or_else(|e| panic!("{e}"));
        for (k, nombre) in ["l5", "hombro", "codo", "mano"].iter().enumerate() {
            let punto = esperada
                .get(nombre)
                .and_then(|v| v.as_array())
                .unwrap_or_else(|| panic!("pose.{nombre}"));
            let (ex, ey) = (punto[0].as_f64().unwrap(), punto[1].as_f64().unwrap());
            let (gx, gy) = obtenida[k];
            let d = (gx - ex).hypot(gy - ey);
            assert!(
                d < 0.01,
                "reel39 muestra {i} · {nombre}: motor ({gx}, {gy}), \
                 dorado ({ex}, {ey}), {d} m aparte"
            );
        }
    }
}
