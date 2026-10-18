//! R-0014 AC6–AC7 — the template reel of a new account, end to end: scaffold
//! a throwaway account, make a post from its template (`guion reel nuevo`),
//! build its scene against guion-marca, render it twice, make its bed with the
//! neutral kit, mix it to −14 LUFS and run the whole gate (`guion reel check`)
//! with the profile derived from the contract.
//!
//! What is stood in, and why: the voice (Piper and a voice model are the
//! account's; `voz.json` and `voz.wav` are synthesized here, a tone per line)
//! and the faces (the account's are not shipped; DejaVu stands in). The test
//! is skipped, saying so, without DejaVu or without ffmpeg (libx264). The
//! scene is built with cargo in release, in `target/cuenta-e2e` (cached
//! between runs), against this workspace's Cargo.lock so nothing is resolved
//! from the network.

use std::path::{Path, PathBuf};
use std::process::Command;

use guion_cuenta::nueva::{crear, Respuestas};
use guion_encode::{mux_at_loudness, LoudnessTarget};
use guion_marca::reel::gate::{self, Inputs, Profile};
use guion_marca::reel::Narration;
use guion_marca::Marca;
use guion_tablero::tablero::Tablero;

const TITULO: &str = "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf";
const TEXTO: &str = "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf";

fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn dir(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("guion-marca-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn faltan() -> Option<String> {
    for f in [TITULO, TEXTO] {
        if !Path::new(f).exists() {
            return Some(format!("{f} is not installed"));
        }
    }
    if guion_encode::ffmpeg::ffmpeg().is_err() {
        return Some("no ffmpeg (PATH or GUION_FFMPEG)".into());
    }
    None
}

/// A throwaway account whose faces are DejaVu and whose scenes build against
/// this workspace.
fn cuenta(d: &Path) -> PathBuf {
    let mut r = Respuestas::con_defaults(
        "El extraño arte de computar",
        "Tecnología, IA y computación, desde cero",
        "computologo",
        "computación, IA y tecnología",
    );
    r.fecha = "2026-10-18".into();
    r.handle = "@extranoarte".into();
    r.fuente_titulo = TITULO.into();
    r.fuente_texto = TEXTO.into();
    r.guion = Some(workspace().display().to_string());
    crear(&r, &d.join("arte"), None).unwrap().raiz
}

/// A 16-bit mono WAV at 48 kHz.
fn wav(path: &Path, samples: &[i16]) {
    let mut b = Vec::new();
    let data = samples.len() * 2;
    b.extend_from_slice(b"RIFF");
    b.extend_from_slice(&((36 + data) as u32).to_le_bytes());
    b.extend_from_slice(b"WAVEfmt ");
    for v in [
        16u32.to_le_bytes().as_slice(),
        &1u16.to_le_bytes(),
        &1u16.to_le_bytes(),
    ] {
        b.extend_from_slice(v);
    }
    b.extend_from_slice(&48_000u32.to_le_bytes());
    b.extend_from_slice(&96_000u32.to_le_bytes());
    b.extend_from_slice(&2u16.to_le_bytes());
    b.extend_from_slice(&16u16.to_le_bytes());
    b.extend_from_slice(b"data");
    b.extend_from_slice(&(data as u32).to_le_bytes());
    for s in samples {
        b.extend_from_slice(&s.to_le_bytes());
    }
    std::fs::write(path, b).unwrap();
}

/// The voice, stood in: the template's two lines as tones, and their
/// timeline as `guion reel voice` writes it.
fn voz(post: &Path) {
    std::fs::write(
        post.join("voz.json"),
        r#"{"modelo": "es-carlfm-x-low", "duracion_voz": 3.9, "lineas": [
 {"id": "gancho", "texto": "¿Qué es un bit?", "subtitulo": "La pregunta, en corto.", "t0": 0.5, "t1": 2.0},
 {"id": "cierre", "texto": "La frase que cierra la idea.", "subtitulo": "La frase que cierra.", "t0": 2.4, "t1": 3.7}]}"#,
    )
    .unwrap();
    let sr = 48_000.0;
    let s: Vec<i16> = (0..(3.9 * sr) as usize)
        .map(|k| {
            let t = k as f64 / sr;
            let a = if (0.5..2.0).contains(&t) || (2.4..3.7).contains(&t) {
                0.4
            } else {
                0.0
            };
            (a * (2.0 * std::f64::consts::PI * 220.0 * t).sin() * 32767.0) as i16
        })
        .collect();
    wav(&post.join("voz.wav"), &s);
}

