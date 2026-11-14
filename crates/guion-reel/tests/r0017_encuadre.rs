//! R-0017 — the cover framing (`encuadre`): where the content of a vertical
//! reel's cover sits against Instagram's grid and feed crops and the Reels
//! UI, one test (or more) per acceptance criterion.
//!
//! The end-to-end test (AC9) encodes a small synthetic reel (ffmpeg with
//! libx264, in PATH or `GUION_FFMPEG`) and measures subtitles in DejaVu
//! Sans: skipped, saying so, where it is not installed.

use std::path::{Path, PathBuf};

use guion_encode::{mux_at_loudness, Encoder, LoudnessTarget, VideoSpec};
use guion_reel::formato::{Encuadre, ENCUADRE_VERTICAL};
use guion_reel::gate::encuadre::{self, Rows};
use guion_reel::gate::pixels::set;
use guion_reel::gate::{self, Inputs, Profile};
use guion_reel::Formato;

const FONT: &str = "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf";

/// A brand at Instagram's size: tokens 0 = background, 1 = bone, 2 = red
/// (the classifier's order is the tokens' names, sorted); a watermark
/// exempt at the bottom right; the subtitle band 1460–1545.
fn profile_text(canvas: (u32, u32), font: &str, extra: &str) -> String {
    format!(
        r#"
[brand]
name = "Test"
[canvas]
width = {}
height = {}
fps = 30.0
[tokens]
a_background = [10, 10, 11]
b_bone = [244, 242, 236]
c_red = [224, 50, 42]
[roles]
background = ["a_background"]
red = ["c_red"]
[subtitle]
font = "{font}"
size = 12.0
size_unit = "em"
max_width = 150.0
center_x = 108.0
baseline = 300.0
band = [{}, {}]
max_words = 4
long_line = 6.0
fade = 0.15
[safe]
x = [18.0, {}]
y = [40.0, {}]
except = [[{}, {}, {}, {}]]
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
units = ["%"]
internal = ["R-####"]
{extra}
"#,
        canvas.0,
        canvas.1,
        canvas.1 as f64 * 1460.0 / 1920.0,
        canvas.1 as f64 * 1545.0 / 1920.0,
        canvas.0 as f64 - 18.0,
        canvas.1 as f64 * 1545.0 / 1920.0,
        canvas.0 as f64 * 0.8,
        canvas.1 as f64 * 0.9,
        canvas.0 as f64 * 0.98,
        canvas.1 as f64 * 0.99,
    )
}

fn ig(extra: &str) -> Profile {
    Profile::parse(&profile_text((1080, 1920), "f.ttf", extra)).expect("parses")
}

const W: usize = 1080;
const H: usize = 1920;

/// A classified 1080 × 1920 frame: background, with bone over `[x0, y0, x1, y1)`.
fn frame(boxes: &[[usize; 4]]) -> Vec<u8> {
    let mut c = vec![0u8; W * H];
    for b in boxes {
        for y in b[1]..b[3] {
            c[y * W + b[0]..y * W + b[2]].fill(1);
        }
    }
    c
}

fn ink() -> [bool; 256] {
    set(&[1, 2])
}

fn rows(boxes: &[[usize; 4]], p: &Profile) -> Option<Rows> {
    encuadre::measure(&frame(boxes), W, H, &ink(), p)
}

fn checked(top: usize, bottom: usize) -> gate::Finding {
    let p = ig("");
    let r = rows(&[[200, top, 880, bottom]], &p);
    encuadre::check(r, &p.encuadre(Formato::Reel).unwrap(), "0.00 s (f0)")
}

