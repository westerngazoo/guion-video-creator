//! `guion reel`: a narrated reel in one command, and its automatic gate (R-0012).
//!
//! ```text
//! guion reel <post> [--twice] [--no-check] [--formato F]
//! guion reel check <post> [--brand PROFILE.toml] [--mp4 FILE] [--compare A.mp4 B.mp4] [--json OUT] [--formato F]
//! guion reel nuevo <cuenta> <slug> [--titulo "…"] [--repo DIR]...   (R-0013, guion-tablero)
//! guion reel sonido <post>                                           (R-0014, guion-marca)
//! ```
//!
//! The pipeline reads the post's `reel.toml` (AC9): the voice if `voz.wav` is
//! missing (AC10), the scene's binary (its MP4 and `eventos.json`), the sound
//! command, the mix to the platform's loudness (guion-encode), the caption
//! and alt text (guion-publish), the cover frame, and the gate. Every step
//! is deterministic, so a second run gives the same bytes; `--twice` renders
//! the scene twice and the gate compares them.
//!
//! The format (R-0016: `reel`, `tiktok`, `short`, `youtube`) comes from
//! `--formato`, `reel.toml` or `escena.toml`. The scene and the sound see it
//! in `GUION_FORMATO`; a YouTube format also writes the title and the
//! description (`youtube.md`), and a long video the `.srt` and the thumbnail.

mod config;
mod voz;
mod youtube;

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::time::Instant;

use guion_encode::{mux_at_loudness, LoudnessTarget};
use guion_reel::gate::{self, Inputs, Profile};
use guion_reel::Formato;

use config::{path, ReelToml};

const USAGE: &str =
    "uso:\n  guion reel <post> [--twice] [--no-check] [--formato reel|tiktok|short|youtube]\n  guion reel voice <post>\n  \
guion reel nuevo <cuenta> <slug> [--titulo \"…\"] [--repo DIR]...\n  guion reel sonido <post>\n  \
guion reel check <post> [--brand PROFILE.toml] [--mp4 FILE] [--compare A.mp4 B.mp4] [--json OUT] [--formato F]";

pub fn run(args: &[String]) -> ExitCode {
    match args.first().map(String::as_str) {
        None | Some("-h") | Some("--help") => {
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
        Some("check") => check_cmd(&args[1..]),
        Some("voice") => voice_cmd(&args[1..]),
        // R-0013: a new post from the account's template (guion-tablero)
        Some("nuevo") => guion_tablero::run_nuevo(&args[1..]),
        // R-0014: the bed with the neutral kit (an account with no instruments of its own)
        Some("sonido") => sonido_cmd(&args[1..]),
        Some(_) => pipeline_cmd(args),
    }
}

fn fail(msg: impl std::fmt::Display) -> ExitCode {
    eprintln!("guion reel: {msg}");
    ExitCode::from(2)
}

// --- the voice alone (rewrites voz.wav and voz.json) ---

fn voice_cmd(args: &[String]) -> ExitCode {
    let [post] = args else { return fail(USAGE) };
    let post = Path::new(post);
    let r = match ReelToml::load(post) {
        Ok(r) => r,
        Err(e) => return fail(e),
    };
    match voice_of(post, &r).and_then(|v| voz::narrate(post, &v)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => fail(e),
    }
}

fn voice_of(post: &Path, r: &ReelToml) -> Result<voz::Voice, String> {
    Ok(voz::Voice {
        voices: path(post, &r.voice.voices)?,
        piper: r
            .voice
            .piper
            .as_deref()
            .map(|p| path(post, p))
            .transpose()?,
        model: Some(r.voice.model.clone()),
    })
}

// --- the sound with the neutral kit (R-0014) ---

fn sonido_cmd(args: &[String]) -> ExitCode {
    let [post] = args else { return fail(USAGE) };
    match guion_marca::sonido::render(Path::new(post)) {
        Ok(o) => {
            println!("{} ({:.2} s)\n{}", o.path.display(), o.duration, o.report);
            ExitCode::SUCCESS
        }
        Err(e) => fail(e),
    }
}

// --- the gate ---

struct CheckArgs {
    post: PathBuf,
    brand: Option<PathBuf>,
    mp4: Option<PathBuf>,
    compare: Option<(PathBuf, PathBuf)>,
    json: Option<PathBuf>,
    formato: Option<Formato>,
}

fn parse_check(args: &[String]) -> Result<CheckArgs, String> {
    let mut post = None;
    let (mut brand, mut mp4, mut compare, mut json) = (None, None, None, None);
    let mut formato = None;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        let mut val = |flag: &str| {
            it.next()
                .map(PathBuf::from)
                .ok_or(format!("{flag} needs a value"))
        };
        match a.as_str() {
            "--brand" => brand = Some(val("--brand")?),
            "--mp4" => mp4 = Some(val("--mp4")?),
            "--json" => json = Some(val("--json")?),
            "--formato" => {
                let f = it.next().ok_or("--formato needs a value")?;
                formato = Some(Formato::de(f)?);
            }
            "--compare" => {
                let a = val("--compare")?;
                let b = it
                    .next()
                    .map(PathBuf::from)
                    .ok_or("--compare needs two files")?;
                compare = Some((a, b));
            }
            f if f.starts_with('-') => return Err(format!("unknown flag {f}\n{USAGE}")),
            p if post.is_none() => post = Some(PathBuf::from(p)),
            p => return Err(format!("unexpected argument {p}")),
        }
    }
    Ok(CheckArgs {
        post: post.ok_or(format!("the post's folder is missing\n{USAGE}"))?,
        brand,
        mp4,
        compare,
        json,
        formato,
    })
}

