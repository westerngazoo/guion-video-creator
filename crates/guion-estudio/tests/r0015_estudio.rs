//! R-0015 AC6 — `guion studio <post>`: the endpoints, a frame's bytes, and
//! «guardar» back to `escena.toml`.
//!
//! The router is tested without a socket ([`responder`]) and once over a
//! real 127.0.0.1 connection. Frames are rendered by guion-reel (no
//! ffmpeg). Text is set in DejaVu Sans where it is installed; without it
//! the studio draws everything but text, and the tests still hold.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};

use guion_cuenta::nueva::{crear, Respuestas};
use guion_estudio::http::{responder, servir_n, Peticion};
use guion_estudio::Estudio;
use guion_gadget::{Escena, Registro};
use serde_json::Value;

const DEJAVU: &str = "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf";
const DEJAVU_BOLD: &str = "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf";

const ESCENA: &str = r#"version = 1
duracion = 3.0

[[gadget]]
id = "brazo"
tipo = "palanca"

[gadget.params]
masa = { valor = 20.0, procedencia = "supuesto", nota = "la mancuerna que el reel declara" }
brazo = { valor = 0.32, procedencia = "supuesto" }
angulo = { valor = 0.0, procedencia = "supuesto" }

[[gadget.anim]]
param = "angulo"
de = 0.0
a = 90.0
desde = "inicio+0.5"
hasta = "inicio+1.5"

[[gadget]]
id = "tau"
tipo = "medidor"
entra = "inicio+0.5"

[gadget.params]
valor = { de = "brazo.tau" }
rotulo = "τ en el codo"
"#;

fn dir(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("guion-estudio-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn post(name: &str) -> PathBuf {
    let d = dir(name);
    std::fs::write(d.join("escena.toml"), ESCENA).unwrap();
    d
}

fn cara() -> Option<&'static Path> {
    Some(Path::new(DEJAVU)).filter(|p| p.exists())
}

fn abrir(p: &Path) -> Estudio {
    Estudio::abrir(p, Registro::integrados(), cara()).unwrap()
}

fn json(r: &guion_estudio::http::Respuesta) -> Value {
    serde_json::from_slice(&r.cuerpo).unwrap()
}

#[test]
fn ac6_a_frame_is_a_png_and_its_bytes_are_deterministic() {
    let p = post("cuadro");
    let mut e = abrir(&p);
    let r = responder(&mut e, &Peticion::get("/api/cuadro.png?t=1.5"));
    assert_eq!(r.estado, 200, "{}", r.texto());
    assert_eq!(r.tipo, "image/png");
    assert_eq!(&r.cuerpo[..8], b"\x89PNG\r\n\x1a\n");
    // the same t: the same bytes, from the same studio and from a new one
    let otra = responder(&mut e, &Peticion::get("/api/cuadro.png?t=1.5"));
    assert_eq!(r.cuerpo, otra.cuerpo);
    let mut e2 = abrir(&p);
    assert_eq!(
        r.cuerpo,
        responder(&mut e2, &Peticion::get("/api/cuadro.png?t=1.5")).cuerpo
    );
    // another t, another frame; full size is larger than the default (÷2)
    assert_ne!(
        r.cuerpo,
        responder(&mut e, &Peticion::get("/api/cuadro.png?t=0.2")).cuerpo
    );
    let completo = responder(&mut e, &Peticion::get("/api/cuadro.png?t=1.5&r=1"));
    assert_eq!(completo.estado, 200);
    assert_eq!(
        &completo.cuerpo[16..24],
        &[0, 0, 4, 56, 0, 0, 7, 128],
        "1080 × 1920"
    );
    assert_eq!(&r.cuerpo[16..24], &[0, 0, 2, 28, 0, 0, 3, 192], "540 × 960");
    // the PNG holds exactly guion-reel's frame
    let rgb = e.cuadro_rgb(1.5).unwrap();
    assert_eq!(rgb.len(), 1080 * 1920 * 3);
    assert_eq!(
        completo.cuerpo,
        guion_estudio::estudio::png(&rgb, 1080, 1920).unwrap()
    );
    let r = responder(&mut e, &Peticion::get("/api/cuadro.png?t=-1"));
    assert_eq!(r.estado, 400);
    assert!(r.texto().contains("seconds"));
}