#[test]
fn ac1_vertical_formats_default_to_instagrams_crops() {
    assert_eq!(
        ENCUADRE_VERTICAL,
        Encuadre {
            min_top: 300.0,
            max_bottom: 1545.0,
            center_y: 960.0,
            max_offset: 60.0
        }
    );
    for f in [Formato::Reel, Formato::Tiktok, Formato::Short] {
        let b = f.bandas();
        assert_eq!(b.encuadre, Some(ENCUADRE_VERTICAL), "{f}");
        // the safe area's top is the feed crop's, as the framing's
        assert_eq!(b.safe_y, [300.0, 1545.0], "{f}");
    }
    assert_eq!(Formato::Youtube.bandas().encuadre, None);
    // a brand at 1080 × 1920 that writes nothing gets the defaults as they are
    assert_eq!(ig("").encuadre(Formato::Reel), Some(ENCUADRE_VERTICAL));
}

#[test]
fn ac2_the_box_is_the_covers_ink_without_subtitle_watermark_or_strays() {
    let p = ig("");
    let title = [200, 700, 880, 1220];
    assert_eq!(
        rows(&[title], &p),
        Some(Rows {
            top: 700,
            bottom: 1220
        })
    );
    // the subtitle band (1460–1545), the watermark in `except`, and fewer
    // than `min_stray` (10) stray pixels at the top do not move the box
    let subtitle = [300, 1470, 780, 1530];
    let watermark = [880, 1740, 1050, 1890];
    let strays = [10, 40, 13, 42]; // 6 px
    assert_eq!(
        rows(&[title, subtitle, watermark, strays], &p),
        Some(Rows {
            top: 700,
            bottom: 1220
        })
    );
    // the background is not content; nothing at all is nothing to frame
    assert_eq!(rows(&[], &p), None);
    let f = encuadre::check(None, &ENCUADRE_VERTICAL, "1.00 s (f30)");
    assert!(
        !f.pass && f.detail.contains("nothing on the cover"),
        "{f:?}"
    );
}

#[test]
fn ac3_a_centred_cover_passes() {
    let f = checked(700, 1220);
    assert!(f.pass, "{f:?}");
    assert_eq!(f.check, "encuadre");
    assert!(f.detail.contains("content y 700–1220, centre 960"), "{f:?}");
}

#[test]
fn ac4_content_above_the_feed_crop_fails_naming_top_and_limit() {
    // (content this high is also off-centre: the subtitle band is not content)
    let f = checked(290, 1450);
    assert!(!f.pass);
    assert!(f.at[0].contains("top y 290 < 300"), "{f:?}");
    assert!(f.detail.contains("content y 290–1450, centre 870"), "{f:?}");
    assert!(f.detail.contains("top ≥ 300"), "{f:?}");
}

#[test]
fn ac5_content_under_the_reels_ui_fails_naming_bottom_and_limit() {
    let f = checked(400, 1560);
    assert!(!f.pass);
    assert_eq!(f.at.len(), 1, "only the bottom: {f:?}");
    assert!(f.at[0].contains("bottom y 1560 > 1545"), "{f:?}");
}

#[test]
fn ac6_an_off_centre_box_fails_naming_centre_and_limit() {
    let f = checked(320, 1400);
    assert!(!f.pass);
    assert_eq!(f.at.len(), 1, "only the centre: {f:?}");
    assert!(
        f.at[0].contains("centre y 860 is -100 from 960 (> ± 60"),
        "{f:?}"
    );
}

