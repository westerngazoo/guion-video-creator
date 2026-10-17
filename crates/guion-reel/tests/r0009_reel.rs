//! R-0009 — the reel studio: one test (or more) per acceptance criterion.
//!
//! The video, PNG and contact-sheet tests need ffmpeg with libx264 (the one in
//! PATH or in `GUION_FFMPEG`), like R-0005's: without it they fail, and the
//! error says how to provide it.

use std::path::{Path, PathBuf};
use std::process::Command as Process;

use guion_reel::subtitle::{break_rows, is_formula_word, row_width, units, word_count, Piece};
use guion_reel::{
    Canvas, Clock, Command, FontLoader, Frame, Narration, NoOverlay, Overlay, Reel, ReelError,
};
use motoreel::shapes::polygon;
use motoreel::{Fill, Prim2, Rgb, Style};
use motoreel_typeset::Fonts;

/// A small canvas, so the tests run in seconds: 64 × 48 px at 30 fps.
const CANVAS: Canvas = Canvas {
    width: 64,
    height: 48,
    fps: 30.0,
    px_per_unit: 32.0,
};
const BLACK: Rgb = Rgb { r: 0, g: 0, b: 0 };

fn no_fonts() -> FontLoader {
    Box::new(|| Ok(Fonts::new()))
}

