use guion_audio::{fit_to_window, parse_script, timeline_duration, NarrationEngine, ScaffoldEngine};
use guion_core::load_and_check;

#[test]
fn parse_narration_script_blocks() {
    let src = r#"
[0.4–3.8]
Hola mundo.

[3.9–7.2]
Segunda línea.
"#;
    let cues = parse_script(src).unwrap();
    assert_eq!(cues.len(), 2);
    assert!((cues[0].at.start - 0.4).abs() < 1e-9);
    assert!(cues[0].text.contains("Hola"));
}

#[test]
fn timeline_duration_includes_narration_end() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let path = root.join("templates/reel-39-shoulder-press.screenplay.toml");
    let sp = load_and_check(&path).unwrap();
    let d = timeline_duration(&sp);
    assert!(d >= 15.5);
}

#[test]
fn scaffold_engine_produces_samples() {
    let engine = ScaffoldEngine;
    let samples = engine.synthesize("tres palabras de prueba").unwrap();
    assert!(!samples.is_empty());
}

#[test]
fn fit_to_window_pads_short_audio() {
    let short = vec![0.1, 0.2, 0.3];
    let out = fit_to_window(&short, 0.5);
    assert!(out.len() > short.len());
}
