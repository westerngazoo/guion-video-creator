//! `guion` — check and render screenplays (R-0003); make and gate narrated reels (R-0012);
//! the accounts dashboard and new posts from a template (R-0013); new accounts (R-0014);
//! the gadget studio (R-0015).

use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

mod check;
mod encode;
mod narrate;
mod path;
mod reel;
mod render;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    match args.get(1).map(String::as_str) {
        None | Some("-h") | Some("--help") | Some("help") => {
            eprintln!(
                "uso:\n\
  guion check <screenplay.toml>\n\
  guion render <screenplay.toml> [--out DIR] [--fps N] [--no-brand]\n\
  guion narrate <screenplay.toml> [--out WAV] [--engine scaffold|piper] [--piper-model PATH] [--with-bed]\n\
  guion encode <screenplay.toml> [--out FILE.mp4] [--frames DIR] [--fps N] [--no-brand] [--narrate]\n\
  guion reel <post> [--twice] [--no-check]\n\
  guion reel check <post> [--brand PROFILE.toml] [--mp4 FILE] [--compare A.mp4 B.mp4] [--json OUT]\n\
  guion reel nuevo <cuenta> <slug> [--titulo \"…\"] [--repo DIR]...\n\
  guion reel sonido <post>\n\
  guion tablero [--repo DIR]... [--todas] [--json] | lote <render|check|voz> [--cuenta X] [--estado Y]\n\
  guion cuenta nueva <dir> --nombre \"…\" --tema \"…\" --revisor NOMBRE --materia \"…\" [...]\n\
  guion cuenta perfil <dir> [--check]\n\
  guion studio <post> [--puerto N] [--cara F.ttf] | --cuadro T [--out F.png] | --check | --video | --sheet"
            );
            ExitCode::SUCCESS
        }
        Some("check") => check::run(args.get(2)),
        Some("render") => render::run(&args[2..]),
        Some("narrate") => narrate::run(&args[2..]),
        Some("encode") => encode::run(&args[2..]),
        Some("reel") => reel::run(&args[2..]),
        Some("tablero") => guion_tablero::run(&args[2..]),
        Some("cuenta") => guion_cuenta::run(&args[2..], &guion_tablero::fecha::hoy(), &al_tablero),
        Some("studio") => guion_estudio::run(&args[2..]),
        Some(cmd) => {
            eprintln!("comando desconocido: {cmd}");
            ExitCode::from(2)
        }
    }
}

/// `guion cuenta nueva --al-tablero`: the new repo joins the user's list.
fn al_tablero(repo: &std::path::Path) -> Result<String, String> {
    let f = guion_tablero::repos::archivo()
        .ok_or("no user list: set GUION_REPOS or HOME".to_string())?;
    Ok(if guion_tablero::repos::agregar(&f, repo)? {
        format!("tablero: agregado a {}", f.display())
    } else {
        format!("tablero: ya estaba en {}", f.display())
    })
}

pub(crate) fn default_out(slug: &str) -> PathBuf {
    PathBuf::from("out").join(slug).join("frames")
}

pub(crate) fn frame_count(duration: f64, fps: f64) -> usize {
    (duration * fps).ceil() as usize
}
