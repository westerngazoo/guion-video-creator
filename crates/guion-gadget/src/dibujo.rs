//! Drawing in canvas pixels (y down), the way a design is laid out, and
//! placing a label where nothing else is (R-0015).
//!
//! Strokes are flat filled polygons (a bar along the segment), the house's
//! low-poly style: no stroke joins or caps that a sink could antialias
//! differently. Every helper is a pure function of its arguments.

use guion_reel::Canvas;
use motoreel::shapes::polygon;
use motoreel::{Align, Fill, Prim2, Rgb, Style};

/// A point in canvas pixels, y down.
pub type P = (f64, f64);

/// A filled polygon through `pts` (px).
pub fn poligono(c: &Canvas, pts: &[P], color: Rgb, alfa: f64) -> Prim2 {
    let v: Vec<_> = pts.iter().map(|&(x, y)| c.to_view(x, y)).collect();
    Prim2::Path {
        subpaths: polygon(&v),
        // no stroke, so the edge does not double the fill's opacity
        style: Style {
            stroke: color,
            width: 0.0,
            alpha: 0.0,
            fill: Some(Fill::solid(color, alfa)),
        },
    }
}

/// The four corners of a bar `ancho` px wide from `a` to `b`.
pub fn barra(a: P, b: P, ancho: f64) -> [P; 4] {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let l = dx.hypot(dy).max(1e-9);
    let (nx, ny) = (-dy / l * ancho / 2.0, dx / l * ancho / 2.0);
    [
        (a.0 + nx, a.1 + ny),
        (b.0 + nx, b.1 + ny),
        (b.0 - nx, b.1 - ny),
        (a.0 - nx, a.1 - ny),
    ]
}

/// A straight line `ancho` px wide.
pub fn linea(c: &Canvas, a: P, b: P, ancho: f64, color: Rgb) -> Prim2 {
    poligono(c, &barra(a, b, ancho), color, 1.0)
}

/// A dashed line: dashes of `on` px every `on + off`, starting at `a`.
pub fn discontinua(
    c: &Canvas,
    a: P,
    b: P,
    ancho: f64,
    on: f64,
    off: f64,
    color: Rgb,
) -> Vec<Prim2> {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let l = dx.hypot(dy);
    let paso = on + off;
    if l < 1e-9 || paso <= 0.0 {
        return Vec::new();
    }
    let n = (l / paso).ceil() as usize;
    (0..n)
        .map(|k| {
            let s0 = k as f64 * paso;
            let s1 = (s0 + on).min(l);
            let p = |s: f64| (a.0 + dx * s / l, a.1 + dy * s / l);
            linea(c, p(s0), p(s1), ancho, color)
        })
        .collect()
}

/// An arrow from `cola` to `punta`: a shaft and a triangular head `cabeza`
/// px long (shorter if the arrow is).
pub fn flecha(c: &Canvas, cola: P, punta: P, ancho: f64, cabeza: f64, color: Rgb) -> Vec<Prim2> {
    let (dx, dy) = (punta.0 - cola.0, punta.1 - cola.1);
    let l = dx.hypot(dy);
    if l < 1e-9 {
        return Vec::new();
    }
    let h = cabeza.min(l);
    let (ux, uy) = (dx / l, dy / l);
    let base = (punta.0 - ux * h, punta.1 - uy * h);
    let w = h * 0.55;
    vec![
        linea(c, cola, base, ancho, color),
        poligono(
            c,
            &[
                punta,
                (base.0 - uy * w, base.1 + ux * w),
                (base.0 + uy * w, base.1 - ux * w),
            ],
            color,
            1.0,
        ),
    ]
}

/// A regular `n`-gon of radius `r` (a low-poly dot or joint).
pub fn disco(c: &Canvas, centro: P, r: f64, n: usize, color: Rgb) -> Prim2 {
    let n = n.max(3);
    let pts: Vec<P> = (0..n)
        .map(|k| {
            let a = std::f64::consts::TAU * k as f64 / n as f64;
            (centro.0 + r * a.cos(), centro.1 + r * a.sin())
        })
        .collect();
    poligono(c, &pts, color, 1.0)
}

/// One line of text with its baseline at `at` (px), `tam` px, in `cara`.
pub fn texto(c: &Canvas, s: &str, at: P, tam: f64, align: Align, cara: usize, color: Rgb) -> Prim2 {
    Prim2::Text {
        at: c.to_view(at.0, at.1),
        text: s.to_string(),
        size: c.len(tam),
        align,
        face: cara,
        style: Style {
            stroke: color,
            width: 0.0,
            alpha: 1.0,
            fill: None,
        },
    }
}

