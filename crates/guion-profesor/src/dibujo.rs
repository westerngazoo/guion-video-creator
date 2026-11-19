//! One frame of the class as flat primitives: header, the stage where the
//! teacher runs the simulation, the board she writes on, the subtitle, the
//! guide's voice meter and the chapter bar. A frame is a pure function of
//! `(class, timeline, t)`.

use motoreel::{Align, Fill, Prim2, Pt2, Rgb, Style, Subpath};

use crate::clase::{Clase, Escena};
use crate::fisica::{Frenado, Plano, Tiro};
use crate::linea::{Cue, Linea, DEMO_LEAD, FONDO};

/// The centred image-space window: 16 × 9 units, y up.
pub const VIEW: (f64, f64) = (16.0, 9.0);
pub const FACE: usize = 0;
pub const BOLD: usize = 1;

const BG: Rgb = rgb(0x0b, 0x10, 0x20);
const PANEL: Rgb = rgb(0x12, 0x19, 0x2e);
const BOARD: Rgb = rgb(0x14, 0x2a, 0x22);
const CHALK: Rgb = rgb(0xec, 0xf2, 0xe8);
const WHITE: Rgb = rgb(0xee, 0xf0, 0xf6);
const MUTED: Rgb = rgb(0x8a, 0x93, 0xa5);
const GRAY: Rgb = rgb(0x78, 0x82, 0x91);
const DIM: Rgb = rgb(0x34, 0x3c, 0x4c);
const AMBER: Rgb = rgb(0xe8, 0xa0, 0x2a);
const BLUE: Rgb = rgb(0x4e, 0x98, 0xe8);
const RED: Rgb = rgb(0xe2, 0x54, 0x48);
const GREEN: Rgb = rgb(0x56, 0xc4, 0x78);

pub const fn rgb(r: u8, g: u8, b: u8) -> Rgb {
    Rgb { r, g, b }
}
pub fn background() -> Rgb {
    BG
}

fn p(x: f64, y: f64) -> Pt2 {
    Pt2 { x, y }
}
fn stroke(c: Rgb, w: f64) -> Style {
    Style {
        stroke: c,
        width: w,
        alpha: 1.0,
        fill: None,
    }
}
fn fill(c: Rgb, alpha: f64) -> Style {
    Style {
        stroke: c,
        width: 0.0,
        alpha: 1.0,
        fill: Some(Fill::solid(c, alpha)),
    }
}
fn poly(pts: &[Pt2], style: Style) -> Prim2 {
    let mut s = Subpath::new(pts[0]);
    for q in &pts[1..] {
        s = s.line_to(*q);
    }
    Prim2::Path {
        subpaths: vec![s.close()],
        style,
    }
}
fn rect(x0: f64, y0: f64, x1: f64, y1: f64, style: Style) -> Prim2 {
    poly(&[p(x0, y0), p(x1, y0), p(x1, y1), p(x0, y1)], style)
}
fn disc(c: Pt2, r: f64, col: Rgb) -> Prim2 {
    let pts: Vec<Pt2> = (0..16)
        .map(|i| {
            let a = i as f64 * std::f64::consts::TAU / 16.0;
            p(c.x + r * a.cos(), c.y + r * a.sin())
        })
        .collect();
    poly(&pts, fill(col, 1.0))
}
fn seg(a: Pt2, b: Pt2, style: Style) -> Prim2 {
    Prim2::Segment { a, b, style }
}
fn text(at: Pt2, s: &str, size: f64, align: Align, face: usize, col: Rgb) -> Prim2 {
    Prim2::Text {
        at,
        text: s.to_string(),
        size,
        align,
        face,
        style: stroke(col, 0.0),
    }
}

/// Greedy wrap by an average glyph width of half an em.
pub fn wrap(s: &str, width: f64, size: f64) -> Vec<String> {
    let max = ((width / (0.5 * size)).floor() as usize).max(8);
    let mut lines = Vec::new();
    let mut cur = String::new();
    for w in s.split_whitespace() {
        if !cur.is_empty() && cur.chars().count() + 1 + w.chars().count() > max {
            lines.push(std::mem::take(&mut cur));
        }
        if !cur.is_empty() {
            cur.push(' ');
        }
        cur.push_str(w);
    }
    if !cur.is_empty() {
        lines.push(cur);
    }
    lines
}

