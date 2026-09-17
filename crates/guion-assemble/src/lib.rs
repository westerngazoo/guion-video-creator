//! `guion-assemble` — validated screenplay → `motoreel::Scene` (R-0002).
//!
//! This is the sole crate that imports `motoreel` (RFC-0001 §3.4).

mod camera;
mod error;
mod label;
mod motion;
mod overlay;
mod shape;
mod style;

pub use error::AssembleError;
pub use motion::{duration, phi_at};

use guion_brand::Theme;
use guion_core::Screenplay;
use guion_motion::ModelRuntime;
use motoreel::{Object, Scene, Shape as MrShape};

/// Assemble a static scene snapshot at time `t` (geometry posed at `t`).
pub fn assemble_at(
    sp: &Screenplay,
    theme: &Theme,
    t: f64,
    runtime: &mut ModelRuntime,
) -> Result<Scene, AssembleError> {
    let dur = duration(sp);
    let phi = motion::phi_at(sp, t);
    let mut scene = Scene::new(dur);
    scene.camera = camera::build(&sp.camera)?;
    scene.view = camera::view_for(&sp.meta);

    let mut ids = Vec::new();
    for obj in &sp.object {
        let heat = heat_for_object(sp, runtime, &obj.style, phi);
        let mr_shape = shape::build(
            sp,
            runtime,
            &obj.shape,
            phi,
            &format!("object[{}].shape", obj.id),
        )?;
        let mr_style = obj
            .style
            .as_ref()
            .map(|s| style::build(s, theme, &sp.meta, heat))
            .unwrap_or_default();
        let oid = scene.add(object_from_shape(mr_shape).with_style(mr_style));
        ids.push((obj.id.clone(), oid));
    }

    for lbl in &sp.label {
        scene.add_label(label::build(sp, runtime, lbl, &ids, theme, phi)?);
    }
    for lbl in overlay::labels(sp, theme, t) {
        scene.add_label(lbl);
    }

    Ok(scene)
}

/// Assemble at t = 0 (convenience for tests).
pub fn assemble(sp: &Screenplay, theme: &Theme) -> Result<Scene, AssembleError> {
    let mut rt = ModelRuntime::new();
    assemble_at(sp, theme, 0.0, &mut rt)
}

fn object_from_shape(shape: MrShape) -> Object {
    match shape {
        MrShape::Point(p) => Object::point(p),
        MrShape::Segment(a, b) => Object::segment(a, b),
        MrShape::Polyline(ps) => Object::polyline(ps),
        MrShape::Edges(es) => Object::edges(es),
    }
}

fn heat_for_object(
    sp: &Screenplay,
    runtime: &mut ModelRuntime,
    style: &Option<guion_core::Style>,
    phi: f64,
) -> Option<f64> {
    let stroke = style.as_ref().map(|s| s.stroke.as_str())?;
    if !stroke.starts_with("heat:") {
        return None;
    }
    for token in guion_core::Token::extract(stroke) {
        if let Some(v) = runtime.resolve_token(sp, &token, phi) {
            return Some((v / 1600.0).clamp(0.0, 1.0));
        }
    }
    None
}
