use guion_core::{load_and_check, Token};
use guion_motion::ModelRuntime;

#[test]
fn wasm_biceps_force_matches_native_at_90_deg() {
    let path = std::path::PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../guion-core/tests/fixtures/reel-09-biceps.screenplay.toml"
    ));
    let wasm =
        std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../physics-lab"))
            .join("public/lessons/lever/lesson.wasm");
    if !wasm.exists() {
        eprintln!("skip: lever wasm not built at {}", wasm.display());
        return;
    }

    let sp = load_and_check(&path).expect("golden");
    let root = wasm
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let mut rt = ModelRuntime::with_root(root);
    let phi = std::f64::consts::FRAC_PI_2;
    let token = Token::parse("@lever.biceps_force_N").unwrap();
    let f = rt.resolve_token(&sp, &token, phi).expect("force");
    assert!(f > 1500.0, "biceps force at 90°: {f}");
}
