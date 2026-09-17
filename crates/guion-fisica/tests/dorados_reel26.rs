use guion_fisica::{default_dorados_path, eval_reel26, load_dorados, within_tol, DORADOS_TOL};

#[test]
fn reel26_matches_dorados() {
    let dorados = load_dorados(&default_dorados_path()).expect("dorados.json");
    let pieza = dorados.piezas.get("reel26").expect("reel26");

    for (i, muestra) in pieza.muestras.iter().enumerate() {
        let got = eval_reel26(muestra).expect("eval reel26");
        let expected = muestra
            .salida
            .get("tau")
            .and_then(|v| v.as_f64())
            .expect("tau");
        assert!(
            within_tol(got, expected, DORADOS_TOL),
            "muestra {i}: got {got}, expected {expected}"
        );
    }
}
