//! R-0016 — formats: the canvas from the format, per-format bands in the
//! gate profile, and the YouTube deliverables (`.srt`, chapters, thumbnail,
//! title and description), one test (or more) per acceptance criterion.
//!
//! The end-to-end test renders a 16:9 reel with guion-reel and gates it as
//! a long YouTube video (ffmpeg with libx264, in PATH or `GUION_FFMPEG`;
//! subtitles measured in DejaVu Sans, skipped, saying so, where it is not
//! installed).

use std::path::{Path, PathBuf};

use guion_encode::{mux_at_loudness, LoudnessTarget};
use guion_publish::youtube::{self as yt, Capitulo};
use guion_reel::gate::{self, Inputs, Profile};
use guion_reel::youtube::{
    check_thumbnail, cues, parse_srt, png_size, srt, THUMB_H, THUMB_MAX_BYTES, THUMB_W,
};
use guion_reel::{Canvas, Clock, Command, Formato, Frame, Narration, NoOverlay, Reel, ReelError};
use motoreel::shapes::polygon;
use motoreel::{Fill, Prim2, Rgb, Style};
use motoreel_typeset::Fonts;

const FONT: &str = "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf";

fn dir(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("guion-r16-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// The R-0012 test brand (a 216 × 384 vertical reel), and what it writes for
/// a long YouTube video: a small 16:9 canvas at 10 fps (so the test runs in
/// seconds) with its subtitle row and safe area.
fn profile_text(font: &str) -> String {
    format!(
        r#"
[brand]
name = "Test"
[canvas]
width = 216
height = 384
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
[formula]
band = [200.0, 240.0]
tokens = ["gold"]
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

[formato.youtube]
canvas = {{ width = 384, height = 216, fps = 10.0 }}
subtitle = {{ band = [188.0, 206.0], baseline = 200.0, center_x = 192.0, max_width = 300.0 }}
safe = {{ x = [20.0, 364.0], y = [12.0, 206.0] }}

[formato.short]
max_duration = 120.0
"#
    )
}

// --- AC1, AC2: the format, the canvas and the bands ---

#[test]
fn ac1_the_format_sets_the_canvas() {
    assert_eq!(Formato::Reel.canvas(), Canvas::VERTICAL_REEL);
    assert_eq!(Formato::Tiktok.canvas(), Canvas::VERTICAL_REEL);
    assert_eq!(Formato::Short.canvas(), Canvas::VERTICAL_REEL);
    assert_eq!(Formato::Youtube.canvas(), Canvas::HORIZONTAL_VIDEO);
    assert_eq!(
        (
            Canvas::HORIZONTAL_VIDEO.width,
            Canvas::HORIZONTAL_VIDEO.height
        ),
        (1920, 1080)
    );
    // the view keeps its scale: the short side is 1.8 units in both
    assert_eq!(Canvas::HORIZONTAL_VIDEO.view(), (3.2, 1.8));
    assert_eq!(Canvas::VERTICAL_REEL.view(), (1.8, 3.2));
    assert!(Formato::de("cuadrado").unwrap_err().contains("youtube"));
}

#[test]
fn ac2_bands_per_format_defaults_here_overrides_in_the_profile() {
    let p = Profile::parse(&profile_text("f.ttf")).unwrap();
    // the brand's own orientation, nothing written for it: the profile itself
    assert_eq!(p.para(Formato::Reel), p);
    assert_eq!(p.para(Formato::Tiktok), p);
    assert_eq!(p.para(Formato::Short), p);
    // the other orientation: the house's defaults, then the brand's overrides
    let mut bare = p.clone();
    bare.formato.clear();
    let d = bare.para(Formato::Youtube);
    let b = Formato::Youtube.bandas();
    assert_eq!(
        (d.canvas.width, d.canvas.height, d.canvas.fps),
        (1920, 1080, 30.0)
    );
    assert_eq!(d.subtitle.band, b.subtitle_band);
    assert_eq!((d.safe.x, d.safe.y), (b.safe_x, b.safe_y));
    assert!(
        d.safe.except.is_empty(),
        "the 9:16 watermark does not carry over"
    );
    assert!(
        d.formula.is_none(),
        "a formula band is the brand's, in its format"
    );
    assert_eq!(
        d.subtitle.size, p.subtitle.size,
        "the face and its size stay the brand's"
    );
    let y = p.para(Formato::Youtube);
    assert_eq!(
        (y.canvas.width, y.canvas.height, y.canvas.fps),
        (384, 216, 10.0)
    );
    assert_eq!(y.subtitle.band, [188.0, 206.0]);
    assert_eq!(y.safe.x, [20.0, 364.0]);
    // max duration: the platform's, tightened by the brand, never loosened
    assert_eq!(p.max_duration(Formato::Short), Some(120.0));
    assert_eq!(bare.max_duration(Formato::Short), Some(180.0));
    assert_eq!(p.max_duration(Formato::Reel), None);
    let loose = profile_text("f.ttf").replace("max_duration = 120.0", "max_duration = 600.0");
    assert_eq!(
        Profile::parse(&loose).unwrap().max_duration(Formato::Short),
        Some(180.0)
    );
    // unknown formats and keys fail by name
    let e = Profile::parse(&profile_text("f.ttf").replace("[formato.short]", "[formato.vertical]"))
        .unwrap_err()
        .to_string();
    assert!(
        e.contains("formato.vertical") && e.contains("youtube"),
        "{e}"
    );
    let e = Profile::parse(
        &profile_text("f.ttf").replace("max_duration = 120.0", "max_durasion = 1.0"),
    )
    .unwrap_err()
    .to_string();
    assert!(e.contains("max_durasion"), "{e}");
}

// --- AC3: 9:16 is byte-identical ---

fn no_fonts() -> guion_reel::FontLoader {
    Box::new(|| Ok(Fonts::new()))
}

fn square(c: Canvas, t: f64, x0: f64, speed: f64, y: f64, side: f64) -> Vec<Prim2> {
    let x = x0 + speed * t;
    let pts =
        [(x, y), (x + side, y), (x + side, y + side), (x, y + side)].map(|(x, y)| c.to_view(x, y));
    let red = Rgb {
        r: 224,
        g: 50,
        b: 42,
    };
    vec![Prim2::Path {
        subpaths: polygon(&pts),
        style: Style {
            stroke: red,
            width: 0.0,
            alpha: 0.0,
            fill: Some(Fill::solid(red, 1.0)),
        },
    }]
}

const BG: Rgb = Rgb {
    r: 10,
    g: 10,
    b: 11,
};

#[test]
fn ac3_a_vertical_reel_renders_the_same_bytes_through_the_format() {
    // what a scene did before (its own 9:16 canvas) and what it does now
    // (the canvas of the format `guion reel` asks for): the same MP4
    std::env::remove_var(guion_reel::formato::VARIABLE);
    let by_format = Formato::desde_entorno().unwrap().canvas();
    let small = |c: Canvas| Canvas {
        width: c.width / 10,
        height: c.height / 10,
        px_per_unit: c.px_per_unit / 10.0,
        ..c
    };
    let mut bytes = Vec::new();
    for (k, c) in [Canvas::VERTICAL_REEL, by_format].into_iter().enumerate() {
        let c = small(c);
        let d = dir(&format!("ident{k}"));
        let r = Reel::new("x", d.clone(), 0.5, BG, Clock::new(), c, no_fonts());
        r.run(&Command::Video { threads: 2 }, |t| -> Frame<NoOverlay> {
            (square(c, t, 10.0, 40.0, 50.0, 20.0), Vec::new())
        })
        .unwrap();
        bytes.push((
            std::fs::read(d.join("x.mp4")).unwrap(),
            std::fs::read(d.join("events.json")).unwrap(),
        ));
    }
    assert_eq!(bytes[0], bytes[1]);
    // and the gate's profile for a reel is the profile it always read
    let p = Profile::parse(&profile_text("f.ttf")).unwrap();
    assert_eq!(p.para(Formato::Reel), p);
}

// --- AC4: the .srt, timed by voz.json ---

const VOZ: &str = r#"{"duracion_voz": 29.5, "lineas": [
 {"id": "gancho", "texto": "Why does a kart lift a wheel? Load moves out.", "subtitulo": "Why lift a wheel", "t0": 0.5, "t1": 9.5, "desde": 2,
  "frases": [{"texto": "Why does a kart lift a wheel?", "t0": 0.5, "t1": 4.25}, {"texto": "Load moves out.", "t0": 4.75, "t1": 9.5}]},
 {"id": "modelo", "texto": "The model is a box on four springs. It leans.", "subtitulo": "A box that leans", "t0": 10.5, "t1": 19.5, "desde": 2,
  "frases": [{"texto": "The model is a box on four springs.", "t0": 10.5, "t1": 14.0004}, {"texto": "It leans.", "t0": 14.5, "t1": 19.5}]},
 {"id": "pista", "texto": "On track, the inner wheel goes light. That is all.", "subtitulo": "The inner wheel", "t0": 20.5, "t1": 29.5, "desde": 2,
  "frases": [{"texto": "On track, the inner wheel goes light.", "t0": 20.5, "t1": 25.0}, {"texto": "That is all.", "t0": 25.5, "t1": 29.5}]}]}"#;

