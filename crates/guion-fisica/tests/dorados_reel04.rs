use guion_fisica::{default_dorados_path, eval_reel04, load_dorados, within_tol, DORADOS_TOL};

#[test]
fn reel04_matches_dorados() {
    let dorados = load_dorados(&default_dorados_path()).expect("dorados.json");
    let pieza = dorados
        .piezas
        .get("reel04")
        .expect("reel04 en dorados.json");

    for (i, muestra) in pieza.muestras.iter().enumerate() {
        let got = eval_reel04(muestra).expect("eval reel04");
        let expected = muestra
            .salida
            .get("tau")
            .and_then(|v| v.as_f64())
            .expect("tau en salida");
        assert!(
            within_tol(got, expected, DORADOS_TOL),
            "muestra {i}: got {got}, expected {expected} (tol {DORADOS_TOL})"
        );
    }
}
