use garust::pga;
use guion_core::{Scale, Screenplay, Shape, Token};
use guion_motion::ModelRuntime;
use motoreel::Shape as MrShape;

use crate::error::AssembleError;

pub fn build(
    sp: &Screenplay,
    runtime: &mut ModelRuntime,
    shape: &Shape,
    phi: f64,
    at: &str,
) -> Result<MrShape, AssembleError> {
    match shape {
        Shape::Point { at: token } => Ok(MrShape::Point(point(sp, runtime, token, phi, at)?)),
        Shape::Segment { from, to } => Ok(MrShape::Segment(
            point(sp, runtime, from, phi, &format!("{at}.from"))?,
            point(sp, runtime, to, phi, &format!("{at}.to"))?,
        )),
        Shape::Polyline { points } => {
            let pts = points
                .iter()
                .enumerate()
                .map(|(i, t)| point(sp, runtime, t, phi, &format!("{at}.points[{i}]")))
                .collect::<Result<Vec<pga::Point>, AssembleError>>()?;
            Ok(MrShape::Polyline(pts))
        }
        Shape::Edges { verts, edges } => {
            let verts = verts
                .iter()
                .enumerate()
                .map(|(i, t)| point(sp, runtime, t, phi, &format!("{at}.verts[{i}]")))
                .collect::<Result<Vec<pga::Point>, AssembleError>>()?;
            let segments = edges
                .iter()
                .map(|(a, b)| {
                    Ok((
                        verts
                            .get(*a)
                            .copied()
                            .ok_or_else(|| AssembleError::Unsupported {
                                what: format!("índice de vértice {a} fuera de rango en {at}"),
                            })?,
                        verts
                            .get(*b)
                            .copied()
                            .ok_or_else(|| AssembleError::Unsupported {
                                what: format!("índice de vértice {b} fuera de rango en {at}"),
                            })?,
                    ))
                })
                .collect::<Result<Vec<(pga::Point, pga::Point)>, AssembleError>>()?;
            Ok(MrShape::Edges(segments))
        }
        Shape::Arrow {
            at: anchor,
            value,
            scale,
        } => arrow(sp, runtime, anchor, value, *scale, phi, at),
    }
}

fn arrow(
    sp: &Screenplay,
    runtime: &mut ModelRuntime,
    at: &Token,
    value: &Token,
    scale: Scale,
    phi: f64,
    site: &str,
) -> Result<MrShape, AssembleError> {
    let origin = point(sp, runtime, at, phi, &format!("{site}.at"))?;
    let magnitude = scalar(sp, runtime, value, phi, &format!("{site}.value"))?;
    let dir = arrow_direction(sp, runtime, at, phi, site)?;
    let len = match scale {
        Scale::Auto => auto_arrow_length(magnitude),
    };
    let tip = offset_point(origin, dir, len);
    Ok(MrShape::Segment(origin, tip))
}

fn arrow_direction(
    sp: &Screenplay,
    runtime: &mut ModelRuntime,
    at: &Token,
    phi: f64,
    site: &str,
) -> Result<[f64; 3], AssembleError> {
    let ins = runtime
        .resolve_point(sp, at, phi)
        .ok_or_else(|| AssembleError::Unresolved {
            token: at.as_str().to_string(),
            at: format!("{site}.at"),
        })?;
    let shoulder = runtime
        .resolve_point(sp, &Token::parse("@lever.shoulder").unwrap(), phi)
        .ok_or_else(|| AssembleError::Unresolved {
            token: "@lever.shoulder".to_string(),
            at: format!("{site}.direction"),
        })?;
    let dx = shoulder[0] - ins[0];
    let dy = shoulder[1] - ins[1];
    let dz = shoulder[2] - ins[2];
    let n = (dx * dx + dy * dy + dz * dz).sqrt();
    if n <= 0.0 || !n.is_finite() {
        return Ok([0.0, 1.0, 0.0]);
    }
    Ok([dx / n, dy / n, dz / n])
}

fn auto_arrow_length(force_n: f64) -> f64 {
    // reel09: PX_POR_N = 300/1570; map to model units (~0.19 m/N at forearm scale).
    (force_n * 0.00019).clamp(0.02, 0.25)
}

fn offset_point(p: pga::Point, dir: [f64; 3], len: f64) -> pga::Point {
    let (x, y, z) = p.to_euclidean();
    pga::Point::new(x + dir[0] * len, y + dir[1] * len, z + dir[2] * len)
}

fn point(
    sp: &Screenplay,
    runtime: &mut ModelRuntime,
    token: &Token,
    phi: f64,
    at: &str,
) -> Result<pga::Point, AssembleError> {
    let p = runtime
        .resolve_point(sp, token, phi)
        .ok_or_else(|| AssembleError::Unresolved {
            token: token.as_str().to_string(),
            at: at.to_string(),
        })?;
    Ok(pga::Point::new(p[0], p[1], p[2]))
}

fn scalar(
    sp: &Screenplay,
    runtime: &mut ModelRuntime,
    token: &Token,
    phi: f64,
    at: &str,
) -> Result<f64, AssembleError> {
    runtime
        .resolve_token(sp, token, phi)
        .ok_or_else(|| AssembleError::Unresolved {
            token: token.as_str().to_string(),
            at: at.to_string(),
        })
}