#[test]
fn ac6_the_state_lists_gadgets_params_and_provenance() {
    let p = post("estado");
    let mut e = abrir(&p);
    let r = responder(&mut e, &Peticion::get("/api/estado?t=2.0"));
    assert_eq!(r.estado, 200, "{}", r.texto());
    let j = json(&r);
    assert_eq!(j["plantilla"]["nombre"], "neutra");
    assert_eq!(j["duracion"], 3.0);
    let gs = j["gadgets"].as_array().unwrap();
    assert_eq!(gs.len(), 2);
    assert_eq!(gs[0]["id"], "brazo");
    let masa = gs[0]["params"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["nombre"] == "masa")
        .unwrap();
    assert_eq!(masa["procedencia"], "supuesto");
    assert_eq!(masa["unidad"], "kg");
    assert_eq!(masa["min"], 0.0);
    let angulo = gs[0]["params"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["nombre"] == "angulo")
        .unwrap();
    assert_eq!(angulo["animado"], true);
    assert_eq!(angulo["valor"], 90.0);
    let valor = gs[1]["params"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["nombre"] == "valor")
        .unwrap();
    assert_eq!(valor["de"], "brazo.tau");
    let lectura = gs[1]["valores"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["nombre"] == "lectura")
        .unwrap();
    assert!((lectura["cantidad"].as_f64().unwrap() - 62.784).abs() < 1e-9);
    assert_eq!(lectura["procedencia"], "calculado");
    let bib: Vec<&str> = j["biblioteca"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["categoria"].as_str().unwrap())
        .collect();
    assert_eq!(bib, vec!["Física", "Datos"]);
    let page = responder(&mut e, &Peticion::get("/"));
    assert_eq!(page.estado, 200);
    assert!(page.texto().contains("GUION STUDIO"));
    assert_eq!(responder(&mut e, &Peticion::get("/nada")).estado, 404);
}

