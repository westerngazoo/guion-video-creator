//! R-0019 AC2: the account sets its natural voice in `cuenta.toml`
//! `[voz.prosodia]`; a value out of range or an unknown key fails by name.

use guion_cuenta::contrato::{Cuenta, ProsodiaCuenta};
use guion_cuenta::nueva::{plan, Respuestas};

fn contrato() -> String {
    let mut r = Respuestas::con_defaults("Cuenta", "tema", "revisor", "materia");
    r.fecha = "2026-11-26".into();
    plan(&r, "")
        .unwrap()
        .archivos
        .into_iter()
        .find(|a| a.ruta == "cuenta.toml")
        .unwrap()
        .texto
}

fn con(prosodia: &str) -> String {
    contrato().replace(
        "\n[marca]\n",
        &format!("\n[voz.prosodia]\n{prosodia}\n\n[marca]\n"),
    )
}

#[test]
fn ac2_the_contract_carries_the_natural_voice() {
    // optional: a new account's contract has none (and shows how, commented)
    let base = contrato();
    assert!(base.contains("# [voz.prosodia]"), "{base}");
    assert_eq!(Cuenta::parse(&base).unwrap().voz.prosodia, None);
    let c = Cuenta::parse(&con("ruido_w = 0.6\nsilencio = 0.4")).unwrap();
    assert_eq!(
        c.voz.prosodia,
        Some(ProsodiaCuenta {
            ruido: None,
            ruido_w: Some(0.6),
            silencio: Some(0.4),
        })
    );
}

/// AC2: new voices are natural by default: the account's reel template asks
/// for it in every new post's `voz.toml`.
#[test]
fn ac2_a_new_post_asks_for_the_natural_voice() {
    let mut r = Respuestas::con_defaults("Cuenta", "tema", "revisor", "materia");
    r.fecha = "2026-11-26".into();
    let voz = plan(&r, "")
        .unwrap()
        .archivos
        .into_iter()
        .find(|a| a.ruta.ends_with("plantillas/reel/voz.toml"))
        .unwrap()
        .texto;
    assert!(
        voz.lines().any(|l| l.starts_with("prosodia = \"natural\"")),
        "{voz}"
    );
}

#[test]
fn ac2_a_bad_natural_voice_fails_by_name() {
    let e = Cuenta::parse(&con("ruido = -0.1")).unwrap_err();
    assert!(e.contains("voz.prosodia.ruido"), "{e}");
    let e = Cuenta::parse(&con("ruidos = 0.5")).unwrap_err();
    assert!(e.contains("ruidos"), "{e}");
}
