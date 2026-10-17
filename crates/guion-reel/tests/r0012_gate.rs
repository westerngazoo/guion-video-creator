//! R-0012 — the narrated reel's automatic gate: one test (or more) per
//! acceptance criterion. The end-to-end test encodes a small synthetic reel
//! with known faults (ffmpeg with libx264, in PATH or `GUION_FFMPEG`) and
//! measures subtitles in DejaVu Sans (skipped, saying so, where it is not
//! installed).

use std::path::{Path, PathBuf};

use guion_encode::{mux_at_loudness, Encoder, LoudnessTarget, VideoSpec};
use guion_reel::gate::{self, Inputs, Profile};

const FONT: &str = "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf";
const W: usize = 216;
const H: usize = 384;
const FPS: f64 = 30.0;

fn dir(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("guion-gate-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn profile_text(font: &str) -> String {
    format!(
        r#"
[brand]
name = "Test"
[canvas]
width = {W}
height = {H}
fps = 30.0
[tokens]
background = [10, 10, 11]
bone = [244, 242, 236]
red = [224, 50, 42]
gold = [233, 178, 63]
[roles]
background = ["background"]
red = ["red"]
focus = ["gold"]
[subtitle]
font = "{font}"
size = 12.0
size_unit = "em"
max_width = 150.0
center_x = 108.0
baseline = 300.0
band = [288.0, 306.0]
max_words = 4
forbidden = "=%"
forbid_digits = true
long_line = 6.0
fade = 0.15
[safe]
x = [18.0, 198.0]
y = [40.0, 310.0]
except = [[0.0, 0.0, 30.0, 30.0]]
[motion]
max_still_with_voice = 0.8
closing_hold = 1.0
[scan]
stride = 3
tolerance = 30.0
merge = 8
min_area = 30
min_stray = 10
[audio]
lufs = -14.0
tolerance = 0.5
true_peak = -1.0
short_gap = 0.5
gap_max_db = -15.0
[caption]
max_hashtags = 5
max_chars = 2200
units = ["%", "g"]
internal = ["R-####", "guion-reel"]
"#
    )
}

#[test]
fn ac1_the_profile_is_the_brands_and_fails_by_name() {
    let p = Profile::parse(&profile_text("f.ttf")).expect("parses");
    assert_eq!(p.tokens["gold"], [233, 178, 63]);
    let typo = profile_text("f.ttf").replace("max_words", "max_wrods");
    assert!(Profile::parse(&typo)
        .unwrap_err()
        .to_string()
        .contains("max_wrods"));
    let unknown = profile_text("f.ttf").replace(r#"red = ["red"]"#, r#"red = ["crimson"]"#);
    let e = Profile::parse(&unknown).unwrap_err().to_string();
    assert!(e.contains("crimson") && e.contains("roles.red"), "{e}");
}

fn rect(rgb: &mut [u8], x0: usize, y0: usize, x1: usize, y1: usize, c: [u8; 3]) {
    for y in y0..y1 {
        for x in x0..x1 {
            rgb[(y * W + x) * 3..(y * W + x) * 3 + 3].copy_from_slice(&c);
        }
    }
}

/// A 16-bit mono WAV.
fn wav(path: &Path, samples: &[i16]) {
    let mut b = Vec::new();
    let data = samples.len() * 2;
    b.extend_from_slice(b"RIFF");
    b.extend_from_slice(&((36 + data) as u32).to_le_bytes());
    b.extend_from_slice(b"WAVEfmt ");
    b.extend_from_slice(&16u32.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
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

/// A 4 s reel: a square that moves until 2.0 s, then holds still to the end
/// (2 s ≥ the 1 s hold, but 1.2 s of it under the second line > 0.8);
/// a second red square from 1.0 to 1.5 s; a gold blob outside the safe
/// area from 0.5 to 1.0 s; two lines of voice, the second with a long
/// subtitle and a digit; a caption with an internal id and a bare unit.
fn synthetic(d: &Path) -> Inputs {
    let n = (4.0 * FPS) as usize;
    let mp4 = d.join("x.mp4");
    let mut enc = Encoder::start(
        &mp4,
        VideoSpec {
            width: W as u32,
            height: H as u32,
            fps: FPS,
        },
    )
    .unwrap();
    for i in 0..n {
        let t = i as f64 / FPS;
        let mut f = vec![0u8; W * H * 3];
        rect(&mut f, 0, 0, W, H, [10, 10, 11]);
        let x = 40 + (t.min(2.0) * 40.0) as usize;
        rect(&mut f, x, 100, x + 24, 124, [224, 50, 42]);
        if (1.0..1.5).contains(&t) {
            rect(&mut f, 60, 200, 84, 224, [224, 50, 42]);
        }
        if (0.5..1.0).contains(&t) {
            rect(&mut f, 200, 150, 214, 170, [233, 178, 63]);
        }
        enc.push(&f).unwrap();
    }
    enc.finish().unwrap();
    // voice: a tone in each line, the bed quiet between them
    let sr = 48_000.0;
    let s: Vec<i16> = (0..(4.0 * sr) as usize)
        .map(|k| {
            let t = k as f64 / sr;
            let a = if (0.2..1.2).contains(&t) || (1.5..3.0).contains(&t) {
                0.4
            } else {
                0.004
            };
            (a * (2.0 * std::f64::consts::PI * 220.0 * t).sin() * 32767.0) as i16
        })
        .collect();
    wav(&d.join("a.wav"), &s);
    let out = d.join("x-audio.mp4");
    mux_at_loudness(&mp4, &d.join("a.wav"), &out, LoudnessTarget::REELS).unwrap();
    std::fs::write(
        d.join("voz.json"),
        r#"{"duracion_voz": 3.0, "lineas": [
 {"id": "a", "texto": "One.", "subtitulo": "Short one.", "t0": 0.2, "t1": 1.2, "frases": [{"texto": "One.", "t0": 0.2, "t1": 1.2}]},
 {"id": "b", "texto": "Two.", "subtitulo": "A much longer subtitle with 2 things", "t0": 1.5, "t1": 3.0, "frases": [{"texto": "Two.", "t0": 1.5, "t1": 3.0}]}]}"#,
    )
    .unwrap();
    std::fs::write(
        d.join("eventos.json"),
        r#"{"duracion": 4.0, "fps": 30.0, "eventos": {}, "series": {}}"#,
    )
    .unwrap();
    std::fs::write(
        d.join("caption.md"),
        "# x\n\n## Caption\n\nEl giro es de 0.6 g, como en R-0009.\n\n## Hashtags\n\n#f1\n\n## Alt text\n\nUn cuadro rojo.\n",
    )
    .unwrap();
    let mut i = Inputs::in_post(d, "x");
    i.max_duration = Some(60.0);
    i
}

fn finding<'a>(r: &'a gate::Report, name: &str) -> &'a gate::Finding {
    r.findings
        .iter()
        .find(|f| f.check == name)
        .unwrap_or_else(|| panic!("no finding «{name}»"))
}

