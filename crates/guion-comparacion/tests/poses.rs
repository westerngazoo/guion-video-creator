//! La cinemática, contra las poses doradas de los reels publicados.
//!
//! Esta prueba es la que el plan pone ANTES de cualquier torque: un torque
//! correcto con una postura equivocada es una coincidencia, no física. Las
//! poses vienen de `fixtures/dorados.json`, que salió de los módulos que
//! de verdad dibujaron los reels 38 y 39.
//!
//! Se toman de ahí el hombro y la mano —que son el camino del ejercicio, y
//! eso es trabajo del `Ejercicio`, no de la máquina— y se le pide a
//! `mecanica::maquina_humana` el codo. Si el codo cae donde cayó en el
//! video, la antropometría y la rama de la IK están bien.

use mecanica::maquina_humana::{codo, Doblez, Persona};

/// R-0001 Q5.
const TOLERANCIA: f64 = 0.02;

fn dorados() -> serde_json::Value {
    let ruta = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/dorados.json");
    serde_json::from_str(&std::fs::read_to_string(ruta).expect("fixtures")).expect("json")
}

fn xy(v: &serde_json::Value) -> [f64; 2] {
    [v[0].as_f64().unwrap(), v[1].as_f64().unwrap()]
}

fn dist(a: [f64; 2], b: [f64; 2]) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
}

/// Las muestras con pose de una pieza, como (variante, u, l5, hombro, codo, mano).
fn muestras(
    d: &serde_json::Value,
    pieza: &str,
) -> Vec<(String, f64, [f64; 2], [f64; 2], [f64; 2], [f64; 2])> {
    d["piezas"][pieza]["muestras"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|m| m.get("pose").is_some())
        .map(|m| {
            let p = &m["pose"];
            (
                m["entrada"]["variante"].as_str().unwrap().to_string(),
                m["entrada"]["u"].as_f64().unwrap(),
                xy(&p["l5"]),
                xy(&p["hombro"]),
                xy(&p["codo"]),
                xy(&p["mano"]),
            )
        })
        .collect()
}

fn persona() -> Persona {
    Persona {
        estatura_m: 1.75,
        masa_kg: 80.0,
    }
}

/// Los largos de segmento que el reel usó son los que salen de Winter.
///
/// Si esto falla, la máquina y el reel están describiendo cuerpos
/// distintos, y cualquier torque que coincida después coincide por
/// casualidad.
#[test]
fn los_segmentos_publicados_salen_de_las_fracciones() {
    let d = dorados();
    let p = persona();
    for pieza in ["reel38", "reel39"] {
        for (v, u, l5, hombro, codo_dorado, mano) in muestras(&d, pieza) {
            let torso = dist(l5, hombro);
            assert!(
                (torso / p.torso() - 1.0).abs() < TOLERANCIA,
                "{pieza} {v} u={u}: torso {torso:.4} contra {:.4}",
                p.torso()
            );
            let humero = dist(hombro, codo_dorado);
            assert!(
                (humero / p.humero() - 1.0).abs() < TOLERANCIA,
                "{pieza} {v} u={u}: húmero {humero:.4} contra {:.4}",
                p.humero()
            );
            let antebrazo = dist(codo_dorado, mano);
            assert!(
                (antebrazo / p.antebrazo() - 1.0).abs() < TOLERANCIA,
                "{pieza} {v} u={u}: antebrazo {antebrazo:.4} contra {:.4}",
                p.antebrazo()
            );
        }
    }
}

/// El codo cae donde cayó en el video publicado.
///
/// La rama se nombra en términos anatómicos y es un dato del ejercicio,
/// no una regla que la geometría infiera. Con el brazo estirado la
/// dirección del doblez no está definida —el codo queda sobre el eje— así
/// que ahí se compara la POSICIÓN, que sí lo está, y es lo único que
/// alguien ve.
#[test]
fn el_codo_cae_donde_cayo_en_el_video() {
    let d = dorados();
    let p = persona();
    let casos: [(&str, &str, Doblez); 4] = [
        ("reel38", "barra", Doblez::Atras),
        ("reel38", "polea", Doblez::Abajo),
        // Adelante Y abajo: al arrancar el press la barra está al frente
        // del hombro a la misma altura, y "adelante" a secas es paralelo
        // al eje, o sea que no nombra ningún lado. El solve lo rechaza en
        // vez de adivinar, que es exactamente lo que debe hacer.
        ("reel39", "militar", Doblez::Hacia([1.0, -0.3])),
        ("reel39", "smith", Doblez::Hacia([1.0, -0.3])),
    ];
    for (pieza, variante, hacia) in casos {
        let mut vistas = 0;
        for (v, u, _l5, hombro, codo_dorado, mano) in muestras(&d, pieza) {
            if v != variante {
                continue;
            }
            vistas += 1;
            let mio = codo(&p, hombro, mano, hacia).expect("el codo se resuelve");
            let error = dist(mio, codo_dorado);
            let escala = p.brazo();
            assert!(
                error / escala < TOLERANCIA,
                "{pieza} {variante} u={u}: el codo cae en {mio:?}, el video lo puso \
                 en {codo_dorado:?} — {:.1}% del brazo",
                error / escala * 100.0
            );
        }
        assert!(vistas >= 5, "{pieza} {variante}: sólo {vistas} muestras");
    }
}
