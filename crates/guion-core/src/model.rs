//! The typed screenplay schema (SPEC-0001 §"Typed model").
//!
//! One declarative file fully describes a physics animation — *what* it is,
//! never *how* it is drawn (R-0001 §1). Every struct is
//! `#[serde(deny_unknown_fields)]` so a typo fails loudly instead of being
//! silently dropped (R-0001 AC3), and the enum-shaped fields carry an internal
//! `kind`/tag so the TOML stays flat and readable (RFC-0001 §11 Q2).
//!
//! Where RFC-0001 §4's illustrative TOML and SPEC-0001's `model.rs` sketch
//! disagree, the requirement's acceptance criteria are the contract:
//!
//! * `[motion]` is the flat `drive/from/to/ease/hold` timeline of AC7 and
//!   RFC-0001 §4 (the elaborate keyframe/spin/`in_place`/… `MotionMode` enum in
//!   the spec sketch is "representative, not final" and its *resolution* is M4).
//! * Model outputs are referenced by *qualified* token, `@model_id.output`
//!   (e.g. `@lever.elbow`), so every token resolves against a declared id
//!   (AC6); RFC-0001 §4's bare `@elbow` is the loose illustrative form.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::bind::Token;

/// The whole screenplay. Top-level sections mirror RFC-0001 §4.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Screenplay {
    pub meta: Meta,
    pub camera: Camera,
    #[serde(default)]
    pub model: Vec<ModelRef>,
    #[serde(default)]
    pub object: Vec<ObjectSpec>,
    #[serde(default)]
    pub label: Vec<LabelSpec>,
    pub motion: Motion,
    #[serde(default)]
    pub hook: Option<Hook>,
    #[serde(default)]
    pub footer: Option<Footer>,
    #[serde(default)]
    pub audio: Option<Audio>,
    /// Timed voice-over lines (voz en off).
    #[serde(default)]
    pub narration: Vec<Narration>,
}

/// Title, slug, output format, timing, theme and language.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Meta {
    pub title: String,
    pub slug: String,
    pub format: Format,
    pub fps: f64,
    #[serde(default)]
    pub theme: Option<String>,
    pub lang: String,
}

/// Output canvas preset. M1 ships only `vertical` (R-0001 Q1); `square`/`wide`
/// arrive with the M2 delivery layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Format {
    Vertical,
}

/// A reference to a named, tested physics model — never inline math (R-0001
/// §1). `source` resolves downstream (M4) to the model's compiled artifact.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ModelRef {
    pub id: String,
    pub source: String,
    #[serde(default)]
    pub params: BTreeMap<String, f64>,
}

/// The camera. M1 needs only the kind and a pose.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Camera {
    pub kind: CameraKind,
    #[serde(default)]
    pub pose: Pose,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CameraKind {
    Orthographic,
    Perspective,
}

/// A rigid placement. Only translation is needed for M1's orthographic camera.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields, default)]
pub struct Pose {
    pub translate: [f64; 3],
}

impl Default for Pose {
    fn default() -> Self {
        Pose {
            translate: [0.0, 0.0, 0.0],
        }
    }
}

/// A drawable object: an id, a geometric shape bound to model outputs, and an
/// optional style.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ObjectSpec {
    pub id: String,
    pub shape: Shape,
    #[serde(default)]
    pub style: Option<Style>,
}

/// The shape vocabulary. Internally tagged by `kind`, so an unknown kind is a
/// loud deserialization error listing the valid kinds (R-0001 AC5). Every
/// endpoint is a binding [`Token`] into a model/object output.
///
/// Note: `#[serde(deny_unknown_fields)]` is intentionally *not* placed here —
/// serde does not support it together with internal tagging. Unknown fields on
/// the plain structs (which is where a real typo lands) are still rejected.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Shape {
    Point {
        at: Token,
    },
    Segment {
        from: Token,
        to: Token,
    },
    Polyline {
        points: Vec<Token>,
    },
    Edges {
        verts: Vec<Token>,
        edges: Vec<(usize, usize)>,
    },
    Arrow {
        at: Token,
        value: Token,
        scale: Scale,
    },
}

/// How an arrow maps a magnitude to a drawn length. M1 supports `auto`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Scale {
    Auto,
}

/// Object paint. `stroke` is a theme color name (`"skin"`) or a value-bound
/// paint (`"heat:@lever.force_N"`); embedded `@` tokens are resolved in
/// validation. Resolving the *colors* is the brand layer's job (M3).
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Style {
    pub stroke: String,
    pub width: f64,
}

/// A text label anchored to an object.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LabelSpec {
    pub anchor: Anchor,
    pub text: String,
    #[serde(default)]
    pub style: Option<String>,
}

/// Where a label attaches. M1 anchors to a point on an object.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Anchor {
    pub pose: PoseAnchor,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PoseAnchor {
    /// The id of a declared object (validated in resolution).
    pub object: String,
    pub at: AnchorAt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AnchorAt {
    Start,
    Mid,
    End,
}

/// The timeline. Drives one model parameter from `from` to `to` with an easing
/// and optional dwell in/out — the declarative form of the hand-built `seq[]`
/// (RFC-0001 §4). Richer motion modes resolve in M4; M1 loads and validates
/// this authored form (R-0001 §1, AC7).
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Motion {
    pub drive: Token,
    pub from: f64,
    pub to: f64,
    pub ease: EaseName,
    #[serde(default)]
    pub hold: Option<Hold>,
}

/// Dwell fractions at the ends of the pull, each in `[0, 1]` (AC7).
#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Hold {
    pub start: f64,
    pub end: f64,
}

/// The known easings. Names match the smoothstep family used by the current
/// reels; the render engine owns the actual curves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EaseName {
    Linear,
    Smoothstep,
    Smootherstep,
    EaseIn,
    EaseOut,
    EaseInOut,
}

/// The opening hook overlay and the window it shows in.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Hook {
    pub text: String,
    pub at: Span,
}

/// A `[start, end]` window in normalized timeline units.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Span {
    pub start: f64,
    pub end: f64,
}

/// The pedagogical footer baked at the bottom of the reel.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Footer {
    pub concept: String,
    pub detail: String,
    pub credit: String,
}

/// The audio bed: a named generator or a path to a wav (RFC-0001 §4).
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Audio {
    #[serde(default)]
    pub generator: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
    /// Optional path to a narration script (plain text, one cue per block).
    #[serde(default)]
    pub narration_script: Option<String>,
}

/// One voice-over cue synchronized to the timeline.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Narration {
    pub text: String,
    pub at: Span,
}
