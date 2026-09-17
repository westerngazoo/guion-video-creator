//! Golden harness: each implemented piece must pass all its dorados samples @ 2 %.

use guion_fisica::{
    default_dorados_path, eval_reel04, eval_reel26, load_dorados, within_tol, DORADOS_TOL,
};

const IMPLEMENTED: &[&str] = &["reel04", "reel26"];

#[test]
fn implemented_pieces_match_dorados() {
    let dorados = load_dorados(&default_dorados_path()).expect("dorados.json");

    for name in IMPLEMENTED {
        let pieza = dorados.piezas.get(*name).expect("pieza en dorados.json");

        for (i, muestra) in pieza.muestras.iter().enumerate() {
            let (got, expected) = match *name {
                "reel04" => {
                    let got = eval_reel04(muestra).expect("eval reel04");
                    let expected = muestra
                        .salida
                        .get("tau")
                        .and_then(|v| v.as_f64())
                        .expect("tau en salida");
                    (got, expected)
                }
                "reel26" => {
                    let got = eval_reel26(muestra).expect("eval reel26");
                    let expected = muestra
                        .salida
                        .get("tau")
                        .and_then(|v| v.as_f64())
                        .expect("tau en salida");
                    (got, expected)
                }
                _ => unreachable!(),
            };

            assert!(
                within_tol(got, expected, DORADOS_TOL),
                "{name} muestra {i}: got {got}, expected {expected}"
            );
        }
    }
}
