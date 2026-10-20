//! R-0015 — the gadget contract, the registry, the timeline and
//! `escena.toml`: one test (or more) per acceptance criterion.
//!
//! The oracles are fbf's model numbers (reel 47, `motor/src/curl.rs`): a
//! 20 kg dumbbell on a 0.32 m forearm, F·L = 196.2 N × 0.32 m = 62.784 N·m.
//! Label tests set text in DejaVu Sans (skipped, saying so, where it is not
//! installed).

use std::path::Path;

use guion_gadget::registro::{contexto_de_prueba, eval_con};
use guion_gadget::{
    comprobar_contrato, Categoria, Contexto, Dato, Escena, Gadget, Paleta, ParamSpec, Params,
    Procedencia, Registro, Salida, SalidaSpec, Tiempos, Tipo, Valor,
};
use motoreel::Prim2;
use motoreel_typeset::{Face, Fonts};

const DEJAVU: &str = "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf";

/// The curl of reel 47 as a scene: the forearm rises from hanging to
/// horizontal between 0.5 s and 1.5 s; a gauge, bound to its torque,
/// enters at 0.5 s.
const ESCENA: &str = r#"
version = 1
duracion = 3.0

[[gadget]]
id = "brazo"
tipo = "palanca"

[gadget.params]
masa = { valor = 20.0, procedencia = "supuesto", nota = "la mancuerna que el reel declara" }
brazo = { valor = 0.32, procedencia = "reportado", fuente = "Winter: 0.20·H con H = 1.60 m" }
angulo = { valor = 0.0, procedencia = "supuesto" }
x = 300.0
y = 700.0

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
maximo = 70.0
marca = 60.0
rotulo = "τ en el codo"
"#;

fn dejavu() -> Option<Fonts> {
    let b = std::fs::read(DEJAVU).ok()?;
    let mut f = Fonts::new();
    f.add(Face::load(b, "dejavu").ok()?);
    Some(f)
}

fn compilar<'r>(e: &Escena, reg: &'r Registro, cx: &Contexto) -> guion_gadget::Compilada<'r> {
    let t = Tiempos::de_voz(None, e.duracion_con(None, 30.0).unwrap());
    e.compilar(reg, &t, cx).unwrap()
}

fn error_de(texto: &str) -> String {
    let reg = Registro::integrados();
    let pal = Paleta::neutra();
    let cx = contexto_de_prueba(&pal);
    match Escena::parse(texto) {
        Err(e) => e,
        Ok(e) => {
            let t = Tiempos::de_voz(None, 3.0);
            match e.compilar(&reg, &t, &cx) {
                Ok(_) => panic!("this scene should fail:\n{texto}"),
                Err(m) => m,
            }
        }
    }
}

fn valor(s: &Salida, n: &str) -> Valor {
    s.valores.iter().find(|v| v.nombre == n).unwrap().clone()
}

// ---------- AC1: the contract ----------

#[test]
fn ac1_every_built_in_keeps_the_contract() {
    let reg = Registro::integrados();
    for g in reg.todos() {
        let f = comprobar_contrato(g);
        assert!(f.is_empty(), "{}: {f:#?}", g.tipo());
    }
}

#[test]
fn ac1_eval_is_pure_bit_for_bit() {
    let reg = Registro::integrados();
    let pal = Paleta::neutra();
    let caras = dejavu();
    let cx = Contexto {
        caras: caras.as_ref(),
        ..contexto_de_prueba(&pal)
    };
    for g in reg.todos() {
        for t in [0.0, 0.37, 2.0] {
            let a = eval_con(g, t, &[("angulo", 33.3)], &cx);
            let b = eval_con(g, t, &[("angulo", 33.3)], &cx);
            assert_eq!(format!("{a:?}"), format!("{b:?}"), "{} at {t}", g.tipo());
        }
    }
}

