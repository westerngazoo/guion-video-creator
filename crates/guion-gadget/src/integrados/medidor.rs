//! «medidor»: a horizontal bar gauge (R-0015 AC2).
//!
//! Generalised from Rotor's `Rf::medidor` (the g gauge of 1.2 and 1.3) and
//! fbf's τ gauge (reel 47): a track from 0 to `maximo`, filled up to the
//! value, an optional fixed mark, the reading above and a caption below.
//!
//! The gauge computes nothing physical: its `lectura` keeps the provenance
//! of the value it shows (bind `valor = { de = "brazo.tau" }` and the
//! reading is *calculado*; type a measured number with its source and it is
//! *medido*).

use motoreel::Align;

use crate::dibujo;
use crate::gadget::{Categoria, Contexto, Gadget, Oraculo, Salida};
use crate::valor::{ParamSpec, Params, Procedencia, SalidaSpec, Tipo, Valor};

/// The gadget.
pub struct Medidor;

const fn num(
    nombre: &'static str,
    doc: &'static str,
    (min, max, paso): (f64, f64, f64),
    unidad: &'static str,
    defecto: f64,
    fisico: bool,
) -> ParamSpec {
    ParamSpec {
        nombre,
        doc,
        tipo: Tipo::Num {
            min,
            max,
            paso,
            unidad,
            defecto,
        },
        fisico,
    }
}

const PARAMS: &[ParamSpec] = &[
    num(
        "valor",
        "lo que marca (enlázalo a la salida de otro gadget)",
        (-1.0e6, 1.0e6, 0.1),
        "",
        0.0,
        true,
    ),
    num(
        "maximo",
        "el tope de la escala",
        (1e-6, 1.0e6, 1.0),
        "",
        70.0,
        false,
    ),
    num(
        "marca",
        "una raya fija en la escala (0: sin marca)",
        (0.0, 1.0e6, 1.0),
        "",
        0.0,
        false,
    ),
    ParamSpec {
        nombre: "unidad",
        doc: "la unidad de la lectura",
        tipo: Tipo::Texto { defecto: "N·m" },
        fisico: false,
    },
    num(
        "decimales",
        "decimales de la lectura",
        (0.0, 3.0, 1.0),
        "",
        0.0,
        false,
    ),
    num(
        "x",
        "el inicio de la vía, px",
        (0.0, 4000.0, 1.0),
        "px",
        140.0,
        false,
    ),
    num(
        "y",
        "el eje de la vía, px",
        (0.0, 4000.0, 1.0),
        "px",
        1300.0,
        false,
    ),
    num(
        "ancho",
        "el largo de la vía",
        (40.0, 4000.0, 1.0),
        "px",
        800.0,
        false,
    ),
    num(
        "alto",
        "el grueso de la vía",
        (4.0, 120.0, 1.0),
        "px",
        16.0,
        false,
    ),
    num(
        "tam",
        "el tamaño de la lectura",
        (16.0, 160.0, 1.0),
        "px",
        52.0,
        false,
    ),
    ParamSpec {
        nombre: "rotulo",
        doc: "lo que dice debajo («τ en el codo»)",
        tipo: Tipo::Texto { defecto: "" },
        fisico: false,
    },
    ParamSpec {
        nombre: "color_via",
        doc: "la vía vacía",
        tipo: Tipo::Token {
            defecto: "secundario",
        },
        fisico: false,
    },
    ParamSpec {
        nombre: "color_relleno",
        doc: "el relleno y la lectura",
        tipo: Tipo::Token { defecto: "texto" },
        fisico: false,
    },
    ParamSpec {
        nombre: "color_marca",
        doc: "la marca fija",
        tipo: Tipo::Token { defecto: "acento" },
        fisico: false,
    },
];

const SALIDAS: &[SalidaSpec] = &[
    SalidaSpec {
        nombre: "lectura",
        unidad: "",
        doc: "el valor que marca, con su procedencia",
    },
    SalidaSpec {
        nombre: "fraccion",
        unidad: "",
        doc: "qué tanto de la escala llena (0 a 1)",
    },
];

