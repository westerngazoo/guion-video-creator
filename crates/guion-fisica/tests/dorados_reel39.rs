use guion_fisica::{
    default_dorados_path, eval_reel39_field, load_dorados, within_tol, DORADOS_TOL,
};

#[test]
fn reel39_matches_dorados() {
    let dorados = load_dorados(&default_dorados_path()).expect("dorados.json");
    let pieza = dorados.piezas.get("reel39").expect("reel39");

    for (i, muestra) in pieza.muestras.iter().enumerate() {
        for (key, value) in &muestra.salida {
            if key == "l5s1" {
                continue;
            }
            let expected = match value.as_f64() {
                Some(v) => v,
                None => continue,
            };
            if key == "tau_hombro_final" && expected.abs() < 1e-10 {
                continue;
            }
            let got = eval_reel39_field(muestra, key).expect("eval reel39");
            assert!(
                within_tol(got, expected, DORADOS_TOL),
                "muestra {i} ({key}): got {got}, expected {expected}"
            );
        }
    }
}