#[test]
fn ac1_provenance_is_required_and_kept() {
    let reg = Registro::integrados();
    let pal = Paleta::neutra();
    let cx = contexto_de_prueba(&pal);
    let e = Escena::parse(ESCENA).unwrap();
    let c = compilar(&e, &reg, &cx);
    let q = c.cuadro(2.0, &cx);
    // every computed value leaves as calculado; the gauge's reading keeps its input's
    for v in &q.gadgets[0].valores {
        assert_eq!(v.procedencia, Procedencia::Calculado, "{}", v.nombre);
    }
    let lectura = q.gadgets[1]
        .valores
        .iter()
        .find(|v| v.nombre == "lectura")
        .unwrap();
    assert_eq!(lectura.procedencia, Procedencia::Calculado);
    assert_eq!(
        q.gadgets[0].params.procedencia("brazo"),
        Some(Procedencia::Reportado)
    );
    assert_eq!(
        q.gadgets[0].params.procedencia("x"),
        None,
        "layout has none"
    );

    let quitar = |de: &str, a: &str| ESCENA.replacen(de, a, 1);
    let e = error_de(&quitar(
        r#"masa = { valor = 20.0, procedencia = "supuesto", nota = "la mancuerna que el reel declara" }"#,
        "masa = 20.0",
    ));
    assert!(
        e.contains("«masa»") && e.contains("needs its procedencia"),
        "{e}"
    );
    let e = error_de(&quitar(
        r#"procedencia = "reportado", fuente = "Winter: 0.20·H con H = 1.60 m""#,
        r#"procedencia = "medido""#,
    ));
    assert!(
        e.contains("«brazo»") && e.contains("needs its fuente"),
        "{e}"
    );
    let e = error_de(&quitar(
        "x = 300.0",
        r#"x = { valor = 300.0, procedencia = "supuesto" }"#,
    ));
    assert!(e.contains("«x»") && e.contains("no procedencia"), "{e}");
    let e = error_de(&quitar(
        r#"angulo = { valor = 0.0, procedencia = "supuesto" }"#,
        "",
    ));
    assert!(
        e.contains("«angulo»") && e.contains("never left to a default"),
        "{e}"
    );
    let e = error_de(&quitar(
        r#"procedencia = "supuesto", nota"#,
        r#"procedencia = "inventado", nota"#,
    ));
    assert!(e.contains("calculado, medido, reportado, supuesto"), "{e}");
}

#[test]
fn ac1_types_and_ranges_fail_by_name() {
    let r = |de: &str, a: &str| error_de(&ESCENA.replacen(de, a, 1));
    let e = r("valor = 20.0,", "valor = 600.0,");
    assert!(
        e.contains("gadget «brazo» param «masa»") && e.contains("[0, 500] kg"),
        "{e}"
    );
    let e = r("x = 300.0", "equis = 300.0");
    assert!(
        e.contains("«equis»") && e.contains("masa, brazo, angulo"),
        "{e}"
    );
    let e = r("rotulo = \"τ en el codo\"", "rotulo = 3.0");
    assert!(e.contains("«rotulo»") && e.contains("is a number"), "{e}");
    let e = r("y = 700.0", "color_fuerza = \"morado\"");
    assert!(
        e.contains("«morado»") && e.contains("neither a role nor a token"),
        "{e}"
    );
    let e = r("version = 1", "version = 1\nautor = \"x\"");
    assert!(e.contains("autor"), "{e}");
    let e = r("tipo = \"medidor\"", "tipo = \"barra\"");
    assert!(
        e.contains("no gadget «barra»") && e.contains("palanca, medidor"),
        "{e}"
    );
}

// ---------- AC2: the two ported gadgets and their oracles ----------

#[test]
fn ac2_palanca_reproduces_fbf_model_numbers() {
    let reg = Registro::integrados();
    let g = reg.buscar("palanca").unwrap();
    let pal = Paleta::neutra();
    let cx = contexto_de_prueba(&pal);
    let pico = eval_con(
        g,
        0.0,
        &[("masa", 20.0), ("brazo", 0.32), ("angulo", 90.0)],
        &cx,
    );
    assert!((valor(&pico, "tau").cantidad - 62.784).abs() < 1e-9);
    assert!((valor(&pico, "fuerza").cantidad - 196.2).abs() < 1e-9);
    assert!((valor(&pico, "d").cantidad - 0.32).abs() < 1e-12);
    // the same number physics-lab gives: the gadget does not re-derive it
    let tau = mecanica::palanca::momento(
        (0.0, 0.0),
        (0.32, 0.0),
        mecanica::palanca::peso(20.0),
        mecanica::palanca::Linea::Vertical,
    );
    assert_eq!(valor(&pico, "tau").cantidad, tau.abs());
    // fbf's τ(α, φ) = CARGA · LF · sen(α + φ): the predicador at α = 45°, φ = 0
    let pred = eval_con(g, 0.0, &[("angulo", 45.0)], &cx);
    let esperado = 20.0 * 9.81 * 0.32 * 45f64.to_radians().sin();
    assert!((valor(&pred, "tau").cantidad - esperado).abs() < 1e-9);
    assert!(pico.eventos.contains(&"maximo".to_string()));
    let plomo = eval_con(g, 0.0, &[("angulo", 0.0)], &cx);
    assert!(valor(&plomo, "tau").cantidad < 1e-12);
    assert!(plomo.eventos.contains(&"cero".to_string()));
}