#[test]
fn ac6_params_round_trip_through_guardar() {
    let p = post("guardar");
    let mut e = abrir(&p);
    let antes = responder(&mut e, &Peticion::get("/api/cuadro.png?t=2.0")).cuerpo;
    let r = responder(
        &mut e,
        &Peticion::post(
            "/api/param?t=2.0",
            r#"{"id":"brazo","param":"masa","valor":25}"#,
        ),
    );
    assert_eq!(r.estado, 200, "{}", r.texto());
    let lectura = json(&r)["gadgets"][1]["valores"][0]["cantidad"]
        .as_f64()
        .unwrap();
    assert!((lectura - 25.0 * 9.81 * 0.32).abs() < 1e-9, "{lectura}");
    let r = responder(
        &mut e,
        &Peticion::post(
            "/api/param",
            r#"{"id":"brazo","param":"etiqueta","valor":"d"}"#,
        ),
    );
    assert_eq!(r.estado, 200, "{}", r.texto());
    // a value the spec refuses: 400 naming it, and nothing changes
    let malo = responder(
        &mut e,
        &Peticion::post("/api/param", r#"{"id":"brazo","param":"masa","valor":900}"#),
    );
    assert_eq!(malo.estado, 400);
    assert!(malo.texto().contains("[0, 500] kg"), "{}", malo.texto());
    let malo = responder(
        &mut e,
        &Peticion::post(
            "/api/param",
            r#"{"id":"brazo","param":"etiqueta","valor":"mucha"}"#,
        ),
    );
    assert_eq!(malo.estado, 400);
    assert_ne!(
        antes,
        responder(&mut e, &Peticion::get("/api/cuadro.png?t=2.0")).cuerpo
    );

    // nothing is written until guardar; then the file is the scene in memory
    assert_eq!(
        std::fs::read_to_string(p.join("escena.toml")).unwrap(),
        ESCENA
    );
    let vista = responder(&mut e, &Peticion::get("/api/escena.toml")).texto();
    let g = responder(&mut e, &Peticion::post("/api/guardar", ""));
    assert_eq!(g.estado, 200, "{}", g.texto());
    let guardado = std::fs::read_to_string(p.join("escena.toml")).unwrap();
    assert_eq!(guardado, vista);
    assert!(
        guardado.contains(r#"masa = { valor = 25.0, procedencia = "supuesto", nota = "la mancuerna que el reel declara" }"#),
        "{guardado}"
    );
    assert!(guardado.contains(r#"etiqueta = "d""#), "{guardado}");
    // reopened from the file: the same scene, the same frame
    let mut e2 = abrir(&p);
    assert_eq!(e2.escena, e.escena);
    assert_eq!(
        responder(&mut e2, &Peticion::get("/api/cuadro.png?t=2.0")).cuerpo,
        responder(&mut e, &Peticion::get("/api/cuadro.png?t=2.0")).cuerpo
    );
    // saving again without changes gives the same bytes
    responder(&mut e2, &Peticion::post("/api/guardar", ""));
    assert_eq!(
        std::fs::read_to_string(p.join("escena.toml")).unwrap(),
        guardado
    );
    assert_eq!(Escena::load(&p).unwrap(), e.escena);
}

#[test]
fn ac6_the_server_answers_over_localhost() {
    let p = post("tcp");
    let mut e = abrir(&p);
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let dir = l.local_addr().unwrap();
    let local = responder(&mut e, &Peticion::get("/api/cuadro.png?t=1.0")).cuerpo;
    let server = std::thread::spawn(move || {
        servir_n(&mut e, &l, 2);
        e
    });
    let pedir = |req: String| -> Vec<u8> {
        let mut s = TcpStream::connect(dir).unwrap();
        s.write_all(req.as_bytes()).unwrap();
        let mut out = Vec::new();
        s.read_to_end(&mut out).unwrap();
        out
    };
    let r = pedir("GET /api/cuadro.png?t=1.0 HTTP/1.1\r\nHost: x\r\n\r\n".into());
    let fin = r.windows(4).position(|w| w == b"\r\n\r\n").unwrap();
    let cab = String::from_utf8_lossy(&r[..fin]).into_owned();
    assert!(cab.starts_with("HTTP/1.1 200 OK"), "{cab}");
    assert!(cab.contains("Content-Type: image/png"), "{cab}");
    assert_eq!(
        &r[fin + 4..],
        local.as_slice(),
        "the socket serves the router's bytes"
    );
    let cuerpo = r#"{"id":"tau","param":"maximo","valor":80}"#;
    let r = pedir(format!(
        "POST /api/param HTTP/1.1\r\nHost: x\r\nContent-Length: {}\r\n\r\n{cuerpo}",
        cuerpo.len()
    ));
    assert!(String::from_utf8_lossy(&r).starts_with("HTTP/1.1 200 OK"));
    let e = server.join().unwrap();
    assert!(e.escena.a_toml(&e.registro).contains("maximo = 80.0"));
}

#[test]
fn ac6_the_check_runs_every_frame() {
    let p = post("check");
    let e = abrir(&p);
    let avisos = e.comprobar().unwrap();
    if cara().is_some() {
        assert!(avisos.is_empty(), "{avisos:#?}");
    } else {
        assert!(
            avisos.iter().any(|a| a.contains("sin caras")),
            "{avisos:#?}"
        );
    }
    // a scale too short for the peak: the gauge says so, with the instant
    std::fs::write(
        p.join("escena.toml"),
        ESCENA.replace("rotulo =", "maximo = 50.0\nrotulo ="),
    )
    .unwrap();
    let avisos = abrir(&p).comprobar().unwrap();
    assert!(
        avisos
            .iter()
            .any(|a| a.contains("· tau: medidor: el valor") && a.contains("tope 50")),
        "{avisos:#?}"
    );
}

#[test]
fn ac6_the_account_sets_the_template_and_names_its_own_gadgets() {
    if !Path::new(DEJAVU).exists() {
        eprintln!("skipped: {DEJAVU} is not installed");
        return;
    }
    let d = dir("cuenta");
    let mut r = Respuestas::con_defaults(
        "El extraño arte de computar",
        "Tecnología, IA y computación, desde cero",
        "computologo",
        "computación, IA y tecnología",
    );
    r.fecha = "2026-10-20".into();
    r.fuente_titulo = DEJAVU_BOLD.into();
    r.fuente_texto = DEJAVU.into();
    let raiz = crear(&r, &d.join("arte"), None).unwrap().raiz;
    let p = raiz.join("posts/palanca");
    std::fs::create_dir_all(&p).unwrap();
    std::fs::write(p.join("escena.toml"), ESCENA).unwrap();
    let mut e = Estudio::abrir(&p, Registro::integrados(), None).unwrap();
    assert_eq!(e.plantilla.nombre, "El extraño arte de computar");
    assert!(e.plantilla.notas.is_empty(), "{:?}", e.plantilla.notas);
    let cuenta = e.plantilla.cuenta.clone().unwrap();
    let acento = cuenta.color(&cuenta.marca.roles.acento[0]);
    let a = e.plantilla.paleta.color("acento").unwrap();
    assert_eq!([a.r, a.g, a.b], acento);
    assert_eq!(
        responder(&mut e, &Peticion::get("/api/cuadro.png?t=2")).estado,
        200
    );

    // the account declares a gadget of its own: the built-in studio names the binary
    let ct = raiz.join("cuenta.toml");
    let mut texto = std::fs::read_to_string(&ct).unwrap();
    texto.push_str("\n[estudio]\ngadgets = [\"kart-tres\"]\n");
    std::fs::write(&ct, &texto).unwrap();
    std::fs::write(
        p.join("escena.toml"),
        "version = 1\nduracion = 2.0\n[[gadget]]\nid = \"kart\"\ntipo = \"kart-tres\"\n",
    )
    .unwrap();
    let err = Estudio::abrir(&p, Registro::integrados(), None)
        .err()
        .unwrap();
    assert!(
        err.contains("a gadget of this account") && err.contains("--bin estudio"),
        "{err}"
    );
    assert!(err.contains("motor/Cargo.toml"), "{err}");
    // and a tipo that is not a slug is refused by the contract
    std::fs::write(&ct, texto.replace("kart-tres", "Kart Tres")).unwrap();
    let err = Estudio::abrir(&p, Registro::integrados(), None)
        .err()
        .unwrap();
    assert!(err.contains("estudio.gadgets"), "{err}");
}
