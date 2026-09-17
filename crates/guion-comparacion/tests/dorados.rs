//! El examen del diseño.
//!
//! Un guion TOML tiene que producir los mismos números que el reel 40 de
//! Python, que están congelados en `fixtures/dorados.json` y verificados
//! contra `physics-lab/mecanica/src/gluteo.rs`.
//!
//! El reel 40 nunca se publicó a mano justamente para esto: un caso que ya
//! salió por el camino viejo deja de ser una prueba.

use guion_comparacion::{guion, Medida, Veredicto};

/// R-0001 Q5. Si no cierra, se entiende por qué; no se sube.
const TOLERANCIA: f64 = 0.02;

fn dorados() -> serde_json::Value {
    let ruta = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/dorados.json");
    let texto = std::fs::read_to_string(ruta).expect("fixtures/dorados.json");
    serde_json::from_str(&texto).expect("json válido")
}

fn comparacion() -> guion_comparacion::Comparacion {
    let ruta = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../guiones/reel40-gluteo.toml"
    );
    let texto = std::fs::read_to_string(ruta).expect("el guion");
    guion::cargar(&texto).expect("el guion carga")
}

fn cerca(a: f64, b: f64, que: &str) {
    let rel = (a - b).abs() / b.abs().max(f64::EPSILON);
    assert!(
        rel <= TOLERANCIA,
        "{que}: {a:.3} contra el dorado {b:.3} ({:.2}% fuera)",
        rel * 100.0
    );
}

#[test]
fn reproduce_los_dorados_del_reel40() {
    let c = comparacion();
    let d = dorados();
    let k = &d["piezas"]["reel40"]["constantes"];

    for (i, nombre) in ["rumano", "hipthrust"].iter().enumerate() {
        cerca(
            c.valor(i, Medida::Pico),
            k["PICO"][nombre].as_f64().unwrap(),
            &format!("pico de {nombre}"),
        );
        cerca(
            c.valor(i, Medida::Trabajo),
            k["TRABAJO"][nombre].as_f64().unwrap(),
            &format!("trabajo de {nombre}"),
        );
        // el eje compartido: 0 es el estiramiento, 1 el cierre
        cerca(
            c.valor(i, Medida::EnProgreso(0.0)),
            k["INICIO"][nombre].as_f64().unwrap(),
            &format!("{nombre} al inicio"),
        );
        let fin_dorado = k["FIN"][nombre].as_f64().unwrap();
        let fin = c.valor(i, Medida::EnProgreso(1.0));
        // el rumano cierra en cero exacto: ahí lo relativo no aplica
        assert!(
            (fin - fin_dorado).abs() <= TOLERANCIA * fin_dorado.abs().max(1.0),
            "{nombre} al cierre: {fin:.3} contra {fin_dorado:.3}"
        );
    }
    cerca(
        c.cruce(0, 1).expect("las curvas se cruzan"),
        k["CRUCE"].as_f64().unwrap(),
        "el cruce",
    );
}

#[test]
fn el_objetivo_se_mueve_con_la_repeticion() {
    // La compuerta del molde, ahora como propiedad de cualquier
    // comparación en vez de treinta comprobaciones escritas por reel.
    let c = comparacion();
    let mejor = (0..c.opciones.len())
        .map(|i| c.recorrido_relativo(i, 200))
        .fold(0.0_f64, f64::max);
    assert!(
        mejor >= 0.30,
        "el τ del objetivo cambia como mucho {:.0}% del pico: se lee fijo",
        mejor * 100.0
    );
}

#[test]
fn el_hallazgo_es_que_se_cruzan() {
    let c = comparacion();
    let s = c.cruce(0, 1).expect("se cruzan");
    assert!(s > 0.0 && s < 1.0, "el cruce cae dentro de la repetición");
    // y quien va arriba cambia de lado: eso es lo que hace al par valioso
    let antes = c.valor(0, Medida::EnProgreso(0.0)) > c.valor(1, Medida::EnProgreso(0.0));
    let despues = c.valor(0, Medida::EnProgreso(1.0)) > c.valor(1, Medida::EnProgreso(1.0));
    assert!(antes != despues, "cargan extremos opuestos del recorrido");
}

#[test]
fn sin_ganador_cuando_los_numeros_no_lo_dan() {
    let c = comparacion();
    // los dos criterios del guion son informativos: no se corona a nadie
    for r in c.renglones() {
        assert_ne!(r.veredicto, Veredicto::Gana(0), "{}", r.etiqueta);
        assert_ne!(r.veredicto, Veredicto::Gana(1), "{}", r.etiqueta);
    }
    // y "empatan" no se usa a la ligera: 359 J contra 290 J difieren un
    // 19%, así que ese renglón NO puede rotularse "iguales" en pantalla
    let trabajo = c
        .renglones()
        .into_iter()
        .find(|r| r.etiqueta.contains("Trabajo"))
        .unwrap();
    assert_eq!(trabajo.veredicto, Veredicto::SinGanador);
}

#[test]
fn un_modelo_fuera_del_catalogo_falla_en_vez_de_adivinar() {
    let malo = r#"
[comparacion]
titulo = "x"
objetivo = "cadera"
carga_kg = 100.0
[[opcion]]
nombre = "a"
modelo = "bisagra_de_kadera"
palanca_m = 0.53
rango_grados = 72.0
[[opcion]]
nombre = "b"
modelo = "puente_de_cadera"
palanca_m = 0.46
rango_grados = 40.0
"#;
    // `expect_err` pediría `Debug` en `Comparacion`, y `Box<dyn Lift>` no
    // lo tiene: el match dice lo mismo sin ensuciar el tipo público.
    let e = match guion::cargar(malo) {
        Err(e) => e,
        Ok(_) => panic!("un modelo inventado debe fallar, no cargar"),
    };
    assert!(
        e.contains("bisagra_de_kadera"),
        "el error nombra el modelo: {e}"
    );
    assert!(e.contains("El catálogo tiene"), "y dice qué sí hay: {e}");
}