/// Maps a world rectangle (metres) into a screen rectangle, aspect kept.
struct Stage {
    s: f64,
    ox: f64,
    oy: f64,
}
impl Stage {
    fn new(world: ([f64; 2], [f64; 2]), screen: ([f64; 2], [f64; 2])) -> Stage {
        let (wx, wy) = world;
        let (sx, sy) = screen;
        let s = ((sx[1] - sx[0]) / (wx[1] - wx[0])).min((sy[1] - sy[0]) / (wy[1] - wy[0]));
        let ox = (sx[0] + sx[1]) / 2.0 - s * (wx[0] + wx[1]) / 2.0;
        let oy = (sy[0] + sy[1]) / 2.0 - s * (wy[0] + wy[1]) / 2.0;
        Stage { s, ox, oy }
    }
    fn at(&self, x: f64, y: f64) -> Pt2 {
        p(self.ox + self.s * x, self.oy + self.s * y)
    }
}

const STAGE: ([f64; 2], [f64; 2]) = ([-7.6, 1.7], [-2.55, 2.85]);
const BOARD_X: [f64; 2] = [2.0, 7.6];

/// The demonstration on stage at `t`: the last one started in this chapter
/// at or before the beat on screen, and how long it has been running.
fn demo_at<'a>(
    clase: &'a Clase,
    linea: &Linea,
    t: f64,
) -> Option<(&'a std::collections::BTreeMap<String, f64>, f64)> {
    let now = linea.tramo_en(t);
    let chapter = linea.tramos[now].capitulo;
    (0..=now)
        .rev()
        .take_while(|&i| linea.tramos[i].capitulo == chapter)
        .find_map(|i| {
            clase.beat[i]
                .demo
                .as_ref()
                .map(|d| (d, (t - linea.tramos[i].t0 - DEMO_LEAD).max(0.0)))
        })
}

pub fn frame(clase: &Clase, linea: &Linea, cues: &[Cue], voz: &[i16], t: f64) -> Vec<Prim2> {
    let mut out = Vec::new();
    let i = linea.tramo_en(t);
    let tr = &linea.tramos[i];
    let chapter = &linea.capitulos[tr.capitulo];

    // header
    out.push(text(
        p(-7.6, 3.95),
        &clase.class.kicker.to_uppercase(),
        0.26,
        Align::Left,
        BOLD,
        AMBER,
    ));
    out.push(text(
        p(-7.6, 3.3),
        &chapter.title,
        0.52,
        Align::Left,
        BOLD,
        WHITE,
    ));
    out.push(text(
        p(7.6, 3.95),
        &format!("Capítulo {} de {}", tr.capitulo + 1, linea.capitulos.len()),
        0.24,
        Align::Right,
        FACE,
        MUTED,
    ));

    // stage
    out.push(rect(
        STAGE.0[0],
        STAGE.1[0],
        STAGE.0[1],
        STAGE.1[1],
        fill(PANEL, 1.0),
    ));
    escena(&mut out, clase, demo_at(clase, linea, t));

    // board
    pizarra(&mut out, clase, linea, i, t);

    // subtitle
    if let Some(c) = cues.iter().find(|c| t >= c.t0 && t < c.t1 + 0.25) {
        let lines = wrap(&c.text, 14.0, 0.34);
        let n = lines.len().min(2);
        for (k, l) in lines.iter().take(2).enumerate() {
            let y = -3.05 - 0.46 * (k as f64) + 0.23 * (n as f64 - 1.0);
            out.push(text(p(0.0, y), l, 0.34, Align::Center, FACE, WHITE));
        }
    }

    // the guide and her voice
    let lvl = nivel(voz, t);
    out.push(disc(p(-7.35, -4.02), 0.2, AMBER));
    out.push(text(
        p(-7.35, -4.11),
        &clase.class.guide[..1],
        0.24,
        Align::Center,
        BOLD,
        BG,
    ));
    out.push(text(
        p(-7.05, -4.11),
        &clase.class.guide,
        0.26,
        Align::Left,
        BOLD,
        WHITE,
    ));
    for k in 0..5 {
        let h = 0.06 + 0.32 * (lvl * (1.0 - 0.15 * ((k as f64) - 2.0).abs())).clamp(0.0, 1.0);
        let x = -6.15 + 0.13 * k as f64;
        out.push(rect(
            x,
            -4.05 - h / 2.0,
            x + 0.07,
            -4.05 + h / 2.0,
            fill(AMBER, 0.9),
        ));
    }

    // chapter bar
    let (x0, x1, y) = (-5.2, 7.6, -4.05);
    out.push(rect(x0, y - 0.04, x1, y + 0.04, fill(DIM, 1.0)));
    let f = (t / linea.duracion).clamp(0.0, 1.0);
    out.push(rect(
        x0,
        y - 0.04,
        x0 + (x1 - x0) * f,
        y + 0.04,
        fill(RED, 1.0),
    ));
    for c in &linea.capitulos {
        let x = x0 + (x1 - x0) * c.t / linea.duracion;
        out.push(rect(
            x - 0.025,
            y - 0.12,
            x + 0.025,
            y + 0.12,
            fill(WHITE, 0.8),
        ));
    }
    out
}