fn check_cmd(args: &[String]) -> ExitCode {
    let a = match parse_check(args) {
        Ok(a) => a,
        Err(e) => return fail(e),
    };
    let reel = if a.post.join("reel.toml").exists() {
        match ReelToml::load(&a.post) {
            Ok(r) => Some(r),
            Err(e) => return fail(e),
        }
    } else {
        None
    };
    let formato = match youtube::formato(&a.post, reel.as_ref(), a.formato) {
        Ok(f) => f,
        Err(e) => return fail(e),
    };
    let gate = Gate {
        brand: a.brand,
        mp4: a.mp4,
        compare: a.compare,
        json: a.json,
        formato,
    };
    match gate_post(&a.post, reel.as_ref(), gate) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(e) => fail(e),
    }
}

/// What the gate is asked beyond the post.
struct Gate {
    brand: Option<PathBuf>,
    mp4: Option<PathBuf>,
    compare: Option<(PathBuf, PathBuf)>,
    json: Option<PathBuf>,
    formato: Formato,
}

/// Where the texts to paste go: `[publish] dir`, or the post.
fn publish_dir(post: &Path, reel: Option<&ReelToml>) -> Result<PathBuf, String> {
    match reel.and_then(|r| r.publish.as_ref()) {
        Some(p) => path(post, &p.dir),
        None => Ok(post.to_path_buf()),
    }
}

/// Runs the gate on a post, prints the summary, writes the JSON. `Ok(pass)`.
fn gate_post(post: &Path, reel: Option<&ReelToml>, g: Gate) -> Result<bool, String> {
    let Gate {
        brand,
        mp4,
        compare,
        json,
        formato,
    } = g;
    let brand = match (brand, reel) {
        (Some(b), _) => b,
        (None, Some(r)) => path(post, &r.reel.brand)?,
        (None, None) => return Err(format!("{}: no reel.toml, and no --brand", post.display())),
    };
    let profile = Profile::load(&brand).map_err(|e| e.to_string())?;
    let name = match reel {
        Some(r) => r.name(post),
        None => post
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
    };
    let mut inputs = Inputs::in_post(post, &name);
    if formato.de_youtube() {
        inputs = inputs.youtube_in(&publish_dir(post, reel)?, &name);
    }
    inputs.formato = formato;
    if let Some(m) = mp4 {
        inputs.mp4 = m;
    }
    inputs.compare = compare;
    inputs.max_duration = reel.map(|r| r.reel.max_duration);
    let report = gate::run(&inputs, &profile).map_err(|e| e.to_string())?;
    let out = json.unwrap_or_else(|| post.join("check.json"));
    std::fs::write(&out, report.to_json()).map_err(|e| format!("{}: {e}", out.display()))?;
    print!("{}", report.summary());
    println!("report → {}", out.display());
    Ok(report.pass)
}

