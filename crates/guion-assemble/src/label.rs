use garust::pga;
use guion_core::{AnchorAt, LabelSpec, Screenplay};
use guion_motion::ModelRuntime;
use motoreel::{Align, Anchor, Label, ObjectId};

use crate::error::AssembleError;
use crate::shape;
use guion_brand::Theme;

pub fn build(
    sp: &Screenplay,
    runtime: &mut ModelRuntime,
    lbl: &LabelSpec,
    ids: &[(String, ObjectId)],
    theme: &Theme,
    phi: f64,
) -> Result<Label, AssembleError> {
    let object = lbl.anchor.pose.object.clone();
    let oid = ids
        .iter()
        .find(|(id, _)| id == &object)
        .map(|(_, oid)| *oid)
        .ok_or_else(|| AssembleError::UnknownObject {
            id: object.clone(),
            at: "label.anchor.object".to_string(),
        })?;

    let anchor_pt = anchor_point(sp, runtime, lbl, phi)?;

    let heat = biceps_heat(sp, runtime, phi);
    let text = interpolate(&lbl.text, sp, runtime, phi);
    let mut label = Label::new(text, Anchor::Pose { object: oid, at: anchor_pt });
    label = label.with_align(Align::Center);
    label.style.stroke = theme.stroke_color("ink", None);
    if lbl.style.as_deref() == Some("callout") {
        label = label.with_size(0.06);
        label.style.stroke = theme.stroke_color("heat:", Some(heat));
    }
    Ok(label)
}

fn anchor_point(
    sp: &Screenplay,
    runtime: &mut ModelRuntime,
    lbl: &LabelSpec,
    phi: f64,
) -> Result<pga::Point, AssembleError> {
    let object = &lbl.anchor.pose.object;
    let obj = sp
        .object
        .iter()
        .find(|o| o.id == *object)
        .ok_or_else(|| AssembleError::UnknownObject {
            id: object.clone(),
            at: "label.anchor.object".to_string(),
        })?;
    let mr_shape = shape::build(sp, runtime, &obj.shape, phi, "label.anchor")?;
    let at = lbl.anchor.pose.at;
    let p = shape_midpoint(&mr_shape, at)?;
    Ok(p)
}

fn shape_midpoint(shape: &motoreel::Shape, at: AnchorAt) -> Result<pga::Point, AssembleError> {
    use motoreel::Shape as S;
    match shape {
        S::Segment(a, b) => {
            let (ax, ay, az) = a.to_euclidean();
            let (bx, by, bz) = b.to_euclidean();
            let pt = match at {
                AnchorAt::Start => *a,
                AnchorAt::End => *b,
                AnchorAt::Mid => pga::Point::new((ax + bx) / 2.0, (ay + by) / 2.0, (az + bz) / 2.0),
            };
            Ok(pt)
        }
        S::Point(p) => Ok(*p),
        _ => Err(AssembleError::Unsupported {
            what: "ancla mid sólo en segmento/punto en M1".to_string(),
        }),
    }
}

fn interpolate(text: &str, sp: &Screenplay, runtime: &mut ModelRuntime, phi: f64) -> String {
    let mut out = text.to_string();
    if let Some(start) = out.find('{') {
        if let Some(end) = out[start..].find('}') {
            let key = &out[start + 1..start + end];
            let (field, _fmt) = key.split_once(':').unwrap_or((key, ""));
            for prefix in ["@lever.", "@press."] {
                let token = format!("{prefix}{field}");
                if let Ok(tok) = guion_core::Token::parse(&token) {
                    if let Some(v) = runtime.resolve_token(sp, &tok, phi) {
                        let rendered = if _fmt.ends_with('f') {
                            format!("{:.0}", v)
                        } else {
                            format!("{v}")
                        };
                        out.replace_range(start..start + end + 1, &rendered);
                        break;
                    }
                }
            }
        }
    }
    out
}

fn biceps_heat(sp: &Screenplay, runtime: &mut ModelRuntime, phi: f64) -> f64 {
    let token = guion_core::Token::parse("@lever.biceps_force_N").unwrap();
    let f = runtime.resolve_token(sp, &token, phi).unwrap_or(0.0);
    (f / 1600.0).clamp(0.0, 1.0)
}