#[test]
fn ac7_an_account_overrides_the_thresholds_in_its_profile() {
    // its own format: [encuadre]; one format: [formato.<name>] encuadre
    let p = ig(
        "[encuadre]\nmin_top = 250.0\ncenter_y = 880.0\n[formato.short]\nencuadre = { max_offset = 5.0 }\n",
    );
    let reel = p.encuadre(Formato::Reel).unwrap();
    assert_eq!(
        (reel.min_top, reel.center_y, reel.max_offset),
        (250.0, 880.0, 60.0)
    );
    let short = p.encuadre(Formato::Short).unwrap();
    assert_eq!(
        (short.min_top, short.center_y, short.max_offset),
        (250.0, 880.0, 5.0)
    );
    // the box that fails the defaults (AC4) passes the account's reel…
    let r = rows(&[[200, 290, 880, 1450]], &p);
    assert!(!encuadre::check(r, &ENCUADRE_VERTICAL, "f0").pass);
    assert!(encuadre::check(r, &reel, "f0").pass);
    // …and fails its Short, centre 870 against 880 ± 5
    let f = encuadre::check(r, &short, "f0");
    assert!(
        !f.pass && f.at[0].contains("is -10 from 880 (> ± 5"),
        "{f:?}"
    );

    // a smaller vertical canvas scales the defaults (Instagram's crops are
    // proportional to the frame)
    let small = Profile::parse(&profile_text((216, 384), "f.ttf", "")).unwrap();
    let e = small.encuadre(Formato::Reel).unwrap();
    assert_eq!(
        (e.min_top, e.max_bottom, e.center_y, e.max_offset),
        (60.0, 309.0, 192.0, 12.0)
    );

    // unknown keys, horizontal formats and impossible limits fail by name
    let typo = Profile::parse(&profile_text(
        (1080, 1920),
        "f.ttf",
        "[encuadre]\nmax_ofset = 1.0\n",
    ));
    assert!(typo.unwrap_err().to_string().contains("max_ofset"));
    let yt = Profile::parse(&profile_text(
        (1080, 1920),
        "f.ttf",
        "[formato.youtube]\nencuadre = { min_top = 10.0 }\n",
    ))
    .unwrap_err()
    .to_string();
    assert!(
        yt.contains("formato.youtube.encuadre") && yt.contains("vertical"),
        "{yt}"
    );
    let flipped = Profile::parse(&profile_text(
        (1080, 1920),
        "f.ttf",
        "[encuadre]\nmin_top = 1600.0\n",
    ))
    .unwrap_err()
    .to_string();
    assert!(
        flipped.contains("min_top 1600") && flipped.contains("reel"),
        "{flipped}"
    );
    let wide = Profile::parse(&profile_text(
        (1920, 1080),
        "f.ttf",
        "[encuadre]\nmin_top = 10.0\n",
    ))
    .unwrap_err()
    .to_string();
    assert!(
        wide.contains("[encuadre]") && wide.contains("horizontal"),
        "{wide}"
    );
}

#[test]
fn ac8_a_horizontal_format_is_not_checked() {
    assert_eq!(ig("").encuadre(Formato::Youtube), None);
    let f = encuadre::skipped(Formato::Youtube);
    assert!(f.pass && f.check == "encuadre" && f.detail.contains("not checked"));
}

// --- AC9: the gate measures the cover frame of the MP4 ---

const SW: usize = 216;
const SH: usize = 384;
const FPS: f64 = 30.0;