// --- the pipeline ---

fn pipeline_cmd(args: &[String]) -> ExitCode {
    let mut post = None;
    let (mut twice, mut check) = (false, true);
    let mut formato = None;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--twice" => twice = true,
            "--no-check" => check = false,
            "--formato" => match it.next().map(|f| Formato::de(f)) {
                Some(Ok(f)) => formato = Some(f),
                Some(Err(e)) => return fail(e),
                None => return fail("--formato needs a value"),
            },
            f if f.starts_with('-') => return fail(format!("unknown flag {f}\n{USAGE}")),
            p if post.is_none() => post = Some(PathBuf::from(p)),
            p => return fail(format!("unexpected argument {p}")),
        }
    }
    let Some(post) = post else { return fail(USAGE) };
    let post = match post.canonicalize() {
        Ok(p) => p,
        Err(e) => return fail(format!("{}: {e}", post.display())),
    };
    match pipeline(&post, twice, check, formato) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(e) => fail(e),
    }
}

fn step(name: &str, t: Instant) {
    println!("  {name:<8} {:6.1} s", t.elapsed().as_secs_f64());
}

fn sh(cmd: &mut Command) -> Result<(), String> {
    let shown = format!("{cmd:?}");
    let st = cmd.status().map_err(|e| format!("{shown}: {e}"))?;
    if st.success() {
        Ok(())
    } else {
        Err(format!("{shown} exited with {}", st.code().unwrap_or(-1)))
    }
}

/// The scene's binary: built with cargo, at the manifest's target folder
/// (or `CARGO_TARGET_DIR`).
fn build_scene(post: &Path, r: &ReelToml) -> Result<PathBuf, String> {
    let manifest = path(post, &r.scene.manifest)?;
    let mut cargo = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()));
    cargo
        .args(["build", "--release", "-q", "--manifest-path"])
        .arg(&manifest)
        .args(["--bin", &r.scene.bin]);
    if !r.scene.features.is_empty() {
        cargo.args(["--features", &r.scene.features.join(",")]);
    }
    sh(&mut cargo)?;
    let target = match std::env::var_os("CARGO_TARGET_DIR") {
        Some(t) => PathBuf::from(t),
        None => manifest.parent().unwrap_or(Path::new(".")).join("target"),
    };
    Ok(target.join("release").join(&r.scene.bin))
}