#[test]
fn ac4_the_srt_is_timed_by_voz_json() {
    let n = Narration::from_json(VOZ).unwrap();
    let text = srt(&n);
    assert!(
        text.starts_with("1\n00:00:00,500 --> 00:00:04,250\nWhy does a kart lift a wheel?\n\n"),
        "{text}"
    );
    let back = parse_srt(&text).unwrap();
    let sentences: Vec<_> = n.lines.iter().flat_map(|l| &l.sentences).collect();
    assert_eq!(back.len(), sentences.len());
    for (c, s) in back.iter().zip(sentences) {
        assert_eq!(c.t0, (s.t0 * 1000.0).round() as u64);
        assert_eq!(c.t1, (s.t1 * 1000.0).round() as u64);
        assert_eq!(c.text.replace('\n', " "), s.text);
    }
    assert_eq!(back, cues(&n));
    // CRLF and a BOM (what other tools write) read the same
    let crlf = format!("\u{feff}{}", text.replace('\n', "\r\n"));
    assert_eq!(parse_srt(&crlf).unwrap(), back);
}

// --- AC5: chapters, from the gate's side ---

#[test]
fn ac5_the_gate_reads_chapters_back_from_the_description() {
    let d = dir("chapters");
    let desc = d.join("descripcion.txt");
    std::fs::write(
        &desc,
        "Text.\n\n00:00 One\n00:12 Two\n00:31 Three\n\nProvenance.\n",
    )
    .unwrap();
    assert!(gate::youtube::chapters(&desc, 45.0).pass);
    // the last chapter would last 9 s
    let f = gate::youtube::chapters(&desc, 40.0);
    assert!(
        !f.pass && f.at.iter().any(|m| m.contains("last chapter")),
        "{f:?}"
    );
    std::fs::write(&desc, "00:05 One\n00:12 Two\n00:31 Three\n").unwrap();
    let f = gate::youtube::chapters(&desc, 45.0);
    assert!(
        !f.pass && f.at.iter().any(|m| m.contains("not 00:00")),
        "{f:?}"
    );
    std::fs::write(&desc, "00:00 One\n00:31 Three\n").unwrap();
    assert!(!gate::youtube::chapters(&desc, 45.0).pass);
}

