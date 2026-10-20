//! «palanca»: a weight hanging from a lever, τ = F·L·sen θ (R-0015 AC2).
//!
//! Generalised from fbf's curl reels (`motor/src/escena.rs`, `palanca_con`;
//! `motor/src/curl.rs`): the lever, the weight as an arrow, the line of
//! action dashed, the moment arm as a dimension line from the pivot to that
//! line, and one label placed where it touches none of them.
//!
//! The physics is physics-lab's `mecanica::palanca` (the lever every gym
//! reel shares): the weight is `peso(masa)`, the moment arm `brazo` and the
//! torque `momento`, with gravity as the line of action. Nothing here
//! re-derives it: θ only places the hand.
//!
//! θ is the lever's angle from the vertical through the pivot (0: hanging,
//! 90°: horizontal), so for a forearm it is the arm's angle plus the elbow's
//! (fbf's α + φ).

use mecanica::palanca::{brazo, momento, peso, Linea};
use motoreel::Align;

use crate::dibujo::{self, colocar, Obstaculo, P};
use crate::gadget::{Categoria, Contexto, Gadget, Oraculo, Salida};
use crate::valor::{ParamSpec, Params, SalidaSpec, Tipo, Valor};

/// The gadget.
pub struct Palanca;

const PARAMS: &[ParamSpec] = &[
    ParamSpec {
        nombre: "masa",
        doc: "la carga que cuelga",
        tipo: Tipo::Num {
            min: 0.0,
            max: 500.0,
            paso: 0.5,
            unidad: "kg",
            defecto: 20.0,
        },
        fisico: true,
    },
    ParamSpec {
        nombre: "brazo",
        doc: "L: del pivote a donde cuelga la carga",
        tipo: Tipo::Num {
            min: 0.01,
            max: 2.0,
            paso: 0.01,
            unidad: "m",
            defecto: 0.32,
        },
        fisico: true,
    },
    ParamSpec {
        nombre: "angulo",
        doc: "θ: la palanca contra la vertical (0 cuelga, 90 horizontal)",
        tipo: Tipo::Num {
            min: -180.0,
            max: 180.0,
            paso: 1.0,
            unidad: "°",
            defecto: 90.0,
        },
        fisico: true,
    },
    ParamSpec {
        nombre: "x",
        doc: "el pivote, px desde la izquierda",
        tipo: Tipo::Num {
            min: 0.0,
            max: 4000.0,
            paso: 1.0,
            unidad: "px",
            defecto: 300.0,
        },
        fisico: false,
    },
    ParamSpec {
        nombre: "y",
        doc: "el pivote, px desde arriba",
        tipo: Tipo::Num {
            min: 0.0,
            max: 4000.0,
            paso: 1.0,
            unidad: "px",
            defecto: 700.0,
        },
        fisico: false,
    },
    ParamSpec {
        nombre: "escala",
        doc: "px por metro de la palanca",
        tipo: Tipo::Num {
            min: 50.0,
            max: 5000.0,
            paso: 10.0,
            unidad: "px/m",
            defecto: 1000.0,
        },
        fisico: false,
    },
    ParamSpec {
        nombre: "escala_fuerza",
        doc: "px por newton de la flecha del peso",
        tipo: Tipo::Num {
            min: 0.0,
            max: 10.0,
            paso: 0.05,
            unidad: "px/N",
            defecto: 0.8,
        },
        fisico: false,
    },
    ParamSpec {
        nombre: "etiqueta",
        doc: "qué dice la etiqueta: el torque, el brazo de momento o nada",
        tipo: Tipo::Opcion {
            opciones: &["tau", "d", "ninguna"],
            defecto: "tau",
        },
        fisico: false,
    },
    ParamSpec {
        nombre: "tam",
        doc: "el tamaño de la etiqueta",
        tipo: Tipo::Num {
            min: 16.0,
            max: 160.0,
            paso: 1.0,
            unidad: "px",
            defecto: 52.0,
        },
        fisico: false,
    },
    ParamSpec {
        nombre: "color_palanca",
        doc: "la palanca y el pivote",
        tipo: Tipo::Token { defecto: "texto" },
        fisico: false,
    },
    ParamSpec {
        nombre: "color_fuerza",
        doc: "la flecha del peso",
        tipo: Tipo::Token { defecto: "acento" },
        fisico: false,
    },
    ParamSpec {
        nombre: "color_cota",
        doc: "la línea de acción, el brazo de momento y la etiqueta",
        tipo: Tipo::Token {
            defecto: "secundario",
        },
        fisico: false,
    },
];