#[test]
fn ac2_palanca_places_its_label_where_nothing_is() {
    let Some(caras) = dejavu() else {
        eprintln!("skipped: {DEJAVU} is not installed");
        return;
    };
    let reg = Registro::integrados();
    let g = reg.buscar("palanca").unwrap();
    let pal = Paleta::neutra();
    let cx = Contexto {
        caras: Some(&caras),
        ..contexto_de_prueba(&pal)
    };
    let textos = |s: &Salida| -> Vec<(f64, f64, String)> {
        s.prims
            .iter()
            .filter_map(|p| match p {
                Prim2::Text { at, text, .. } => Some((at.x, at.y, text.clone())),
                _ => None,
            })
            .collect()
    };
    let s = eval_con(g, 0.0, &[("angulo", 90.0)], &cx);
    let t = textos(&s);
    assert_eq!(t.len(), 1, "{:?}", s.avisos);
    assert_eq!(t[0].2, "τ = 62.8\u{202F}N·m");
    assert!(s.avisos.is_empty(), "{:?}", s.avisos);
    // the first choice, right of the line of action, is past the hand (x = 300 + 320)
    let x_px = t[0].0 * cx.canvas.px_per_unit + cx.canvas.width as f64 / 2.0;
    assert!(x_px > 620.0 + 24.0, "{x_px}");
    // against the right edge the first choice does not fit: it moves, it does not overlap
    let s = eval_con(g, 0.0, &[("angulo", 90.0), ("x", 680.0)], &cx);
    let t2 = textos(&s);
    assert_eq!(t2.len(), 1, "{:?}", s.avisos);
    let x2 = t2[0].0 * cx.canvas.px_per_unit + cx.canvas.width as f64 / 2.0;
    assert!(x2 < 1000.0 - 24.0, "{x2}");
    // in a box too small for any candidate, no label and an aviso
    let apretado = Contexto {
        seguro: [250.0, 650.0, 700.0, 750.0],
        ..cx
    };
    let s = eval_con(g, 0.0, &[("angulo", 90.0)], &apretado);
    assert!(textos(&s).is_empty());
    assert!(
        s.avisos.iter().any(|a| a.contains("no cabe")),
        "{:?}",
        s.avisos
    );
}

#[test]
fn ac2_medidor_fills_marks_and_warns() {
    let reg = Registro::integrados();
    let g = reg.buscar("medidor").unwrap();
    let pal = Paleta::neutra();
    let cx = contexto_de_prueba(&pal);
    let s = eval_con(
        g,
        0.0,
        &[("valor", 62.784), ("maximo", 70.0), ("marca", 60.0)],
        &cx,
    );
    assert!((valor(&s, "fraccion").cantidad - 62.784 / 70.0).abs() < 1e-12);
    assert_eq!(s.eventos, vec!["marca".to_string()]);
    let s = eval_con(g, 0.0, &[("valor", 75.0), ("maximo", 70.0)], &cx);
    assert_eq!(valor(&s, "fraccion").cantidad, 1.0);
    assert!(s.avisos.iter().any(|a| a.contains("se sale de la escala")));
    assert!(s.eventos.contains(&"tope".to_string()));
}

// ---------- AC3: the registry ----------

/// An account's own gadget: a dot whose radius is a param.
struct Punto;

impl Gadget for Punto {
    fn tipo(&self) -> &'static str {
        "kart-punto"
    }
    fn categoria(&self) -> Categoria {
        Categoria::TresD
    }
    fn doc(&self) -> &'static str {
        "un punto"
    }
    fn params(&self) -> &'static [ParamSpec] {
        &[ParamSpec {
            nombre: "r",
            doc: "radio",
            tipo: Tipo::Num {
                min: 1.0,
                max: 100.0,
                paso: 1.0,
                unidad: "px",
                defecto: 10.0,
            },
            fisico: false,
        }]
    }
    fn salidas(&self) -> &'static [SalidaSpec] {
        &[SalidaSpec {
            nombre: "area",
            unidad: "px²",
            doc: "π r²",
        }]
    }
    fn eval(&self, _t: f64, p: &Params, cx: &Contexto) -> Salida {
        let r = p.num("r");
        Salida {
            prims: vec![guion_gadget::dibujo::disco(
                &cx.canvas,
                (540.0, 960.0),
                r,
                8,
                cx.color("acento"),
            )],
            valores: vec![Valor::calculado(
                "area",
                std::f64::consts::PI * r * r,
                "px²",
                "π r²",
            )],
            ..Salida::default()
        }
    }
}