#[test]
fn ac2_to_ac8_a_synthetic_reel_fails_where_it_should() {
    if !Path::new(FONT).exists() {
        eprintln!("skipped: {FONT} is not installed");
        return;
    }
    let d = dir("e2e");
    let mut inputs = synthetic(&d);
    let p = Profile::parse(&profile_text(FONT)).unwrap();
    // AC9: two renders, compared
    std::fs::copy(d.join("x.mp4"), d.join("y.mp4")).unwrap();
    inputs.compare = Some((d.join("x.mp4"), d.join("y.mp4")));
    let r = gate::run(&inputs, &p).expect("the gate runs");

    // AC2: the subtitles
    assert!(
        !finding(&r, "sub-width").pass,
        "{:?}",
        finding(&r, "sub-width")
    );
    assert!(!finding(&r, "sub-words").pass);
    assert!(!finding(&r, "sub-signs").pass);
    assert!(finding(&r, "sub-overlap").pass);
    // AC3: one red region per frame
    let red = finding(&r, "red");
    assert!(!red.pass && red.at[0].starts_with("1.00"), "{red:?}");
    assert!(finding(&r, "focus").pass);
    // AC4: the safe area (the gold blob at x ≥ 200)
    let safe = finding(&r, "safe-area");
    assert!(!safe.pass && safe.at[0].contains("0.50"), "{safe:?}");
    assert!(
        finding(&r, "sub-band").pass,
        "{:?}",
        finding(&r, "sub-band")
    );
    // AC5: still under the voice, and the closing hold
    let still = finding(&r, "still-voice");
    assert!(!still.pass && still.at[0].starts_with("2.0"), "{still:?}");
    assert!(
        finding(&r, "closing-hold").pass,
        "{:?}",
        finding(&r, "closing-hold")
    );
    assert!(
        finding(&r, "duration").pass,
        "{:?}",
        finding(&r, "duration")
    );
    // AC6: the audio
    assert!(
        finding(&r, "loudness").pass,
        "{:?}",
        finding(&r, "loudness")
    );
    // AC7: the caption
    assert!(finding(&r, "caption").pass);
    assert!(!finding(&r, "units").pass);
    assert!(!finding(&r, "internal").pass);
    // AC9
    assert!(finding(&r, "determinism").pass);
    // AC8: the report
    assert!(!r.pass);
    assert!(r.summary().lines().count() <= 40);
    let json: serde_json::Value = serde_json::from_str(&r.to_json()).unwrap();
    assert_eq!(json["pass"], false);
    assert_eq!(json["findings"].as_array().unwrap().len(), r.findings.len());
}

#[test]
fn ac9_two_renders_that_differ_fail() {
    let d = dir("det");
    std::fs::write(d.join("a"), b"one").unwrap();
    std::fs::write(d.join("b"), b"two").unwrap();
    std::fs::write(d.join("c"), b"one").unwrap();
    assert!(
        !gate::determinism(Some(&(d.join("a"), d.join("b"))))
            .unwrap()
            .pass
    );
    assert!(
        gate::determinism(Some(&(d.join("a"), d.join("c"))))
            .unwrap()
            .pass
    );
}

#[test]
fn ac8_a_missing_input_is_an_error_naming_it() {
    let d = dir("missing");
    let i = Inputs::in_post(&d, "nothing");
    let e = gate::run(&i, &Profile::parse(&profile_text(FONT)).unwrap()).unwrap_err();
    assert!(e.to_string().contains("nothing-audio.mp4"), "{e}");
}
