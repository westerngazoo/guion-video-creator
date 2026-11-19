//! `guion-profesor <clase.toml> <out-dir>` — teacher mode: render a class
//! screenplay to `<slug>.mp4` (picture + the teacher's voice), a poster and
//! `<slug>.json`, the sidecar the classroom player reads (chapters,
//! transcript, question stops and the board as a formula sheet).

mod clase;
mod dibujo;
mod fisica;
mod linea;
mod voz;

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use motoreel::{FrameSink, PpmSink};
use motoreel_typeset::{Face, Fonts};
use serde::Serialize;

const SIZE: (u32, u32) = (1280, 720);
const FONT: &str = "/System/Library/Fonts/Supplemental/Arial Unicode.ttf";
const FONT_BOLD: &str = "/System/Library/Fonts/Supplemental/Arial Bold.ttf";

#[derive(Serialize)]
struct Sidecar<'a> {
    slug: &'a str,
    title: &'a str,
    kicker: &'a str,
    guide: &'a str,
    /// The narration's language (BCP 47), for the player's captions track.
    lang: &'a str,
    duration: f64,
    video: String,
    poster: String,
    chapters: &'a [linea::Capitulo],
    cues: &'a [linea::Cue],
    stops: Vec<linea::Parada>,
    /// The board, chapter by chapter: the class's formula sheet.
    board: Vec<Hoja>,
}

#[derive(Serialize)]
struct Hoja {
    chapter: String,
    lines: Vec<String>,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("usage: guion-profesor <clase.toml> <out-dir>");
        std::process::exit(2);
    }
    if let Err(e) = run(Path::new(&args[1]), Path::new(&args[2])) {
        eprintln!("guion-profesor: {e}");
        std::process::exit(1);
    }
}

fn fonts() -> Result<Fonts, Box<dyn std::error::Error>> {
    let mut f = Fonts::new();
    f.add(Face::load(fs::read(FONT)?, "Arial Unicode")?);
    f.add(Face::load(fs::read(FONT_BOLD)?, "Arial Bold")?);
    Ok(f)
}

fn run(src: &Path, out: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let clase = clase::load(src)?;
    fs::create_dir_all(out)?;
    let slug = clase.class.slug.clone();

    // the voice sets the clock
    let cache = out.join(".voz");
    let n = &clase.narration;
    let clips: Vec<voz::Clip> = clase
        .beat
        .iter()
        .map(|b| voz::decir(&b.say, &n.voice, n.speed, &cache))
        .collect::<Result<_, _>>()?;
    let habla: Vec<f64> = clips.iter().map(voz::Clip::segundos).collect();
    let plan = linea::plan(&clase, &habla);
    let cues = plan.cues(&clase);
    let placed: Vec<(f64, &voz::Clip)> = plan.tramos.iter().map(|t| t.t0).zip(&clips).collect();
    let track = voz::mezclar(&placed, plan.duracion);
    let wav = out.join(format!("{slug}.wav"));
    voz::escribir_wav(&wav, &track)?;

    // picture: frames straight into ffmpeg, one scratch PPM at a time
    let mp4 = out.join(format!("{slug}.mp4"));
    let scratch: PathBuf = out.join(".frame");
    let mut sink = PpmSink::with_view(&scratch, SIZE, dibujo::VIEW)?
        .with_background(dibujo::background())
        .with_fonts(fonts()?);
    let fps = clase.class.fps;
    let mut ff = Command::new("ffmpeg")
        .args([
            "-y",
            "-loglevel",
            "error",
            "-f",
            "image2pipe",
            "-c:v",
            "ppm",
            "-framerate",
        ])
        .arg(format!("{fps}"))
        .args(["-i", "-", "-i"])
        .arg(&wav)
        .args([
            "-c:v", "libx264", "-preset", "veryfast", "-crf", "22", "-pix_fmt", "yuv420p",
        ])
        .args([
            "-c:a",
            "aac",
            "-b:a",
            "128k",
            "-shortest",
            "-movflags",
            "+faststart",
        ])
        .arg(&mp4)
        .stdin(Stdio::piped())
        .spawn()?;
    let mut pipe = ff.stdin.take().ok_or("ffmpeg: no stdin")?;
    let frames = (plan.duracion * fps).ceil() as usize;
    let frame_file = scratch.join("frame_00000.ppm");
    for k in 0..frames {
        let t = k as f64 / fps;
        sink.frame(0, &dibujo::frame(&clase, &plan, &cues, &track, t))?;
        pipe.write_all(&fs::read(&frame_file)?)?;
        if k % (fps as usize * 10) == 0 {
            eprintln!("  {t:>5.0} s / {:.0} s", plan.duracion);
        }
    }
    drop(pipe);
    if !ff.wait()?.success() {
        return Err("ffmpeg failed".into());
    }
    let _ = fs::remove_dir_all(&scratch);

    // poster: the first demonstration under way
    let poster = out.join(format!("{slug}.jpg"));
    let at = plan
        .tramos
        .iter()
        .zip(&clase.beat)
        .find(|(_, b)| b.demo.is_some())
        .map(|(t, _)| t.t0 + linea::DEMO_LEAD + 1.0)
        .unwrap_or(1.0);
    let st = Command::new("ffmpeg")
        .args(["-y", "-loglevel", "error", "-ss", &format!("{at:.2}"), "-i"])
        .arg(&mp4)
        .args(["-frames:v", "1", "-q:v", "3"])
        .arg(&poster)
        .status()?;
    if !st.success() {
        return Err("ffmpeg poster failed".into());
    }

    let mut board: Vec<Hoja> = plan
        .capitulos
        .iter()
        .map(|c| Hoja {
            chapter: c.title.clone(),
            lines: vec![],
        })
        .collect();
    for (b, t) in clase.beat.iter().zip(&plan.tramos) {
        board[t.capitulo].lines.extend(b.pizarra.iter().cloned());
    }
    board.retain(|h| !h.lines.is_empty());
    let side = Sidecar {
        slug: &slug,
        title: &clase.class.title,
        kicker: &clase.class.kicker,
        guide: &clase.class.guide,
        lang: &clase.class.lang,
        duration: linea::round3(plan.duracion),
        video: format!("{slug}.mp4"),
        poster: format!("{slug}.jpg"),
        chapters: &plan.capitulos,
        cues: &cues,
        stops: plan.paradas(&clase),
        board,
    };
    fs::write(
        out.join(format!("{slug}.json")),
        serde_json::to_string_pretty(&side)?,
    )?;
    let _ = fs::remove_file(&wav);
    eprintln!(
        "✓ {} ({:.1} s, {} capítulos)",
        mp4.display(),
        plan.duracion,
        plan.capitulos.len()
    );
    Ok(())
}