// --- AC6: the thumbnail ---

#[test]
fn ac6_the_thumbnail_is_1280_by_720_and_light() {
    let c = Canvas::HORIZONTAL_VIDEO;
    let d = dir("thumb");
    let r = Reel::new("x", d.clone(), 2.0, BG, Clock::new(), c, no_fonts());
    let frame =
        |t: f64| -> Frame<NoOverlay> { (square(c, t, 200.0, 300.0, 400.0, 240.0), Vec::new()) };
    let out = r.run(&Command::Miniatura(1.0), frame).unwrap();
    assert_eq!(out, vec![d.join("miniatura.png")]);
    let bytes = std::fs::read(&out[0]).unwrap();
    assert_eq!(png_size(&bytes), Some((THUMB_W, THUMB_H)));
    let n = check_thumbnail(&out[0]).unwrap();
    assert!(n <= THUMB_MAX_BYTES);
    // deterministic: the same frame, the same bytes
    r.run(&Command::Miniatura(1.0), frame).unwrap();
    assert_eq!(std::fs::read(&out[0]).unwrap(), bytes);
    assert!(gate::youtube::thumbnail(&out[0]).pass);
    // the flag
    let args: Vec<String> = ["--miniatura", "1.0"].map(String::from).to_vec();
    assert_eq!(Command::from_args(&args).unwrap(), Command::Miniatura(1.0));
    // a 9:16 canvas has no 16:9 thumbnail
    let v = Reel::new(
        "v",
        d.clone(),
        2.0,
        BG,
        Clock::new(),
        Canvas::VERTICAL_REEL,
        no_fonts(),
    );
    let e = v.miniatura(1.0, &frame).unwrap_err();
    assert!(
        matches!(e, ReelError::Format(ref m) if m.contains("1080×1920")),
        "{e}"
    );
    // a wrong size fails the check, naming it
    std::fs::write(d.join("small.png"), {
        let mut b = bytes.clone();
        b[16..20].copy_from_slice(&640u32.to_be_bytes());
        b
    })
    .unwrap();
    let f = gate::youtube::thumbnail(&d.join("small.png"));
    assert!(!f.pass && f.at[0].contains("640×720"), "{f:?}");
}

// --- AC7: a 16:9 reel, rendered and gated as a long YouTube video ---

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