/// Multiplies the opacity of every primitive (stroke and fill) by `a`.
pub fn con_alfa(mut v: Vec<Prim2>, a: f64) -> Vec<Prim2> {
    for p in &mut v {
        let s = match p {
            Prim2::Point { style, .. }
            | Prim2::Segment { style, .. }
            | Prim2::Polyline { style, .. }
            | Prim2::Edges { style, .. }
            | Prim2::Text { style, .. }
            | Prim2::Path { style, .. } => style,
        };
        s.alpha *= a;
        if let Some(f) = &mut s.fill {
            f.alpha *= a;
        }
    }
    v
}

/// The distance from `p` to the segment `ab`.
pub fn dist_seg(p: P, a: P, b: P) -> f64 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let l2 = dx * dx + dy * dy;
    let s = if l2 < 1e-12 {
        0.0
    } else {
        (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / l2).clamp(0.0, 1.0)
    };
    (p.0 - a.0 - s * dx).hypot(p.1 - a.1 - s * dy)
}

/// Whether segments `ab` and `cd` cross (touching counts).
fn se_cruzan(a: P, b: P, c: P, d: P) -> bool {
    let o = |p: P, q: P, r: P| (q.0 - p.0) * (r.1 - p.1) - (q.1 - p.1) * (r.0 - p.0);
    let (d1, d2, d3, d4) = (o(c, d, a), o(c, d, b), o(a, b, c), o(a, b, d));
    (d1 * d2 <= 0.0) && (d3 * d4 <= 0.0)
}

/// A box `[x0, y0, x1, y1]` px.
pub type Caja = [f64; 4];

/// The distance from the segment `ab` to the box `k` (0 if it enters it).
pub fn dist_caja(a: P, b: P, k: Caja) -> f64 {
    let dentro = |p: P| p.0 >= k[0] && p.0 <= k[2] && p.1 >= k[1] && p.1 <= k[3];
    let esq = [(k[0], k[1]), (k[2], k[1]), (k[2], k[3]), (k[0], k[3])];
    if dentro(a) || dentro(b) {
        return 0.0;
    }
    let lados = [
        (esq[0], esq[1]),
        (esq[1], esq[2]),
        (esq[2], esq[3]),
        (esq[3], esq[0]),
    ];
    if lados.iter().any(|&(p, q)| se_cruzan(a, b, p, q)) {
        return 0.0;
    }
    let mut d = esq
        .iter()
        .map(|&q| dist_seg(q, a, b))
        .fold(f64::INFINITY, f64::min);
    for &(p, q) in &lados {
        d = d.min(dist_seg(a, p, q)).min(dist_seg(b, p, q));
    }
    d
}

/// Something a label keeps away from: a segment and the clearance it needs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Obstaculo {
    pub a: P,
    pub b: P,
    pub holgura: f64,
}

/// Obstacle avoidance: the first candidate centre (in the order given, the
/// designer's order of preference) whose `w` × `h` box stays inside `dentro`
/// and clears every obstacle by its clearance. `None` if none does: the
/// gadget then says so in its avisos instead of overlapping (the rule the
/// fbf reels kept by hand).
pub fn colocar(
    candidatos: &[P],
    w: f64,
    h: f64,
    obstaculos: &[Obstaculo],
    dentro: Caja,
) -> Option<P> {
    candidatos.iter().copied().find(|&(x, y)| {
        let k = [x - w / 2.0, y - h / 2.0, x + w / 2.0, y + h / 2.0];
        k[0] >= dentro[0]
            && k[1] >= dentro[1]
            && k[2] <= dentro[2]
            && k[3] <= dentro[3]
            && obstaculos
                .iter()
                .all(|o| dist_caja(o.a, o.b, k) > o.holgura)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_distancia_a_una_caja() {
        let k = [0.0, 0.0, 10.0, 10.0];
        assert_eq!(dist_caja((5.0, -5.0), (5.0, 15.0), k), 0.0, "la cruza");
        assert_eq!(dist_caja((2.0, 2.0), (3.0, 3.0), k), 0.0, "adentro");
        assert!((dist_caja((13.0, -5.0), (13.0, 15.0), k) - 3.0).abs() < 1e-12);
    }

    #[test]
    fn colocar_salta_los_candidatos_que_estorban() {
        let obst = [Obstaculo {
            a: (0.0, 0.0),
            b: (100.0, 0.0),
            holgura: 10.0,
        }];
        let cands = [(50.0, 0.0), (50.0, -30.0), (50.0, 30.0)];
        let p = colocar(&cands, 40.0, 20.0, &obst, [0.0, -100.0, 200.0, 100.0]);
        assert_eq!(p, Some((50.0, -30.0)));
        // el primero solo: toca la línea, y no hay otro
        assert_eq!(
            colocar(&cands[..1], 40.0, 20.0, &obst, [0.0, -100.0, 200.0, 100.0]),
            None
        );
    }
}