/// A new, empty folder for each test.
fn dir(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("guion-reel-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn reel(name: &str, duration: f64, clock: Clock) -> Reel {
    Reel::new(name, dir(name), duration, BLACK, clock, CANVAS, no_fonts())
}

/// A square that slides with t: every frame differs from the last.
fn square(t: f64) -> Vec<Prim2> {
    let x = 8.0 + 40.0 * t;
    let pts = [(x, 12.0), (x + 12.0, 12.0), (x + 12.0, 30.0), (x, 30.0)]
        .map(|(x, y)| CANVAS.to_view(x, y));
    let gold = Rgb {
        r: 227,
        g: 169,
        b: 58,
    };
    vec![Prim2::Path {
        subpaths: polygon(&pts),
        style: Style {
            stroke: gold,
            width: 0.0,
            alpha: 0.0,
            fill: Some(Fill::solid(gold, 1.0)),
        },
    }]
}

fn plain(t: f64) -> Frame<NoOverlay> {
    (square(t), Vec::new())
}

/// The RGB24 pixels of a PNG, decoded by ffmpeg.
fn decode(png: &Path) -> Vec<u8> {
    let ff = guion_encode::ffmpeg::ffmpeg()
        .expect("these tests need ffmpeg (in PATH or in GUION_FFMPEG)");
    let out = Process::new(ff)
        .args(["-hide_banner", "-loglevel", "error", "-i"])
        .arg(png)
        .args(["-f", "rawvideo", "-pix_fmt", "rgb24", "-"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "ffmpeg could not read {}",
        png.display()
    );
    out.stdout
}

// ---------- AC1: the clock and events.json ----------

#[test]
fn ac1_events_json_has_the_schema_bytes() {
    let clock = Clock::new()
        .event("titulo", 5.316671)
        .unwrap()
        .event("voz:gancho", 0.5)
        .unwrap()
        .series("vueltas", 0.0, 0.5, vec![0.0, 0.25, 1.0 / 3.0])
        .unwrap();
    assert_eq!(
        clock.to_json(89.0, 30.0),
        r#"{"duracion":89.000000,"fps":30.000000,"eventos":{"titulo":5.316671,"voz:gancho":0.500000},"series":{"vueltas":{"t0":0.000000,"dt":0.500000,"v":[0.000000,0.250000,0.333333]}}}"#
    );
    let r = reel("ac1", 1.0, clock);
    let path = r.write_events().unwrap();
    assert_eq!(path.file_name().unwrap(), "events.json");
    assert_eq!(
        std::fs::read_to_string(path).unwrap(),
        r.clock.to_json(1.0, 30.0)
    );
}

#[test]
fn ac1_an_empty_clock_is_valid_json() {
    assert_eq!(
        Clock::new().to_json(2.0, 30.0),
        r#"{"duracion":2.000000,"fps":30.000000,"eventos":{},"series":{}}"#
    );
}

#[test]
fn ac1_bad_entries_fail_naming_the_entry() {
    for t in [f64::NAN, -0.1, f64::INFINITY] {
        let e = Clock::new().event("qed", t).unwrap_err();
        assert!(
            matches!(&e, ReelError::InvalidClock { name, .. } if name == "qed"),
            "{e}"
        );
    }
    let e = Clock::new()
        .series("angulo", 0.0, 0.1, vec![1.0, f64::NAN])
        .unwrap_err();
    assert!(
        e.to_string().contains("angulo") && e.to_string().contains("sample 1"),
        "{e}"
    );
    assert!(Clock::new().series("angulo", 0.0, 0.0, vec![1.0]).is_err());
}

// ---------- AC2: the parallel MP4 is the one-thread MP4 ----------

#[test]
fn ac2_parallel_video_is_byte_identical_to_one_thread() {
    let r = reel("ac2", 1.0, Clock::new());
    assert_eq!(r.frame_count(), 30);
    let one = r.dir.join("one.mp4");
    let many = r.dir.join("many.mp4");
    r.video(&one, 1, &plain).unwrap();
    r.video(&many, 3, &plain).unwrap();
    let (a, b) = (std::fs::read(&one).unwrap(), std::fs::read(&many).unwrap());
    assert!(!a.is_empty());
    assert_eq!(a, b, "the MP4 with 3 threads differs from the one with 1");
    // the frame folders are gone
    assert!(!one.with_extension("frames").exists() && !many.with_extension("frames").exists());
}

#[test]
fn ac2_run_writes_the_events_and_the_mp4() {
    let r = reel("ac2run", 0.5, Clock::new().event("a", 0.1).unwrap());
    let written = r.run(&Command::Video { threads: 2 }, plain).unwrap();
    assert_eq!(
        written,
        vec![r.dir.join("events.json"), r.dir.join("ac2run.mp4")]
    );
    assert!(written.iter().all(|p| p.exists()));
}

// ---------- AC3: review frames, the contact sheet, the command line ----------

#[test]
fn ac3_frames_png_are_the_frames() {
    let r = reel("ac3png", 1.0, Clock::new());
    let pngs = r.frames_png(&[0.0, 0.5], &plain).unwrap();
    assert_eq!(
        pngs,
        vec![
            r.dir.join("frames/t-00.00.png"),
            r.dir.join("frames/t-00.50.png")
        ]
    );
    let (a, b) = (decode(&pngs[0]), decode(&pngs[1]));
    assert_eq!(a.len(), CANVAS.rgb_len());
    assert_ne!(a, b, "two different times gave the same frame");
}

#[test]
fn ac3_contact_sheet_is_the_grid_of_reduced_frames() {
    let r = reel("ac3sheet", 1.0, Clock::new());
    let times = [0.0, 0.2, 0.4, 0.6, 0.8];
    let png = r.contact_sheet(&times, &plain).unwrap();
    assert_eq!(png, r.dir.join("frames/sheet.png"));
    // 5 cells in rows of 4: 2 rows of (64/4) × (48/4) cells, 6 px apart
    let (w, h) = (16, 12);
    let (aw, ah) = (4 * w + 5 * 6, 2 * h + 3 * 6);
    let sheet = decode(&png);
    assert_eq!(sheet.len(), aw * ah * 3);
    // the first cell is the mean of each 4×4 block of the frame at t = 0
    let full = decode(&r.frames_png(&[0.0], &plain).unwrap()[0]);
    for (y, x) in [(0, 0), (5, 3), (11, 15)] {
        for c in 0..3 {
            let mut acc = 0u32;
            for dy in 0..4 {
                for dx in 0..4 {
                    acc += full[3 * ((4 * y + dy) * 64 + 4 * x + dx) + c] as u32;
                }
            }
            let cell = sheet[3 * ((6 + y) * aw + 6 + x) + c];
            assert_eq!(
                cell as u32,
                (acc + 8) / 16,
                "cell pixel ({x}, {y}) channel {c}"
            );
        }
    }
}

#[test]
fn ac3_key_times_are_the_middle_of_each_stretch() {
    let clock = Clock::new()
        .event("b", 2.0)
        .unwrap()
        .event("a", 0.0)
        .unwrap()
        .event("a2", 0.0)
        .unwrap();
    let r = reel("ac3keys", 3.0, clock);
    // events 0, 0, 2 and the end 3 → stretches [0, 2], [2, 3]; 2.5 s is frame 75
    assert_eq!(r.key_times(), vec![1.0, 2.5]);
    let r = reel("ac3keys2", 1.0, Clock::new().event("x", 0.31).unwrap());
    // (0.31 + 1) / 2 = 0.655 s → frame 19.65 → 20 → 0.6667 s
    assert_eq!(r.key_times(), vec![20.0 / 30.0]);
}

#[test]
fn ac3_labels_name_the_time_and_the_last_event() {
    let clock = Clock::new()
        .event("frena", 1.0)
        .unwrap()
        .event("pasa", 18.1)
        .unwrap();
    let r = reel("ac3label", 30.0, clock);
    assert_eq!(r.label_text(0.5), "0.50 s");
    assert_eq!(r.label_text(18.1), "18.10 s · pasa");
    assert_eq!(r.label_text(20.65), "20.65 s · pasa +2.55");
    // no face, no label; with a face, a band and the text
    assert!(r.label(3.0).is_empty());
    let r = r.with_label_face(4);
    let prims = r.label(3.0);
    assert_eq!(prims.len(), 2);
    assert!(
        matches!(&prims[1], Prim2::Text { text, face: 4, .. } if text == "3.00 s · frena +2.00")
    );
}

#[test]
fn ac3_command_line() {
    let args = |s: &str| s.split_whitespace().map(String::from).collect::<Vec<_>>();
    assert_eq!(
        Command::from_args(&args("--frames 1.2,4")).unwrap(),
        Command::Frames(vec![1.2, 4.0])
    );
    assert_eq!(
        Command::from_args(&args("--sheet")).unwrap(),
        Command::Sheet(None)
    );
    assert_eq!(
        Command::from_args(&args("--sheet 1,3")).unwrap(),
        Command::Sheet(Some(vec![1.0, 3.0]))
    );
    assert_eq!(
        Command::from_args(&args("--events")).unwrap(),
        Command::Events
    );
    assert_eq!(
        Command::from_args(&args("--serial")).unwrap(),
        Command::Video { threads: 1 }
    );
    assert!(
        matches!(Command::from_args(&args("")).unwrap(), Command::Video { threads } if threads >= 1)
    );
    let e = Command::from_args(&args("--frames 1,x")).unwrap_err();
    assert!(e.to_string().contains("«x»"), "{e}");
    assert!(Command::from_args(&args("--frames")).is_err());
}

// ---------- AC4: raster overlays, in order ----------

/// Paints pixel (x, y) with a colour: enough to see order and placement.
struct Dot(usize, usize, [u8; 3]);

impl Overlay for Dot {
    fn compose(&self, p6: &mut [u8]) -> std::io::Result<()> {
        let base = p6.len() - CANVAS.rgb_len();
        let k = base + 3 * (self.1 * CANVAS.width as usize + self.0);
        p6[k..k + 3].copy_from_slice(&self.2);
        Ok(())
    }
}

#[test]
fn ac4_overlays_compose_after_the_raster_in_order() {
    let r = reel("ac4", 1.0, Clock::new());
    let frame = |t: f64| {
        (
            square(t),
            vec![
                Dot(1, 1, [255, 0, 0]),
                Dot(1, 1, [0, 0, 255]),
                Dot(2, 1, [0, 255, 0]),
            ],
        )
    };
    let px = decode(&r.frames_png(&[0.0], &frame).unwrap()[0]);
    let at = |x: usize, y: usize| &px[3 * (y * 64 + x)..3 * (y * 64 + x) + 3];
    assert_eq!(at(1, 1), [0, 0, 255], "the later overlay is on top");
    assert_eq!(at(2, 1), [0, 255, 0]);
    // the overlays reach the MP4 as well: one frame with and one without differ
    let with = r.dir.join("with.mp4");
    let without = r.dir.join("without.mp4");
    r.video(&with, 2, &frame).unwrap();
    r.video(&without, 2, &plain).unwrap();
    assert_ne!(
        std::fs::read(with).unwrap(),
        std::fs::read(without).unwrap()
    );
}

// ---------- AC5: the narration timeline ----------

const VOZ: &str = r#"{"modelo": "en-gb-alan-low", "duracion_voz": 4, "lineas": [
  {"id": "a", "texto": "Uno dos. Tres cuatro.", "subtitulo": "x", "t0": 0.5, "t1": 3.5, "desde": 2,
   "frases": [{"texto": "Uno dos.", "t0": 0.5, "t1": 1.5}, {"texto": "Tres cuatro.", "t0": 2.0, "t1": 3.5}]},
  {"id": "b", "texto": "Cinco, seis.", "subtitulo": "y", "t0": 3.9, "t1": 4.0}]}"#;

