use guion_fisica::{default_dorados_path, eval_reel29, load_dorados, within_tol, DORADOS_TOL};

#[test]
fn reel29_beta0_matches_dorados() {
    let dorados = load_dorados(&default_dorados_path()).expect("dorados.json");
    let pieza = dorados.piezas.get("reel29").expect("reel29");

    for (i, muestra) in pieza.muestras.iter().enumerate() {
        let beta = muestra
            .entrada
            .get("beta_grados")
            .and_then(|v| v.as_f64())
            .unwrap_or(-1.0);
        if beta != 0.0 {
            continue;
        }

        let got = eval_reel29(muestra).expect("eval reel29");
        let key = if muestra.salida.contains_key("largo_cable") {
            "largo_cable"
        } else if muestra.salida.contains_key("recorrido") {
            "recorrido"
        } else {
            "tau"
        };
        let expected = muestra.salida.get(key).and_then(|v| v.as_f64()).expect(key);
        assert!(
            within_tol(got, expected, DORADOS_TOL),
            "muestra {i} ({key}): got {got}, expected {expected}"
        );
    }
}