fn dir(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("guion-r17-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn rect(rgb: &mut [u8], x0: usize, y0: usize, x1: usize, y1: usize, c: [u8; 3]) {
    for y in y0..y1 {
        for x in x0..x1 {
            rgb[(y * SW + x) * 3..(y * SW + x) * 3 + 3].copy_from_slice(&c);
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

/// A 2 s, 216 × 384 reel (the defaults scale to top ≥ 60, bottom ≤ 309,
/// centre 192 ± 12): until 0.5 s a title high on the frame (y 64–110,
/// off-centre); from 0.5 s a block centred on y 192 (150–234). A
/// watermark in `except` and a subtitle in its band all along.
fn synthetic(d: &Path) -> Inputs {
    let n = (2.0 * FPS) as usize;
    let mp4 = d.join("x.mp4");
    let mut enc = Encoder::start(
        &mp4,
        VideoSpec {
            width: SW as u32,
            height: SH as u32,
            fps: FPS,
        },
    )
    .unwrap();
    let bone = [244, 242, 236];
    for i in 0..n {
        let t = i as f64 / FPS;
        let mut f = vec![0u8; SW * SH * 3];
        rect(&mut f, 0, 0, SW, SH, [10, 10, 11]);
        if t < 0.5 {
            rect(&mut f, 40, 64, 176, 110, bone);
        } else {
            rect(&mut f, 40, 150, 176, 234, bone);
        }
        rect(&mut f, 178, 348, 210, 378, bone); // the watermark, exempt
        rect(&mut f, 60, 294, 156, 306, bone); // in the subtitle band (292–309)
        enc.push(&f).unwrap();
    }
    enc.finish().unwrap();
    let sr = 48_000.0;
    let s: Vec<i16> = (0..(2.0 * sr) as usize)
        .map(|k| {
            let t = k as f64 / sr;
            let a = if (0.2..1.5).contains(&t) { 0.4 } else { 0.004 };
            (a * (2.0 * std::f64::consts::PI * 220.0 * t).sin() * 32767.0) as i16
        })
        .collect();
    wav(&d.join("a.wav"), &s);
    mux_at_loudness(
        &mp4,
        &d.join("a.wav"),
        &d.join("x-audio.mp4"),
        LoudnessTarget::REELS,
    )
    .unwrap();
    std::fs::write(
        d.join("voz.json"),
        r#"{"duracion_voz": 1.5, "lineas": [
 {"id": "a", "texto": "One.", "subtitulo": "Short one.", "t0": 0.2, "t1": 1.5, "frases": [{"texto": "One.", "t0": 0.2, "t1": 1.5}]}]}"#,
    )
    .unwrap();
    std::fs::write(
        d.join("eventos.json"),
        r#"{"duracion": 2.0, "fps": 30.0, "eventos": {}, "series": {}}"#,
    )
    .unwrap();
    std::fs::write(
        d.join("caption.md"),
        "# x\n\n## Caption\n\nUn bloque.\n\n## Hashtags\n\n#f1\n\n## Alt text\n\nUn bloque.\n",
    )
    .unwrap();
    let mut i = Inputs::in_post(d, "x");
    i.max_duration = Some(60.0);
    i
}

fn framing(r: &gate::Report) -> &gate::Finding {
    r.findings
        .iter()
        .find(|f| f.check == "encuadre")
        .expect("an encuadre finding")
}

#[test]
fn ac9_the_gate_checks_the_cover_frame_of_the_mp4() {
    if !Path::new(FONT).exists() {
        eprintln!("skipped: {FONT} is not installed");
        return;
    }
    let d = dir("e2e");
    let mut inputs = synthetic(&d);
    let p = Profile::parse(&profile_text((SW as u32, SH as u32), FONT, "")).unwrap();

    // no cover: the first frame, whose title is off-centre
    let r = gate::run(&inputs, &p).expect("the gate runs");
    let f = framing(&r);
    assert!(!f.pass, "{f:?}");
    assert!(
        f.detail.contains("f0") && f.detail.contains("content y 64–110"),
        "{f:?}"
    );
    assert!(!r.pass);

    // the cover at 1.0 s: the centred block; watermark and subtitle ignored
    inputs.cover = Some(1.0);
    let r = gate::run(&inputs, &p).expect("the gate runs");
    let f = framing(&r);
    assert!(f.pass, "{f:?}");
    assert!(
        f.detail.contains("f30") && f.detail.contains("content y 150–234"),
        "{f:?}"
    );

    // a cover past the end fails, naming it
    inputs.cover = Some(5.0);
    let r = gate::run(&inputs, &p).expect("the gate runs");
    let f = framing(&r);
    assert!(
        !f.pass && f.detail.contains("past the video's last frame"),
        "{f:?}"
    );

    // the same post checked as a long YouTube video would be skipped: the
    // format decides, not the post (AC8's end of it is in r0016's AC7)
    assert_eq!(p.encuadre(Formato::Youtube), None);
}