#[test]
fn ac3_accounts_add_gadgets_to_the_registry() {
    let mut reg = Registro::integrados();
    reg.registrar(Box::new(Punto)).unwrap();
    assert_eq!(reg.nombres(), vec!["palanca", "kart-punto", "medidor"]);
    assert!(comprobar_contrato(reg.buscar("kart-punto").unwrap()).is_empty());
    let e = reg.registrar(Box::new(Punto)).unwrap_err();
    assert!(e.contains("already registered"), "{e}");
    let estantes = guion_gadget::registro::estantes(&reg);
    let nombres: Vec<&str> = estantes.iter().map(|(c, _)| c.nombre()).collect();
    assert_eq!(nombres, vec!["Física", "3D", "Datos"]);
}

// ---------- AC4: the timeline ----------

#[test]
fn ac4_clips_and_bindings_follow_the_clock() {
    let reg = Registro::integrados();
    let pal = Paleta::neutra();
    let cx = contexto_de_prueba(&pal);
    let e = Escena::parse(ESCENA).unwrap();
    let c = compilar(&e, &reg, &cx);
    let antes = c.cuadro(0.25, &cx);
    assert_eq!(antes.gadgets[1].alfa, 0.0, "the gauge enters at 0.5 s");
    assert_eq!(antes.gadgets[0].params.num("angulo"), 0.0);
    let medio = c.cuadro(1.0, &cx);
    assert!(
        (medio.gadgets[0].params.num("angulo") - 45.0).abs() < 1e-9,
        "smoothstep at half way"
    );
    let fin = c.cuadro(2.0, &cx);
    let lectura = fin.gadgets[1]
        .valores
        .iter()
        .find(|v| v.nombre == "lectura")
        .unwrap();
    assert!(
        (lectura.cantidad - 62.784).abs() < 1e-9,
        "the gauge reads the lever"
    );
    assert_eq!(
        c.cuadro(1.37, &cx),
        c.cuadro(1.37, &cx),
        "a frame is a pure function of t"
    );

    let reloj = c.reloj(None, 30.0, &cx).unwrap();
    let ev = |n: &str| {
        reloj
            .events()
            .iter()
            .find(|(m, _)| m == n)
            .map(|(_, t)| *t)
            .unwrap_or_else(|| panic!("no event {n}: {:?}", reloj.events()))
    };
    assert_eq!(ev("brazo:entra"), 0.0);
    assert_eq!(ev("brazo:cero"), 0.0);
    assert_eq!(ev("tau:entra"), 0.5);
    let m = ev("brazo:maximo");
    assert!((1.3..=1.5).contains(&m), "{m}");
    assert!(ev("tau:marca") < m, "60 N·m comes before the peak");
    let s = reloj
        .series_list()
        .iter()
        .find(|s| s.name == "brazo.tau")
        .unwrap();
    assert_eq!(s.values.len(), 90);
    assert!((s.values[89] - 62.784).abs() < 1e-9);
    assert!(reloj.to_json(3.0, 30.0).contains("\"tau.lectura\""));
}

#[test]
fn ac4_anchors_are_never_typed_times() {
    let e = error_de(&ESCENA.replacen("entra = \"inicio+0.5\"", "entra = \"0.5\"", 1));
    assert!(
        e.contains("gadget «tau» entra") && e.contains("never typed"),
        "{e}"
    );
    let e = error_de(&ESCENA.replacen("desde = \"inicio+0.5\"", "desde = \"gancho\"", 1));
    assert!(
        e.contains("names no instant") && e.contains("inicio, fin"),
        "{e}"
    );
    let e = error_de(&ESCENA.replacen("hasta = \"inicio+1.5\"", "hasta = \"inicio+0.2\"", 1));
    assert!(e.contains("desde") && e.contains("before hasta"), "{e}");
    let e = error_de(&ESCENA.replacen(
        "valor = { de = \"brazo.tau\" }",
        "valor = { de = \"brazo.torque\" }",
        1,
    ));
    assert!(
        e.contains("no output «torque»") && e.contains("fuerza, d, tau"),
        "{e}"
    );
    // a binding reads a gadget above it, so there are no cycles
    let al_reves = format!(
        "{}\n[[gadget]]\nid = \"otro\"\ntipo = \"medidor\"\n[gadget.params]\nvalor = {{ de = \"tercero.lectura\" }}\n",
        ESCENA
    );
    let e = error_de(&al_reves);
    assert!(e.contains("no earlier gadget «tercero»"), "{e}");
}