#[test]
fn ac7_a_16_9_reel_renders_and_passes_the_gate_as_youtube() {
    if !Path::new(FONT).exists() {
        eprintln!("skipped: {FONT} is not installed");
        return;
    }
    let base = Profile::parse(&profile_text(FONT)).unwrap();
    let p = base.para(Formato::Youtube);
    let c = Canvas {
        width: p.canvas.width,
        height: p.canvas.height,
        fps: p.canvas.fps,
        px_per_unit: f64::from(p.canvas.height) / 1.8,
    };
    let d = dir("e2e");
    std::fs::write(d.join("voz.json"), VOZ).unwrap();
    let n = Narration::load(&d.join("voz.json")).unwrap();
    // the scene: a red square crossing the frame until 30 s, then the closing hold
    let clock = Clock::new()
        .event("beat-1", 10.0)
        .unwrap()
        .event("beat-2", 20.0)
        .unwrap();
    let r =
        Reel::new("x", d.clone(), 32.0, BG, clock, c, no_fonts()).with_layout(guion_reel::Layout {
            events: "eventos.json".into(),
            review: "cuadros".into(),
            sheet: "hoja.png".into(),
        });
    let frame = |t: f64| -> Frame<NoOverlay> {
        (square(c, t.min(30.0), 30.0, 10.0, 90.0, 24.0), Vec::new())
    };
    r.run(&Command::Video { threads: 2 }, frame).unwrap();
    r.run(&Command::Miniatura(15.0), frame).unwrap();
    // the voice: a tone in each line, the bed quiet between them
    let sr = 48_000.0;
    let s: Vec<i16> = (0..(32.0 * sr) as usize)
        .map(|k| {
            let t = k as f64 / sr;
            let a = if n.lines.iter().any(|l| (l.t0..l.t1).contains(&t)) {
                0.4
            } else {
                0.004
            };
            (a * (2.0 * std::f64::consts::PI * 220.0 * t).sin() * 32767.0) as i16
        })
        .collect();
    wav(&d.join("a.wav"), &s);
    mux_at_loudness(
        &d.join("x.mp4"),
        &d.join("a.wav"),
        &d.join("x-audio.mp4"),
        LoudnessTarget::REELS,
    )
    .unwrap();
    // the texts: chapters on the voice's lines (as `guion reel` resolves them)
    std::fs::write(d.join("x.srt"), srt(&n)).unwrap();
    let fuente = yt::parse(
        "## Title\nWhy does a kart lift a wheel?\n## Description\nLoad transfer, in a corner.\n\
         ## Chapters\n- inicio | The question\n- modelo | The model\n- pista | On track\n\
         ## Provenance\nThe numbers are computed with the reel's model, not measured.\n",
        "youtube.md",
    )
    .unwrap();
    let caps: Vec<Capitulo> = [0.0, n.span("modelo").unwrap().0, n.span("pista").unwrap().0]
        .iter()
        .zip(&fuente.capitulos)
        .map(|(&t, (_, titulo))| Capitulo {
            inicio: t,
            titulo: titulo.clone(),
        })
        .collect();
    yt::export(&fuente, &caps, 32.0, "youtube.md")
        .unwrap()
        .write_to(&d)
        .unwrap();

    let mut inputs = Inputs::in_post(&d, "x");
    inputs.formato = Formato::Youtube;
    inputs.max_duration = Some(900.0);
    let report = gate::run(&inputs, &base).expect("the gate runs");
    let failed: Vec<_> = report.findings.iter().filter(|f| !f.pass).collect();
    assert!(report.pass, "{failed:#?}");
    let names: Vec<&str> = report.findings.iter().map(|f| f.check.as_str()).collect();
    for want in [
        "srt",
        "chapters",
        "thumbnail",
        "yt-title",
        "yt-description",
        "safe-area",
        "duration",
    ] {
        assert!(names.contains(&want), "no «{want}» in {names:?}");
    }
    assert!(
        !names.contains(&"caption"),
        "a long YouTube video has no Instagram caption"
    );

    // what fails, fails where it should: a moved cue, a lost chapter
    std::fs::write(
        d.join("x.srt"),
        srt(&n).replace("00:00:04,250", "00:00:04,300"),
    )
    .unwrap();
    std::fs::write(d.join("descripcion.txt"), "00:00 One\n00:25 Two\n").unwrap();
    let report = gate::run(&inputs, &base).unwrap();
    let get = |c: &str| {
        report
            .findings
            .iter()
            .find(|f| f.check == c)
            .unwrap()
            .clone()
    };
    assert!(
        !get("srt").pass && get("srt").at[0].contains("cue 1"),
        "{:?}",
        get("srt")
    );
    assert!(!get("chapters").pass);
    // as a Short (9:16 only), the same 16:9 MP4 is refused for its size
    inputs.formato = Formato::Short;
    let e = gate::run(&inputs, &base).unwrap_err().to_string();
    assert!(e.contains("384×216") && e.contains("216×384"), "{e}");
}