const SALIDAS: &[SalidaSpec] = &[
    SalidaSpec {
        nombre: "fuerza",
        unidad: "N",
        doc: "F = m·g",
    },
    SalidaSpec {
        nombre: "d",
        unidad: "m",
        doc: "el brazo de momento: del pivote a la línea de acción",
    },
    SalidaSpec {
        nombre: "tau",
        unidad: "N·m",
        doc: "|τ| = F·d = F·L·sen θ",
    },
];

/// The model's numbers at θ: (F, d, |τ|) and the hand, in metres with y up
/// and the pivot at the origin (physics-lab's frame).
fn modelo(p: &Params) -> (f64, f64, f64, (f64, f64)) {
    let (l, th) = (p.num("brazo"), p.num("angulo").to_radians());
    let mano = (l * th.sin(), -l * th.cos());
    let f = peso(p.num("masa"));
    let d = brazo((0.0, 0.0), mano, Linea::Vertical);
    let tau = momento((0.0, 0.0), mano, f, Linea::Vertical).abs();
    (f, d, tau, mano)
}

/// A number and its unit with the narrow no-break space between them.
fn cifra(x: f64, decimales: usize, unidad: &str) -> String {
    format!("{x:.decimales$}\u{202F}{unidad}")
}

impl Gadget for Palanca {
    fn tipo(&self) -> &'static str {
        "palanca"
    }

    fn categoria(&self) -> Categoria {
        Categoria::Fisica
    }

    fn doc(&self) -> &'static str {
        "Una carga que cuelga de una palanca: τ = F·L·sen θ, con el brazo de momento como cota"
    }

    fn params(&self) -> &'static [ParamSpec] {
        PARAMS
    }

    fn salidas(&self) -> &'static [SalidaSpec] {
        SALIDAS
    }

    fn eval(&self, _t: f64, p: &Params, cx: &Contexto) -> Salida {
        let (f, d, tau, mano) = modelo(p);
        let c = &cx.canvas;
        let (s, sf, tam) = (p.num("escala"), p.num("escala_fuerza"), p.num("tam"));
        let piv: P = (p.num("x"), p.num("y"));
        // y down on the canvas
        let m: P = (piv.0 + mano.0 * s, piv.1 - mano.1 * s);
        let e: P = (m.0, piv.1);
        let punta: P = (m.0, m.1 + f * sf);
        let (cp, cf, cc) = (
            cx.color(p.texto("color_palanca")),
            cx.color(p.texto("color_fuerza")),
            cx.color(p.texto("color_cota")),
        );
        let mut out = Salida::default();

        // the line of action through the hand, up to the pivot's height and past it
        if (m.1 - piv.1).abs() > 10.0 {
            let fin = piv.1 + 24.0 * (piv.1 - m.1).signum();
            out.prims
                .extend(dibujo::discontinua(c, m, (m.0, fin), 3.0, 14.0, 10.0, cc));
        }
        // the moment arm: pivot → line of action, with its end ticks
        let con_cota = (m.0 - piv.0).abs() >= 8.0;
        if con_cota {
            out.prims.push(dibujo::linea(c, piv, e, 4.0, cc));
            for q in [piv, e] {
                out.prims.push(dibujo::linea(
                    c,
                    (q.0, q.1 - 12.0),
                    (q.0, q.1 + 12.0),
                    4.0,
                    cc,
                ));
            }
        }
        // the lever and its pivot, the weight on top
        out.prims.push(dibujo::linea(c, piv, m, 22.0, cp));
        out.prims.push(dibujo::disco(c, piv, 16.0, 12, cp));
        out.prims
            .push(dibujo::disco(c, piv, 7.0, 12, cx.color("fondo")));
        out.prims.extend(dibujo::flecha(c, m, punta, 8.0, 28.0, cf));

        // the label, where it touches nothing
        let etiqueta = match p.texto("etiqueta") {
            "tau" => Some(format!("τ = {}", cifra(tau, 1, "N·m"))),
            "d" => Some(format!("d = {}", cifra(d * 100.0, 0, "cm"))),
            _ => None,
        };
        if let Some(et) = etiqueta {
            let medido = cx.ancho(&et, tam);
            // without faces the box is estimated, so the asserts still run
            let w = medido.unwrap_or(0.55 * tam * et.chars().count() as f64);
            let h = tam;
            let sg = if m.0 >= piv.0 { 1.0 } else { -1.0 };
            let mid = (piv.0 + m.0) / 2.0;
            let cands = [
                (e.0 + sg * (24.0 + w / 2.0), piv.1),
                (mid, piv.1 - 0.9 * tam),
                (mid, piv.1 + 1.1 * tam),
                (piv.0 - sg * (36.0 + w / 2.0), piv.1),
                (mid, piv.1 - 2.0 * tam),
                (mid, piv.1 + 2.2 * tam),
            ];
            let mut obst = vec![
                Obstaculo {
                    a: piv,
                    b: m,
                    holgura: 22.0,
                },
                Obstaculo {
                    a: m,
                    b: punta,
                    holgura: 18.0,
                },
                Obstaculo {
                    a: m,
                    b: (m.0, piv.1),
                    holgura: 10.0,
                },
            ];
            if con_cota {
                obst.push(Obstaculo {
                    a: piv,
                    b: e,
                    holgura: 10.0,
                });
            }
            match colocar(&cands, w, h, &obst, cx.seguro) {
                Some(q) if medido.is_some() => out.prims.push(dibujo::texto(
                    c,
                    &et,
                    (q.0, q.1 + 0.35 * tam),
                    tam,
                    Align::Center,
                    cx.cara,
                    cc,
                )),
                Some(_) => out
                    .avisos
                    .push(format!("palanca: sin caras, la etiqueta «{et}» no se dibuja")),
                None => out.avisos.push(format!(
                    "palanca: la etiqueta «{et}» no cabe sin tocar el dibujo ni salir del área segura"
                )),
            }
        }
        let [x0, y0, x1, y1] = cx.seguro;
        if !(m.0 >= x0 && m.0 <= x1 && m.1 >= y0 && m.1 <= y1) {
            out.avisos
                .push("palanca: la carga sale del área segura".to_string());
        }

        let l = p.num("brazo");
        if d >= 0.999 * l {
            out.eventos.push("maximo".to_string());
        }
        if d <= 0.001 * l {
            out.eventos.push("cero".to_string());
        }
        out.valores = vec![
            Valor::calculado("fuerza", f, "N", "F = m·g, g = 9.81 m/s²"),
            Valor::calculado("d", d, "m", "d = L·sen θ"),
            Valor::calculado("tau", tau, "N·m", "τ = F·L·sen θ (cuasi-estático, 2D)"),
        ];
        out
    }

    fn comprobar(&self, p: &Params, _cx: &Contexto) -> Vec<String> {
        let largo = peso(p.num("masa")) * p.num("escala_fuerza");
        if largo > 900.0 {
            vec![format!(
                "palanca: la flecha del peso mide {largo:.0} px; baja escala_fuerza"
            )]
        } else {
            Vec::new()
        }
    }

    fn oraculos(&self) -> Vec<Oraculo> {
        let fbf =
            "fbf, el modelo del reel 47 (motor/src/curl.rs): F·L = 196.2 N × 0.32 m = 62.784 N·m";
        vec![
            Oraculo {
                nombre: "el pico del curl: F·L con 20 kg y 0.32 m",
                params: vec![("masa", 20.0), ("brazo", 0.32), ("angulo", 90.0)],
                salida: "tau",
                esperado: 62.784,
                tolerancia: 1e-9,
                fuente: fbf,
            },
            Oraculo {
                nombre: "el peso de la mancuerna",
                params: vec![("masa", 20.0)],
                salida: "fuerza",
                esperado: 196.2,
                tolerancia: 1e-9,
                fuente: fbf,
            },
            Oraculo {
                nombre: "a plomo no hay palanca",
                params: vec![("masa", 20.0), ("brazo", 0.32), ("angulo", 0.0)],
                salida: "tau",
                esperado: 0.0,
                tolerancia: 1e-12,
                fuente: "mecanica::palanca, a_plomo_no_hay_palanca",
            },
            Oraculo {
                nombre: "a 30°, la mitad del pico",
                params: vec![("masa", 20.0), ("brazo", 0.32), ("angulo", 30.0)],
                salida: "tau",
                esperado: 31.392,
                tolerancia: 1e-9,
                fuente: "62.784 × sen 30°, a mano",
            },
            Oraculo {
                nombre: "el brazo de momento horizontal es L",
                params: vec![("brazo", 0.32), ("angulo", -90.0)],
                salida: "d",
                esperado: 0.32,
                tolerancia: 1e-12,
                fuente: "d = L·|sen θ|, a mano",
            },
        ]
    }
}
