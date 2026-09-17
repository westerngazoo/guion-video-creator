//! Acceptance tests for R-0001 / SPEC-0001, one test per criterion (AC1–AC9).

use std::path::PathBuf;

use guion_core::{
    check, from_str, load, load_and_check, to_string, EaseName, Format, GuionError, LoadError,
    Scale, Screenplay, Shape, Syntax, ValidateError,
};

fn fixture_path() -> PathBuf {
    PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/reel-09-biceps.screenplay.toml"
    ))
}

/// A minimal screenplay that loads *and* validates clean, for mutating in the
/// semantic-error tests.
const BASE: &str = r#"
[meta]
title = "t"
slug = "s"
format = "vertical"
fps = 30
lang = "es"

[[model]]
id = "m"
source = "physics-lab:m"

[camera]
kind = "orthographic"

[motion]
drive = "@m.x"
from = 0.0
to = 1.0
ease = "linear"
"#;

fn parse(src: &str) -> Result<Screenplay, LoadError> {
    from_str(src, Syntax::Toml)
}

// ── AC9: the reel-09 lever golden fixture loads + validates clean ───────────

#[test]
fn ac9_golden_fixture_loads_and_validates() {
    let sp = load_and_check(&fixture_path()).expect("golden fixture must load and validate clean");

    assert_eq!(sp.meta.slug, "reel-09-biceps");
    assert_eq!(sp.meta.format, Format::Vertical);
    assert_eq!(sp.meta.fps, 30.0);
    assert_eq!(sp.meta.theme.as_deref(), Some("fbf"));

    assert_eq!(sp.model.len(), 1);
    assert_eq!(sp.model[0].id, "lever");
    assert_eq!(sp.model[0].params["load_mass"], 20.0);

    assert_eq!(sp.object.len(), 2);
    match &sp.object[0].shape {
        Shape::Segment { from, to } => {
            assert_eq!(from.id(), "lever");
            assert_eq!(from.field(), Some("elbow"));
            assert_eq!(to.as_str(), "@lever.hand");
        }
        other => panic!("expected segment, got {other:?}"),
    }
    match &sp.object[1].shape {
        Shape::Arrow { scale, value, .. } => {
            assert_eq!(*scale, Scale::Auto);
            assert_eq!(value.as_str(), "@lever.biceps_force_N");
        }
        other => panic!("expected arrow, got {other:?}"),
    }

    assert_eq!(sp.motion.ease, EaseName::Smootherstep);
    assert_eq!(sp.motion.drive.as_str(), "@lever.phi");
    let hold = sp.motion.hold.expect("hold present");
    assert_eq!((hold.start, hold.end), (1.0, 0.87));

    assert_eq!(sp.camera.pose.translate, [0.0, 0.0, 6.0]);
    assert!(sp.hook.is_some() && sp.footer.is_some() && sp.audio.is_some());
}

// ── AC1: round-trip load → serialize → load is structurally identical ───────

#[test]
fn ac1_roundtrip_is_structurally_identical() {
    let sp = load(&fixture_path()).expect("load");
    let emitted = to_string(&sp, Syntax::Toml).expect("serialize");
    let reparsed = from_str(&emitted, Syntax::Toml).expect("reload");
    assert_eq!(sp, reparsed, "round-trip changed the model");
}

// ── AC2: a missing required field is a typed error naming the field ─────────

#[test]
fn ac2_missing_required_field_names_it() {
    let src = BASE.replace("title = \"t\"\n", "");
    match parse(&src) {
        Err(LoadError::Parse { msg, .. }) => {
            assert!(msg.contains("title"), "error should name `title`: {msg}");
        }
        other => panic!("expected Parse error, got {other:?}"),
    }
}

// ── AC3: an unknown/misspelled field is rejected, naming the key ────────────

#[test]
fn ac3_unknown_field_is_rejected() {
    let src = BASE.replace("slug = \"s\"", "slug = \"s\"\ntilte = \"oops\"");
    match parse(&src) {
        Err(LoadError::Parse { msg, .. }) => {
            assert!(msg.contains("tilte"), "error should name `tilte`: {msg}");
        }
        other => panic!("expected Parse error, got {other:?}"),
    }
}

// ── AC4: fps>0, known format, non-empty lang ────────────────────────────────

#[test]
fn ac4_fps_must_be_positive() {
    let src = BASE.replace("fps = 30", "fps = 0");
    let sp = parse(&src).expect("loads");
    match check(&sp) {
        Err(ValidateError::OutOfRange { field, .. }) => {
            assert_eq!(field, "meta.fps");
        }
        other => panic!("expected OutOfRange, got {other:?}"),
    }
}

#[test]
fn ac4_lang_must_be_non_empty() {
    let src = BASE.replace("lang = \"es\"", "lang = \"\"");
    let sp = parse(&src).expect("loads");
    match check(&sp) {
        Err(ValidateError::Empty { field }) => assert_eq!(field, "meta.lang"),
        other => panic!("expected Empty, got {other:?}"),
    }
}

#[test]
fn ac4_unknown_format_is_rejected_at_load() {
    let src = BASE.replace("format = \"vertical\"", "format = \"potrait\"");
    match parse(&src) {
        Err(LoadError::Parse { msg, .. }) => {
            assert!(msg.contains("vertical"), "should list valid: {msg}");
        }
        other => panic!("expected Parse error, got {other:?}"),
    }
}

// ── AC5: unknown shape kind → typed error listing valid kinds ───────────────