/// Loudness around `t`, 0..1, from the narration track itself.
fn nivel(voz: &[i16], t: f64) -> f64 {
    let rate = f64::from(crate::voz::RATE);
    let c = (t * rate) as usize;
    let w = (0.05 * rate) as usize;
    let a = c.saturating_sub(w).min(voz.len());
    let b = (c + w).min(voz.len());
    if b <= a {
        return 0.0;
    }
    let rms =
        (voz[a..b].iter().map(|s| f64::from(*s).powi(2)).sum::<f64>() / (b - a) as f64).sqrt();
    (rms / 6000.0).min(1.0)
}

fn pizarra(out: &mut Vec<Prim2>, clase: &Clase, linea: &Linea, i: usize, t: f64) {
    out.push(rect(
        BOARD_X[0],
        STAGE.1[0],
        BOARD_X[1],
        STAGE.1[1],
        fill(BOARD, 1.0),
    ));
    out.push(rect(
        BOARD_X[0],
        STAGE.1[0],
        BOARD_X[1],
        STAGE.1[0] + 0.08,
        fill(rgb(0x5a, 0x44, 0x2c), 1.0),
    ));
    out.push(text(
        p(BOARD_X[0] + 0.3, 2.45),
        "PIZARRA",
        0.2,
        Align::Left,
        BOLD,
        rgb(0x8f, 0xb8, 0x9f),
    ));
    let tr = &linea.tramos[i];
    // the question card replaces the board once it is asked
    if let Some(q) = &clase.beat[i].pregunta {
        let mut y = 1.9;
        out.push(text(
            p(BOARD_X[0] + 0.3, y),
            "Pregunta",
            0.34,
            Align::Left,
            BOLD,
            AMBER,
        ));
        y -= 0.5;
        for l in wrap(&q.prompt, 5.0, 0.28) {
            out.push(text(
                p(BOARD_X[0] + 0.3, y),
                &l,
                0.28,
                Align::Left,
                FACE,
                CHALK,
            ));
            y -= 0.4;
        }
        y -= 0.15;
        if t >= tr.habla - 0.5 {
            for (k, o) in q.opciones.iter().enumerate() {
                let tag = (b'A' + k as u8) as char;
                out.push(text(
                    p(BOARD_X[0] + 0.45, y),
                    &format!("{tag})  {o}"),
                    0.3,
                    Align::Left,
                    FACE,
                    CHALK,
                ));
                y -= 0.45;
            }
            out.push(text(
                p(BOARD_X[0] + 0.3, STAGE.1[0] + 0.3),
                "El video se pausa: responde en el aula.",
                0.2,
                Align::Left,
                FACE,
                MUTED,
            ));
        }
        return;
    }
    // every line written in this chapter so far, the current beat's one by one
    let mut lines: Vec<&str> = Vec::new();
    for (j, b) in clase.beat.iter().enumerate().take(i + 1) {
        if linea.tramos[j].capitulo != tr.capitulo {
            continue;
        }
        let n = b.pizarra.len();
        for (k, l) in b.pizarra.iter().enumerate() {
            let shown = j < i || {
                let span = (linea.tramos[j].habla - linea.tramos[j].t0).max(0.5);
                t >= linea.tramos[j].t0 + span * (k as f64) / (n as f64)
            };
            if shown {
                lines.push(l);
            }
        }
    }
    let mut y = 1.9;
    for l in lines.iter().rev().take(8).rev() {
        for w in wrap(l, 5.0, 0.3) {
            out.push(text(
                p(BOARD_X[0] + 0.3, y),
                &w,
                0.3,
                Align::Left,
                FACE,
                CHALK,
            ));
            y -= 0.44;
        }
        y -= 0.1;
    }
}

