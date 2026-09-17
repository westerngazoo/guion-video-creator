//! Semantic validation — the checks deserialization cannot do (SPEC-0001
//! §"Validation").
//!
//! Structure, required fields, unknown fields, unknown enum kinds and token
//! *grammar* are already enforced by the format boundary (they surface as
//! [`LoadError`]). What remains is meaning:
//!
//! * numeric ranges: `meta.fps > 0`, `hold` fractions in `[0, 1]` (AC4/AC7),
//! * `meta.lang` is a non-empty tag (AC4),
//! * every binding token / id reference resolves to a declared id (AC6).
//!
//! `meta.format` and `motion.ease` are already constrained to known presets by
//! their enums at load, so there is nothing left to check for them here.
//!
//! Validation is pure and deterministic: it reports the first problem in a
//! fixed traversal order, so the same input always yields the same error
//! (R-0001 AC8).
//!
//! [`LoadError`]: crate::error::LoadError

use crate::bind::{SymbolTable, Token};
use crate::error::ValidateError;
use crate::model::{Screenplay, Shape};

/// Run every semantic check, returning the first failure.
pub fn validate(sp: &Screenplay) -> Result<(), ValidateError> {
    check_meta(sp)?;
    check_motion(sp)?;
    check_refs(sp)?;
    Ok(())
}

fn check_meta(sp: &Screenplay) -> Result<(), ValidateError> {
    if !sp.meta.fps.is_finite() || sp.meta.fps <= 0.0 {
        return Err(ValidateError::OutOfRange {
            field: "meta.fps".to_string(),
            value: sp.meta.fps,
            bound: "finito y > 0",
        });
    }
    if sp.meta.lang.trim().is_empty() {
        return Err(ValidateError::Empty {
            field: "meta.lang".to_string(),
        });
    }
    Ok(())
}

fn check_motion(sp: &Screenplay) -> Result<(), ValidateError> {
    if let Some(hold) = &sp.motion.hold {
        for (name, v) in [("start", hold.start), ("end", hold.end)] {
            if !(0.0..=1.0).contains(&v) {
                return Err(ValidateError::OutOfRange {
                    field: format!("motion.hold.{name}"),
                    value: v,
                    bound: "in [0, 1]",
                });
            }
        }
    }
    Ok(())
}

/// Resolve every binding token and bare id reference against the declared ids.
fn check_refs(sp: &Screenplay) -> Result<(), ValidateError> {
    let table = SymbolTable::build(sp);
    for site in collect_refs(sp) {
        if !table.contains(&site.id) {
            return Err(ValidateError::DanglingRef {
                token: site.display,
                at: site.at,
            });
        }
    }
    Ok(())
}

/// One reference to resolve: the `id` that must be declared, the text to show
/// in an error, and where in the screenplay it was used.
struct Ref {
    id: String,
    display: String,
    at: String,
}

/// Gather every reference in a fixed order: motion, then objects (shapes then
/// stroke paints), then label anchors.
fn collect_refs(sp: &Screenplay) -> Vec<Ref> {
    let mut refs = Vec::new();

    push_token(&mut refs, &sp.motion.drive, "motion.drive");

    for (i, obj) in sp.object.iter().enumerate() {
        match &obj.shape {
            Shape::Point { at } => {
                push_token(&mut refs, at, &site(i, "point.at"));
            }
            Shape::Segment { from, to } => {
                push_token(&mut refs, from, &site(i, "segment.from"));
                push_token(&mut refs, to, &site(i, "segment.to"));
            }
            Shape::Polyline { points } => {
                for (j, p) in points.iter().enumerate() {
                    let at = site(i, &format!("polyline.points[{j}]"));
                    push_token(&mut refs, p, &at);
                }
            }
            Shape::Edges { verts, .. } => {
                for (j, v) in verts.iter().enumerate() {
                    let at = site(i, &format!("edges.verts[{j}]"));
                    push_token(&mut refs, v, &at);
                }
            }
            Shape::Arrow { at, value, .. } => {
                push_token(&mut refs, at, &site(i, "arrow.at"));
                push_token(&mut refs, value, &site(i, "arrow.value"));
            }
        }
        if let Some(style) = &obj.style {
            let at = site(i, "style.stroke");
            for t in Token::extract(&style.stroke) {
                push_token(&mut refs, &t, &at);
            }
        }
    }

    for (i, lbl) in sp.label.iter().enumerate() {
        refs.push(Ref {
            id: lbl.anchor.pose.object.clone(),
            display: lbl.anchor.pose.object.clone(),
            at: format!("label[{i}].anchor.object"),
        });
    }

    refs
}

fn push_token(refs: &mut Vec<Ref>, t: &Token, at: &str) {
    refs.push(Ref {
        id: t.id().to_string(),
        display: t.as_str().to_string(),
        at: at.to_string(),
    });
}

fn site(i: usize, tail: &str) -> String {
    format!("object[{i}].shape.{tail}")
}