#[test]
fn ac5_the_clock_comes_from_the_sentences() {
    let v = Narration::from_json(VOZ).unwrap();
    assert_eq!(v.model.as_deref(), Some("en-gb-alan-low"));
    assert_eq!(v.duration, 4.0);
    assert_eq!(v.span("a").unwrap(), (0.5, 3.5));
    assert_eq!(v.sentence("a", 1).unwrap(), (2.0, 3.5));
    assert_eq!(v.line("a").unwrap().subtitle_start(), 2.0);
    assert!((v.at("a", "Tres").unwrap() - 2.0).abs() < 1e-12);
    assert!((v.after("a", "cuatro.").unwrap() - 3.5).abs() < 1e-12);
    assert!((v.at("a", "dos").unwrap() - (0.5 + 4.0 / 8.0)).abs() < 1e-12);
    // no frases: the whole line is one sentence; a comma weighs 1 + 2.5 letters
    let b = v.line("b").unwrap();
    assert_eq!((b.sentences.len(), b.from), (1, 1));
    let k = (6.0 + 2.5) / (12.0 + 2.5);
    assert!((v.at("b", " seis").unwrap() - (3.9 + 0.1 * k)).abs() < 1e-12);
}

#[test]
fn ac5_failures_name_the_line_or_the_phrase() {
    let v = Narration::from_json(VOZ).unwrap();
    assert!(matches!(v.span("zz"), Err(ReelError::NoSuchLine { id }) if id == "zz"));
    let e = v.at("a", "siete").unwrap_err();
    assert!(
        matches!(&e, ReelError::NoSuchPhrase { id, phrase, .. } if id == "a" && phrase == "siete"),
        "{e}"
    );
    assert!(matches!(
        v.sentence("a", 2),
        Err(ReelError::NoSuchSentence { count: 2, .. })
    ));
}