#[test]
fn ac4_the_voice_sets_the_length_and_the_anchors() {
    let voz = guion_reel::Narration::from_json(
        r#"{"duracion_voz": 4.0, "lineas": [
            {"id": "gancho", "texto": "a", "subtitulo": "a", "t0": 0.5, "t1": 1.9},
            {"id": "sube", "texto": "b", "subtitulo": "b", "t0": 2.0, "t1": 4.0}]}"#,
    )
    .unwrap();
    let texto = ESCENA
        .replacen("duracion = 3.0", "cola = 1.0", 1)
        .replacen("desde = \"inicio+0.5\"", "desde = \"sube\"", 1)
        .replacen("hasta = \"inicio+1.5\"", "hasta = \"sube.fin\"", 1);
    let e = Escena::parse(&texto).unwrap();
    assert_eq!(e.duracion_con(Some(&voz), 30.0).unwrap(), 5.0);
    let reg = Registro::integrados();
    let pal = Paleta::neutra();
    let cx = contexto_de_prueba(&pal);
    let t = Tiempos::de_voz(Some(&voz), 5.0);
    let c = e.compilar(&reg, &t, &cx).unwrap();
    assert!((c.cuadro(3.0, &cx).gadgets[0].params.num("angulo") - 45.0).abs() < 1e-9);
    let reloj = c.reloj(Some(&voz), 30.0, &cx).unwrap();
    assert_eq!(reloj.events()[0], ("voz:gancho".to_string(), 0.5));
    // with a voice, a typed length is refused
    let e = Escena::parse(ESCENA).unwrap();
    assert!(e
        .duracion_con(Some(&voz), 30.0)
        .unwrap_err()
        .contains("voice sets the length"));
}

// ---------- AC5: escena.toml is the source of truth ----------

#[test]
fn ac5_load_save_load_is_the_identity() {
    let reg = Registro::integrados();
    let e = Escena::parse(ESCENA).unwrap();
    let texto = e.a_toml(&reg);
    assert_eq!(Escena::parse(&texto).unwrap(), e);
    assert_eq!(
        Escena::parse(&texto).unwrap().a_toml(&reg),
        texto,
        "saving twice, other bytes"
    );
    // params in the gadget's declared order, not alphabetical
    let (m, x) = (
        texto.find("masa =").unwrap(),
        texto.find("x = 300.0").unwrap(),
    );
    assert!(m < x, "{texto}");
    assert!(texto.contains(r#"valor = { de = "brazo.tau" }"#), "{texto}");
}

#[test]
fn ac5_a_change_from_the_studio_lands_in_the_file() {
    let reg = Registro::integrados();
    let mut e = Escena::parse(ESCENA).unwrap();
    e.fijar(&reg, "brazo", "masa", Dato::Num(25.0), None)
        .unwrap();
    e.fijar(&reg, "brazo", "escala", Dato::Num(900.0), None)
        .unwrap();
    e.fijar(
        &reg,
        "tau",
        "valor",
        Dato::Num(10.0),
        Some(Procedencia::Supuesto),
    )
    .unwrap();
    let texto = e.a_toml(&reg);
    assert!(
        texto.contains(r#"masa = { valor = 25.0, procedencia = "supuesto", nota = "la mancuerna que el reel declara" }"#),
        "{texto}"
    );
    assert!(texto.contains("escala = 900.0"), "{texto}");
    assert!(
        texto.contains(r#"valor = { valor = 10.0, procedencia = "supuesto" }"#),
        "{texto}"
    );
    let e2 = Escena::parse(&texto).unwrap();
    assert_eq!(e2, e);
    let err = e
        .fijar(&reg, "brazo", "masa", Dato::Num(900.0), None)
        .unwrap_err();
    assert!(err.contains("[0, 500] kg"), "{err}");
    assert!(e
        .fijar(&reg, "nadie", "masa", Dato::Num(1.0), None)
        .is_err());
}

#[test]
fn ac5_the_example_in_the_spec_parses() {
    let spec = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/specs/0015-studio.md");
    let texto = std::fs::read_to_string(spec).unwrap();
    let ini = texto
        .find("```toml\n# escena.toml")
        .expect("the spec shows an escena.toml")
        + 8;
    let fin = ini + texto[ini..].find("```").unwrap();
    let e = Escena::parse(&texto[ini..fin]).unwrap();
    assert_eq!(e.gadgets.len(), 2);
}
