//! `guion` — check and render screenplays (R-0003).

use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

mod check;
mod encode;
mod narrate;
mod path;
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
  guion encode <screenplay.toml> [--out FILE.mp4] [--frames DIR] [--fps N] [--no-brand] [--narrate]"
            );
            ExitCode::SUCCESS
        }
        Some("check") => check::run(args.get(2)),
        Some("render") => render::run(&args[2..]),
        Some("narrate") => narrate::run(&args[2..]),
        Some("encode") => encode::run(&args[2..]),
        Some(cmd) => {
            eprintln!("comando desconocido: {cmd}");
            ExitCode::from(2)
        }
    }
}

pub(crate) fn default_out(slug: &str) -> PathBuf {
    PathBuf::from("out").join(slug).join("frames")
}

pub(crate) fn frame_count(duration: f64, fps: f64) -> usize {
    (duration * fps).ceil() as usize
}