fn pipeline(post: &Path, twice: bool, check: bool, flag: Option<Formato>) -> Result<bool, String> {
    let r = ReelToml::load(post)?;
    let name = r.name(post);
    let formato = youtube::formato(post, Some(&r), flag)?;
    // the scene and the sound render the format they are asked for
    std::env::set_var(guion_reel::formato::VARIABLE, formato.nombre());
    let all = Instant::now();
    println!("guion reel {} ({formato})", post.display());

    // 1. the voice, if missing: it sets the clock
    let t = Instant::now();
    if !post.join("voz.wav").exists() {
        let voice = voz::Voice {
            voices: path(post, &r.voice.voices)?,
            piper: r
                .voice
                .piper
                .as_deref()
                .map(|p| path(post, p))
                .transpose()?,
            model: Some(r.voice.model.clone()),
        };
        voz::narrate(post, &voice)?;
        step("voice", t);
    } else {
        println!("  voice    kept (voz.wav exists)");
    }

    // 2. the scene: <name>.mp4 and eventos.json
    let t = Instant::now();
    let bin = build_scene(post, &r)?;
    sh(Command::new(&bin).current_dir(post))?;
    let mp4 = post.join(format!("{name}.mp4"));
    let mut compare = None;
    if twice {
        let first = post.join(format!("{name}.first.mp4"));
        std::fs::rename(&mp4, &first).map_err(|e| format!("{}: {e}", mp4.display()))?;
        sh(Command::new(&bin).current_dir(post))?;
        compare = Some((first, mp4.clone()));
    }
    step("scene", t);

    // 3. the sound
    let t = Instant::now();
    let post_s = post.display().to_string();
    // `{guion}`: this very binary (R-0014: `guion reel sonido`, the neutral kit)
    let guion_s = std::env::current_exe()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "guion".into());
    let cmd: Vec<String> = r
        .sound
        .command
        .iter()
        .map(|a| a.replace("{post}", &post_s).replace("{guion}", &guion_s))
        .collect();
    let (prog, rest) = cmd.split_first().ok_or("[sound] command is empty")?;
    sh(Command::new(prog).args(rest).current_dir(post))?;
    step("sound", t);

    // 4. the mix: the platform's loudness, measured in AAC
    let t = Instant::now();
    let out = post.join(format!("{name}-audio.mp4"));
    let m = mux_at_loudness(
        &mp4,
        &post.join(&r.sound.output),
        &out,
        LoudnessTarget::REELS,
    )
    .map_err(|e| e.to_string())?;
    println!(
        "  mix      {:6.1} s  {:.1} LUFS, {:.1} dBTP → {}",
        t.elapsed().as_secs_f64(),
        m.integrated_lufs,
        m.true_peak_dbtp,
        out.display()
    );

    // 5. the caption and the alt text (a reel, a TikTok; any format that has one)
    let dir = publish_dir(post, Some(&r))?;
    if formato.con_caption() || post.join("caption.md").exists() {
        let ex = guion_publish::export_file(&post.join("caption.md")).map_err(|e| e.to_string())?;
        let (c, a) = ex.write_to(&dir).map_err(|e| e.to_string())?;
        println!("  caption  {} + {}", c.display(), a.display());
    }

    // 5b. YouTube: the title and the description; a long video, the .srt and the thumbnail
    if formato.de_youtube() {
        let (t, d) = youtube::textos(post, &r, &dir)?;
        println!("  youtube  {} + {}", t.display(), d.display());
    }
    if formato == Formato::Youtube {
        let srt = youtube::subtitulos(post, &name, &dir)?;
        println!("  srt      {}", srt.display());
        match r.youtube.as_ref().and_then(|y| y.miniatura) {
            Some(t) => {
                sh(Command::new(&bin)
                    .args([&r.scene.miniatura_flag, &format!("{t}")])
                    .current_dir(post)
                    .stdout(std::process::Stdio::null()))?;
                let png = post.join("miniatura.png");
                if !png.is_file() {
                    return Err(format!(
                        "{} {} {t} wrote no {}: does the scene read guion-reel's Command?",
                        bin.display(),
                        r.scene.miniatura_flag,
                        png.display()
                    ));
                }
                if dir != post {
                    std::fs::copy(&png, dir.join("miniatura.png"))
                        .map_err(|e| format!("{}: {e}", dir.display()))?;
                }
                println!("  thumb    {t} s → {}", dir.join("miniatura.png").display());
            }
            None => println!("  thumb    none ([youtube] miniatura = <s> in reel.toml)"),
        }
    }

    // 6. the cover frame
    if let Some(cover) = r.reel.cover {
        sh(Command::new(&bin)
            .args([&r.scene.frames_flag, &format!("{cover}")])
            .current_dir(post)
            .stdout(std::process::Stdio::null()))?;
        println!("  cover    {cover} s ({} {cover})", r.scene.frames_flag);
    }
    println!("  total    {:6.1} s", all.elapsed().as_secs_f64());

    // 7. the gate
    let pass = if check {
        gate_post(
            post,
            Some(&r),
            Gate {
                brand: None,
                mp4: None,
                compare: compare.clone(),
                json: None,
                formato,
            },
        )?
    } else {
        true
    };
    if let Some((first, _)) = compare {
        let _ = std::fs::remove_file(first);
    }
    Ok(pass)
}