#[test]
fn ac5_unknown_shape_kind_lists_valid_kinds() {
    let src = format!("{BASE}\n[[object]]\nid = \"o\"\nshape = {{ kind = \"blob\" }}\n");
    match parse(&src) {
        Err(LoadError::Parse { msg, .. }) => {
            assert!(msg.contains("blob"), "names the bad kind: {msg}");
            for k in ["point", "segment", "polyline", "edges", "arrow"] {
                assert!(msg.contains(k), "should list `{k}`: {msg}");
            }
        }
        other => panic!("expected Parse error, got {other:?}"),
    }
}

// ── AC6: a dangling @ reference names the token and the site ────────────────

#[test]
fn ac6_dangling_motion_drive() {
    let src = BASE.replace("drive = \"@m.x\"", "drive = \"@ghost.x\"");
    let sp = parse(&src).expect("loads");
    match check(&sp) {
        Err(ValidateError::DanglingRef { token, at }) => {
            assert_eq!(token, "@ghost.x");
            assert_eq!(at, "motion.drive");
        }
        other => panic!("expected DanglingRef, got {other:?}"),
    }
}

#[test]
fn ac6_dangling_shape_and_label_refs() {
    // Object shape references an undeclared id.
    let src = format!(
        "{BASE}\n[[object]]\nid = \"o\"\n\
         shape = {{ kind = \"point\", at = \"@nope\" }}\n"
    );
    let sp = parse(&src).expect("loads");
    match check(&sp) {
        Err(ValidateError::DanglingRef { token, at }) => {
            assert_eq!(token, "@nope");
            assert_eq!(at, "object[0].shape.point.at");
        }
        other => panic!("expected DanglingRef, got {other:?}"),
    }

    // Label anchor references an undeclared object id.
    let src = format!(
        "{BASE}\n[[label]]\n\
         anchor = {{ pose = {{ object = \"ghost\", at = \"mid\" }} }}\n\
         text = \"x\"\n"
    );
    let sp = parse(&src).expect("loads");
    match check(&sp) {
        Err(ValidateError::DanglingRef { token, at }) => {
            assert_eq!(token, "ghost");
            assert_eq!(at, "label[0].anchor.object");
        }
        other => panic!("expected DanglingRef, got {other:?}"),
    }
}

#[test]
fn ac6_embedded_stroke_token_resolves() {
    // A `heat:@id` paint whose token dangles must be caught too.
    let src = format!(
        "{BASE}\n[[object]]\nid = \"o\"\n\
         shape = {{ kind = \"point\", at = \"@m.tip\" }}\n\
         style = {{ stroke = \"heat:@missing.force\", width = 4 }}\n"
    );
    let sp = parse(&src).expect("loads");
    match check(&sp) {
        Err(ValidateError::DanglingRef { token, at }) => {
            assert_eq!(token, "@missing.force");
            assert_eq!(at, "object[0].shape.style.stroke");
        }
        other => panic!("expected DanglingRef, got {other:?}"),
    }
}

// ── AC7: motion consistency (from/to present, ease known, hold ∈ [0,1]) ─────

#[test]
fn ac7_missing_from_is_a_load_error() {
    let src = BASE.replace("from = 0.0\n", "");
    match parse(&src) {
        Err(LoadError::Parse { msg, .. }) => {
            assert!(msg.contains("from"), "names `from`: {msg}");
        }
        other => panic!("expected Parse error, got {other:?}"),
    }
}

#[test]
fn ac7_unknown_ease_is_a_load_error() {
    let src = BASE.replace("ease = \"linear\"", "ease = \"bouncy\"");
    match parse(&src) {
        Err(LoadError::Parse { msg, .. }) => {
            assert!(msg.contains("smootherstep"), "lists eases: {msg}");
        }
        other => panic!("expected Parse error, got {other:?}"),
    }
}

#[test]
fn ac7_hold_fraction_out_of_range() {
    let src = format!("{BASE}hold = {{ start = 1.5, end = 0.0 }}\n");
    let sp = parse(&src).expect("loads");
    match check(&sp) {
        Err(ValidateError::OutOfRange { field, value, .. }) => {
            assert_eq!(field, "motion.hold.start");
            assert_eq!(value, 1.5);
        }
        other => panic!("expected OutOfRange, got {other:?}"),
    }
}

// ── AC8: loading is deterministic and side-effect free ──────────────────────

#[test]
fn ac8_same_bytes_same_result() {
    let a = load(&fixture_path()).expect("load a");
    let b = load(&fixture_path()).expect("load b");
    assert_eq!(a, b);

    // The same malformed bytes always produce the same error text.
    let bad = "not = valid = screenplay";
    let e1 = format!("{:?}", parse(bad).unwrap_err());
    let e2 = format!("{:?}", parse(bad).unwrap_err());
    assert_eq!(e1, e2);
}

#[test]
fn unknown_extension_is_typed_error() {
    let p = PathBuf::from("/tmp/guion-does-not-exist.ron");
    match load(&p) {
        Err(LoadError::UnknownFormat { ext, .. }) => assert_eq!(ext, "ron"),
        other => panic!("expected UnknownFormat, got {other:?}"),
    }
}

#[test]
fn load_and_check_reports_validate_errors() {
    let src = BASE.replace("fps = 30", "fps = -1");
    // Write to a temp file to drive the full disk path once.
    let dir = std::env::temp_dir();
    let path = dir.join("guion-core-badfps.screenplay.toml");
    std::fs::write(&path, src).expect("write temp");
    match load_and_check(&path) {
        Err(GuionError::Validate(ValidateError::OutOfRange { field, .. })) => {
            assert_eq!(field, "meta.fps")
        }
        other => panic!("expected Validate/OutOfRange, got {other:?}"),
    }
    let _ = std::fs::remove_file(&path);
}