impl Gadget for Medidor {
    fn tipo(&self) -> &'static str {
        "medidor"
    }

    fn categoria(&self) -> Categoria {
        Categoria::Datos
    }

    fn doc(&self) -> &'static str {
        "Un medidor de barra: la vía de 0 al tope, el relleno hasta el valor y una marca fija"
    }

    fn params(&self) -> &'static [ParamSpec] {
        PARAMS
    }

    fn salidas(&self) -> &'static [SalidaSpec] {
        SALIDAS
    }

    fn eval(&self, _t: f64, p: &Params, cx: &Contexto) -> Salida {
        let c = &cx.canvas;
        let (v, max, marca) = (p.num("valor"), p.num("maximo"), p.num("marca"));
        let (x0, y, ancho, alto, tam) = (
            p.num("x"),
            p.num("y"),
            p.num("ancho"),
            p.num("alto"),
            p.num("tam"),
        );
        let frac = (v / max).clamp(0.0, 1.0);
        let x = |g: f64| x0 + ancho * (g / max).clamp(0.0, 1.0);
        let caja = |a: f64, b: f64, c: f64, d: f64| [(a, b), (c, b), (c, d), (a, d)];
        let (cv, cr, cm) = (
            cx.color(p.texto("color_via")),
            cx.color(p.texto("color_relleno")),
            cx.color(p.texto("color_marca")),
        );
        let mut out = Salida::default();
        let h = alto / 2.0;
        out.prims.push(dibujo::poligono(
            c,
            &caja(x0, y - h, x0 + ancho, y + h),
            cv,
            0.45,
        ));
        if frac > 0.0 {
            out.prims
                .push(dibujo::poligono(c, &caja(x0, y - h, x(v), y + h), cr, 1.0));
        }
        if marca > 0.0 {
            let xm = x(marca);
            out.prims.push(dibujo::poligono(
                c,
                &caja(xm - 3.0, y - h - 12.0, xm + 3.0, y + h + 12.0),
                cm,
                1.0,
            ));
        }
        let dec = p.num("decimales").round().clamp(0.0, 3.0) as usize;
        let lectura = format!("{v:.dec$}\u{202F}{}", p.texto("unidad"));
        let rotulo = p.texto("rotulo");
        let textos = [
            (lectura.as_str(), (x0, y - h - 30.0), tam, Align::Left, cr),
            (
                rotulo,
                (x0 + ancho / 2.0, y + h + 30.0 + 0.8 * tam),
                0.8 * tam,
                Align::Center,
                cv,
            ),
        ];
        for (s, at, tm, al, col) in textos {
            if s.is_empty() {
                continue;
            }
            if cx.ancho(s, tm).is_some() {
                out.prims
                    .push(dibujo::texto(c, s, at, tm, al, cx.cara, col));
            } else {
                out.avisos
                    .push(format!("medidor: sin caras, «{s}» no se dibuja"));
            }
        }
        if v > max * (1.0 + 1e-9) {
            out.avisos.push(format!(
                "medidor: el valor {v:.3} se sale de la escala (tope {max})"
            ));
        }
        if v < 0.0 {
            out.avisos
                .push(format!("medidor: el valor {v:.3} es negativo"));
        }
        if v >= max {
            out.eventos.push("tope".to_string());
        }
        if marca > 0.0 && v >= marca {
            out.eventos.push("marca".to_string());
        }
        out.valores = vec![
            Valor {
                nombre: "lectura".to_string(),
                cantidad: v,
                unidad: p.texto("unidad").to_string(),
                procedencia: p.procedencia("valor").unwrap_or(Procedencia::Supuesto),
                nota: "lo que marca: la procedencia es la de su valor".to_string(),
            },
            Valor::calculado("fraccion", frac, "", "valor / tope"),
        ];
        out
    }

    fn comprobar(&self, p: &Params, _cx: &Contexto) -> Vec<String> {
        let (max, marca) = (p.num("maximo"), p.num("marca"));
        if marca > max {
            vec![format!(
                "medidor: la marca {marca} queda fuera de la escala (tope {max})"
            )]
        } else {
            Vec::new()
        }
    }

    fn oraculos(&self) -> Vec<Oraculo> {
        vec![
            Oraculo {
                nombre: "el pico del curl en la escala de 70 N·m",
                params: vec![("valor", 62.784), ("maximo", 70.0)],
                salida: "fraccion",
                esperado: 62.784 / 70.0,
                tolerancia: 1e-12,
                fuente: "fbf, reel 47: pico de 62.784 N·m, escala TAU_ESCALA = 70 N·m",
            },
            Oraculo {
                nombre: "la lectura es el valor",
                params: vec![("valor", 62.784)],
                salida: "lectura",
                esperado: 62.784,
                tolerancia: 0.0,
                fuente: "el medidor no calcula: muestra",
            },
            Oraculo {
                nombre: "pasado el tope, la barra se queda llena",
                params: vec![("valor", 90.0), ("maximo", 70.0)],
                salida: "fraccion",
                esperado: 1.0,
                tolerancia: 0.0,
                fuente: "la escala se satura (y el aviso lo dice)",
            },
        ]
    }
}