#[test]
fn ac6_the_scene_checks_its_subtitles_with_the_gates_rules() {
    if let Some(m) = faltan() {
        eprintln!("skipped: {m}");
        return;
    }
    let d = dir("subs");
    let raiz = cuenta(&d);
    let m = Marca::cargar(&raiz).expect("loads the contract and its faces");
    assert_eq!(m.canvas.width, 1080);
    assert_eq!(
        m.fondo(),
        guion_marca::Rgb {
            r: 12,
            g: 12,
            b: 14
        }
    );
    let bien = Narration::from_json(
        r#"{"duracion_voz": 2.0, "lineas": [{"id": "a", "texto": "x", "subtitulo": "Corto y claro.", "t0": 0.5, "t1": 1.5}]}"#,
    )
    .unwrap();
    m.comprobar_subtitulos(&bien).expect("passes");
    let mal = Narration::from_json(
        r#"{"duracion_voz": 2.0, "lineas": [{"id": "a", "texto": "x", "subtitulo": "Un subtítulo con 3 cifras y muchas más palabras de la cuenta", "t0": 0.5, "t1": 1.5}]}"#,
    )
    .unwrap();
    let e = m.comprobar_subtitulos(&mal).unwrap_err();
    assert!(e.contains("sub-words") && e.contains("sub-signs"), "{e}");
    // the title shrinks to the safe area; the card holds still after 0.6 s
    let largo = "Un título larguísimo que no cabría nunca en un solo renglón del reel";
    assert!(!m.titulo(largo, 1.0).is_empty());
    assert_eq!(
        format!("{:?}", m.cierre(0.7)),
        format!("{:?}", m.cierre(2.0))
    );
}

#[test]
fn ac7_the_template_reel_builds_renders_and_passes_the_gate() {
    if let Some(m) = faltan() {
        eprintln!("skipped: {m}");
        return;
    }
    let d = dir("e2e");
    let raiz = cuenta(&d);
    // a post from the account's template, registered in its motor
    let t = Tablero::leer(std::slice::from_ref(&raiz), &|_| None).unwrap();
    let n = guion_tablero::nuevo::crear(
        &t.cuentas[0],
        "que-es-un-bit",
        Some("¿Qué es un bit?"),
        "2026-10-18",
    )
    .unwrap();
    let post = n.dir;
    voz(&post);

    // the scene, built against guion-marca alone
    let motor = raiz.join("motor");
    std::fs::copy(workspace().join("Cargo.lock"), motor.join("Cargo.lock")).unwrap();
    let target = workspace().join("target/cuenta-e2e");
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let st = Command::new(cargo)
        .args([
            "build",
            "--release",
            "-q",
            "--bin",
            "que-es-un-bit",
            "--manifest-path",
        ])
        .arg(motor.join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", &target)
        .status()
        .expect("cargo runs");
    assert!(st.success(), "the template scene does not build");
    let bin = target.join("release/que-es-un-bit");

    // rendered twice: same bytes (the gate's determinism check)
    let mp4 = post.join("que-es-un-bit.mp4");
    let primero = post.join("primero.mp4");
    for k in 0..2 {
        let out = Command::new(&bin).current_dir(&post).output().unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        if k == 0 {
            std::fs::rename(&mp4, &primero).unwrap();
        }
    }
    let ev = std::fs::read_to_string(post.join("eventos.json")).unwrap();
    assert!(
        ev.contains("\"voz:gancho\":0.500000") && ev.contains("\"cierre\":2.100000"),
        "{ev}"
    );

    // the bed with the neutral kit, then the platform's loudness
    let s = guion_marca::sonido::render(&post);
    assert!(s.is_ok(), "{s:?}");
    let audio_mp4 = post.join("que-es-un-bit-audio.mp4");
    mux_at_loudness(
        &mp4,
        &post.join("audio.wav"),
        &audio_mp4,
        LoudnessTarget::REELS,
    )
    .unwrap();

    // the whole gate, with the profile the scaffold derived
    let p = Profile::load(&raiz.join("brand/reel-perfil.toml")).unwrap();
    let mut i = Inputs::in_post(&post, "que-es-un-bit");
    i.max_duration = Some(90.0);
    i.compare = Some((primero, mp4));
    let r = gate::run(&i, &p).expect("the gate runs");
    assert!(r.pass, "the template must pass its gate:\n{}", r.summary());
    assert_eq!(r.findings.len(), 18, "{}", r.summary());
}
