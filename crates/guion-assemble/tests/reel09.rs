use std::path::PathBuf;

use guion_assemble::{assemble_at, duration};
use guion_brand::default_theme;
use guion_core::load_and_check;
use guion_motion::ModelRuntime;

fn fixture() -> PathBuf {
    PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../guion-core/tests/fixtures/reel-09-biceps.screenplay.toml"
    ))
}

#[test]
fn ac1_duration_matches_hook_end() {
    let sp = load_and_check(&fixture()).expect("golden");
    assert!((duration(&sp) - 2.05).abs() < 1e-9);
}

#[test]
fn ac6_frame_count_at_30fps() {
    let sp = load_and_check(&fixture()).expect("golden");
    let frames = (duration(&sp) * sp.meta.fps).ceil() as usize;
    assert_eq!(frames, 62);
}

#[test]
fn ac8_assembles_mid_frame_with_primitives() {
    let sp = load_and_check(&fixture()).expect("golden");
    let theme = default_theme();
    let mut runtime = ModelRuntime::new();
    let scene = assemble_at(&sp, &theme, 1.0, &mut runtime).expect("assemble");
    let prims = scene.eval(0.0);
    assert!(
        prims.len() >= 2,
        "expected forearm segment + biceps arrow, got {}",
        prims.len()
    );
}