#[test]
fn ac5_a_timeline_that_does_not_hold_together_fails() {
    let unknown = VOZ.replace("\"desde\": 2", "\"dsede\": 2");
    assert!(Narration::from_json(&unknown)
        .unwrap_err()
        .to_string()
        .contains("dsede"));
    let past = VOZ.replace("\"desde\": 2", "\"desde\": 3");
    assert!(Narration::from_json(&past)
        .unwrap_err()
        .to_string()
        .contains("desde = 3"));
    let overlap = VOZ.replace("\"t0\": 3.9", "\"t0\": 3.4");
    assert!(Narration::from_json(&overlap)
        .unwrap_err()
        .to_string()
        .contains("«b» starts before «a» ends"));
    let path = dir("ac5").join("voz.json");
    std::fs::write(&path, past).unwrap();
    let e = Narration::load(&path).unwrap_err();
    assert!(
        e.to_string().starts_with(&path.display().to_string()),
        "{e}"
    );
}

// ---------- AC6: subtitles ----------

#[test]
fn ac6_units_keep_formulas_whole() {
    let f = |s: &str| Piece::Formula(s.into());
    let t = |s: &str| Piece::Text(s.into());
    assert_eq!(
        units("e₁e₁ = 1, e₂e₂ = 1", is_formula_word),
        vec![vec![f("e₁e₁ = 1"), t(",")], vec![f("e₂e₂ = 1")]]
    );
    assert_eq!(
        units("i = e₁e₂: the plane's square.", is_formula_word),
        vec![
            vec![f("i = e₁e₂"), t(":")],
            vec![t("the")],
            vec![t("plane's")],
            vec![t("square.")]
        ]
    );
    assert_eq!(
        word_count(&units("× (−1) = half a turn.", is_formula_word)),
        3
    );
    assert_eq!(
        word_count(&units("√−1 was called “imaginary”.", is_formula_word)),
        3
    );
}

#[test]
fn ac6_rows_break_where_the_wider_row_is_narrowest() {
    // every unit is 10 px per character, a space is 5 px
    let w = |u: &Vec<Piece>| {
        u.iter()
            .map(|p| match p {
                Piece::Text(s) | Piece::Formula(s) => 10.0 * s.chars().count() as f64,
            })
            .sum::<f64>()
    };
    let s = "aa bbbb cc dd";
    let us = units(s, |_| false);
    assert_eq!(row_width(&us, &w, 5.0), 100.0 + 15.0);
    assert_eq!(
        break_rows(us.clone(), w, 5.0, 120.0, 1, s).unwrap().len(),
        1
    );
    // too wide for one row: the best of the three breaks is after "bbbb" (65 | 45)
    let rows = break_rows(us.clone(), w, 5.0, 100.0, 2, s).unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].len(), 2);
    assert!(
        matches!(break_rows(us.clone(), w, 5.0, 100.0, 1, s), Err(ReelError::Subtitle(m)) if m.contains("one row"))
    );
    assert!(
        matches!(break_rows(us, w, 5.0, 60.0, 2, s), Err(ReelError::Subtitle(m)) if m.contains("two rows"))
    );
}

#[test]
fn ac6_one_subtitle_at_a_time_with_its_fade() {
    let v = Narration::from_json(VOZ).unwrap();
    assert!(
        v.active(1.0, 0.15).is_none(),
        "line a's subtitle enters with its second sentence"
    );
    let (l, a) = v.active(2.075, 0.15).unwrap();
    assert_eq!(l.id, "a");
    assert!((a - (1.0 - 0.5f64.powi(3))).abs() < 1e-12);
    assert_eq!(v.active(3.0, 0.15).unwrap().1, 1.0);
    assert!(v.check_no_overlap(0.15).is_ok());
    let e = v.check_no_overlap(0.5).unwrap_err();
    assert!(
        e.to_string().contains("«b» enters before «a» leaves"),
        "{e}"
    );
}