fn escena(
    out: &mut Vec<Prim2>,
    clase: &Clase,
    demo: Option<(&std::collections::BTreeMap<String, f64>, f64)>,
) {
    let get = |k: &str, d: f64| demo.and_then(|(m, _)| m.get(k).copied()).unwrap_or(d);
    let tau = demo.map(|(_, t)| t);
    let label_at = |out: &mut Vec<Prim2>, s: &str| {
        out.push(text(
            p(STAGE.0[0] + 0.3, STAGE.1[1] - 0.45),
            s,
            0.28,
            Align::Left,
            FACE,
            WHITE,
        ));
    };
    match &clase.escena {
        Escena::Frenado {
            v0,
            bandera,
            acantilado,
        } => {
            let st = Stage::new(([-2.0, acantilado + 6.0], [-FONDO - 0.6, 4.0]), STAGE);
            out.push(seg(
                st.at(-1.5, 0.0),
                st.at(*acantilado, 0.0),
                stroke(GRAY, 0.05),
            ));
            out.push(seg(
                st.at(*acantilado, 0.0),
                st.at(*acantilado, -FONDO),
                stroke(GRAY, 0.05),
            ));
            out.push(seg(
                st.at(*bandera, 0.0),
                st.at(*bandera, 2.6),
                stroke(AMBER, 0.05),
            ));
            out.push(poly(
                &[
                    st.at(*bandera, 2.6),
                    st.at(bandera + 1.3, 2.15),
                    st.at(*bandera, 1.7),
                ],
                fill(AMBER, 1.0),
            ));
            out.push(text(
                st.at(*bandera, 3.0),
                &format!("{bandera:.0} m"),
                0.24,
                Align::Center,
                FACE,
                WHITE,
            ));
            let a = get("a", 0.0);
            let (x, y, v, frenando, fin) = match tau {
                Some(tau) if a > 0.0 => {
                    let f = Frenado {
                        v0: *v0,
                        a,
                        borde: *acantilado,
                        fondo: FONDO,
                    };
                    let c = f.en(tau);
                    (
                        c.x,
                        c.y,
                        c.v,
                        c.frenando,
                        (tau >= f.duracion()).then_some(f),
                    )
                }
                _ => (0.0, 0.0, *v0, false, None),
            };
            let body = [
                st.at(x - 1.6, y + 0.35),
                st.at(x, y + 0.35),
                st.at(x, y + 1.3),
                st.at(x - 1.6, y + 1.3),
            ];
            out.push(poly(&body, fill(BLUE, 1.0)));
            out.push(disc(st.at(x - 1.25, y + 0.25), 0.1, WHITE));
            out.push(disc(st.at(x - 0.35, y + 0.25), 0.1, WHITE));
            if frenando {
                out.push(seg(
                    st.at(x - 0.4, 2.0),
                    st.at(x - 2.6, 2.0),
                    stroke(RED, 0.05),
                ));
                out.push(poly(
                    &[
                        st.at(x - 2.9, 2.0),
                        st.at(x - 2.5, 2.25),
                        st.at(x - 2.5, 1.75),
                    ],
                    fill(RED, 1.0),
                ));
                out.push(text(
                    st.at(x - 1.6, 2.4),
                    &format!("a = {a:.0} m/s²"),
                    0.22,
                    Align::Center,
                    FACE,
                    RED,
                ));
            }
            if a > 0.0 {
                label_at(
                    out,
                    &format!("v₀ = {v0:.0} m/s   ·   a = {a:.0} m/s²   ·   v = {v:.1} m/s"),
                );
            } else {
                label_at(out, &format!("v₀ = {v0:.0} m/s"));
            }
            if let Some(f) = fin {
                let d = f.distancia();
                let msg = if f.cae() {
                    "¡Al precipicio!".to_string()
                } else {
                    format!("se detuvo a {d:.1} m")
                };
                out.push(seg(
                    st.at(0.0, -0.7),
                    st.at(x.min(*acantilado), -0.7),
                    stroke(AMBER, 0.03),
                ));
                out.push(text(
                    st.at((x.min(*acantilado)) / 2.0, -1.4),
                    &msg,
                    0.3,
                    Align::Center,
                    BOLD,
                    if f.cae() { RED } else { GREEN },
                ));
            }
        }
        Escena::Tiro {
            blanco,
            muralla,
            zona,
            alcance,
        } => {
            let st = Stage::new(([-2.0, alcance + 2.0], [-2.5, alcance * 0.5]), STAGE);
            out.push(seg(
                st.at(-1.5, 0.0),
                st.at(*alcance, 0.0),
                stroke(GRAY, 0.05),
            ));
            if let Some(b) = blanco {
                out.push(rect(
                    st.at(b - 1.0, 0.0).x,
                    st.at(0.0, 0.0).y,
                    st.at(b + 1.0, 0.0).x,
                    st.at(0.0, 1.4).y,
                    fill(RED, 1.0),
                ));
                out.push(text(
                    st.at(*b, 2.4),
                    &format!("blanco · {b:.0} m"),
                    0.22,
                    Align::Center,
                    FACE,
                    WHITE,
                ));
            }
            if let Some(m) = muralla {
                out.push(rect(
                    st.at(m.x, 0.0).x,
                    st.at(0.0, 0.0).y,
                    st.at(m.x + m.ancho, 0.0).x,
                    st.at(0.0, m.alto).y,
                    fill(DIM, 1.0),
                ));
                out.push(text(
                    st.at(m.x + m.ancho / 2.0, m.alto + 1.0),
                    &format!("muralla · {:.0} m", m.alto),
                    0.22,
                    Align::Center,
                    FACE,
                    WHITE,
                ));
            }
            if let Some(z) = zona {
                out.push(rect(
                    st.at(z[0], 0.0).x,
                    st.at(0.0, -0.25).y,
                    st.at(z[1], 0.0).x,
                    st.at(0.0, 0.25).y,
                    fill(GREEN, 1.0),
                ));
            }
            let (th, v0) = (get("th", 45.0), get("v0", 15.0));
            let r = th.to_radians();
            out.push(seg(
                st.at(0.0, 0.0),
                st.at(2.6 * r.cos(), 2.6 * r.sin()),
                stroke(WHITE, 0.1),
            ));
            label_at(out, &format!("θ = {th:.0}°   ·   v₀ = {v0:.1} m/s"));
            if let Some(tau) = tau.filter(|_| demo.is_some()) {
                let sh = Tiro { th, v0 };
                let end = muralla
                    .and_then(|m| sh.choca(m.x, m.alto))
                    .unwrap_or_else(|| sh.vuelo());
                let t1 = tau.min(end);
                let pts: Vec<Pt2> = (0..=60)
                    .map(|k| {
                        let q = sh.en(t1 * k as f64 / 60.0);
                        st.at(q[0], q[1])
                    })
                    .collect();
                out.push(Prim2::Polyline {
                    points: pts,
                    style: stroke(AMBER, 0.05),
                });
                let q = sh.en(t1);
                out.push(disc(st.at(q[0], q[1]), 0.12, WHITE));
                if tau >= end {
                    let hit = muralla.is_some_and(|m| sh.choca(m.x, m.alto).is_some());
                    let msg = if hit {
                        "¡Chocó con la muralla!".to_string()
                    } else {
                        format!("cayó a {:.1} m", sh.alcance())
                    };
                    out.push(text(
                        st.at(q[0].max(4.0), -1.5),
                        &msg,
                        0.3,
                        Align::Center,
                        BOLD,
                        if hit { RED } else { GREEN },
                    ));
                }
            }
        }
        Escena::Plano { largo, mu } => {
            let st = Stage::new(([-1.0, largo + 2.0], [-2.0, largo * 0.75]), STAGE);
            let th = get("th", 15.0);
            let r = th.to_radians();
            let (sn, cs) = (r.sin(), r.cos());
            out.push(seg(
                st.at(-0.5, 0.0),
                st.at(largo + 1.5, 0.0),
                stroke(GRAY, 0.05),
            ));
            let top = [0.0, largo * sn];
            out.push(poly(
                &[
                    st.at(0.0, 0.0),
                    st.at(largo * cs, 0.0),
                    st.at(top[0], top[1]),
                ],
                fill(DIM, 1.0),
            ));
            let pl = Plano {
                th,
                mu: *mu,
                recorrido: largo - 1.0,
            };
            let along = 0.5 + tau.map(|t| pl.s(t)).unwrap_or(0.0);
            let (u, n) = ([cs, -sn], [sn, cs]);
            let c = [top[0] + u[0] * along, top[1] + u[1] * along];
            let k = |du: f64, dn: f64| {
                st.at(c[0] + u[0] * du + n[0] * dn, c[1] + u[1] * du + n[1] * dn)
            };
            out.push(poly(
                &[k(-0.45, 0.0), k(0.45, 0.0), k(0.45, 0.9), k(-0.45, 0.9)],
                fill(BLUE, 1.0),
            ));
            label_at(out, &format!("θ = {th:.0}°   ·   μ = {mu}"));
            if let Some(tau) = tau {
                if tau >= pl.duracion() {
                    let msg = if pl.a() > 0.0 {
                        format!("a = {:.2} m/s²", pl.a())
                    } else {
                        "no se mueve: la fricción lo sujeta".into()
                    };
                    out.push(text(
                        st.at(largo / 2.0, -1.2),
                        &msg,
                        0.3,
                        Align::Center,
                        BOLD,
                        if pl.a() > 0.0 { GREEN } else { RED },
                    ));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::wrap;

    #[test]
    fn wrapping_keeps_words_whole() {
        let l = wrap("uno dos tres cuatro cinco seis siete", 2.0, 0.4);
        assert!(l.len() > 1);
        assert_eq!(l.join(" "), "uno dos tres cuatro cinco seis siete");
    }
}
