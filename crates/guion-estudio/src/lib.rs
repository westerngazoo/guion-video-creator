//! `guion-estudio`: `guion studio <post>` (R-0015 AC6).
//!
//! A local studio over localhost: the page lists the post's gadgets with a
//! slider per param and its provenance tag, a scrub bar, and the frame at
//! that instant, rendered by guion-reel exactly as the MP4 will have it.
//! «guardar» writes `escena.toml`, the post's source of truth. No gdk, no
//! window toolkit: it builds and runs in CI and in the cloud, and the
//! browser is the window.
//!
//! - [`plantilla`]: the account's template (canvas, palette by role, faces,
//!   safe area) from the `cuenta.toml` above the post;
//! - [`estudio`]: the post open in memory, what the page asks of it;
//! - [`http`]: the server and its router (std only).
//!
//! An account with its own gadgets serves the same studio from its motor
//! crate: a binary whose `main` is `guion_estudio::run_con(registro, args)`
//! with [`guion_gadget::Registro::integrados`] plus its gadgets.

pub mod estudio;
pub mod http;
pub mod plantilla;

use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

pub use estudio::Estudio;
use guion_gadget::Registro;
use guion_reel::Command;

/// The command's usage.
pub const USO: &str = "\
uso:
  guion studio <post> [--puerto N] [--cara FUENTE.ttf]   el studio en http://127.0.0.1:N (7878; 0: uno libre)
  guion studio <post> --cuadro T [--r K] [--out F.png]   un cuadro como PNG (reducido K veces)
  guion studio <post> --check                            la escena y sus avisos, cuadro por cuadro
  guion studio <post> --video | --sheet [T,…] | --frames T,… | --events | --miniatura T
                                                         el render de guion-reel desde escena.toml
                                                         (--miniatura: la miniatura de YouTube, 1280×720, en 16:9)";

fn valor<'a>(args: &'a [String], flag: &str) -> Option<&'a str> {
    args.iter()
        .position(|a| a == flag)
        .and_then(|i| args.get(i + 1))
        .map(String::as_str)
}

/// `guion studio …` with the built-in gadgets.
pub fn run(args: &[String]) -> ExitCode {
    run_con(Registro::integrados(), args)
}

/// `guion studio …` with `registro`: what an account's own studio binary
/// calls, with its gadgets registered next to the built-ins.
pub fn run_con(registro: Registro, args: &[String]) -> ExitCode {
    match ejecutar(registro, args) {
        Ok(()) => ExitCode::SUCCESS,
        Err((code, m)) => {
            eprintln!("guion studio: {m}");
            ExitCode::from(code)
        }
    }
}

fn ejecutar(registro: Registro, args: &[String]) -> Result<(), (u8, String)> {
    let Some(post) = args.first().filter(|a| !a.starts_with("--")) else {
        return Err((2, USO.to_string()));
    };
    let cara = valor(args, "--cara").map(PathBuf::from);
    let mut e = Estudio::abrir(Path::new(post), registro, cara.as_deref()).map_err(|m| (1, m))?;
    for n in &e.plantilla.notas {
        eprintln!("guion studio: {n}");
    }
    if let Some(t) = valor(args, "--cuadro") {
        let t: f64 = t
            .parse()
            .map_err(|_| (2, format!("--cuadro «{t}»: seconds")))?;
        let r: usize = valor(args, "--r")
            .map(str::parse)
            .transpose()
            .map_err(|_| (2, "--r: a reduction from 1 to 8".to_string()))?
            .unwrap_or(1);
        let out = valor(args, "--out")
            .map(PathBuf::from)
            .unwrap_or_else(|| e.post.join(format!("cuadro-{t:05.2}.png")));
        let png = e.cuadro_png(t, r).map_err(|m| (1, m))?;
        std::fs::write(&out, png).map_err(|x| (1, format!("{}: {x}", out.display())))?;
        println!("{}", out.display());
        return Ok(());
    }
    if args.iter().any(|a| a == "--check") {
        let avisos = e.comprobar().map_err(|m| (1, m))?;
        for a in &avisos {
            println!("{a}");
        }
        return if avisos.is_empty() {
            println!("escena: {} gadgets, sin avisos", e.escena.gadgets.len());
            Ok(())
        } else {
            Err((1, format!("{} avisos", avisos.len())))
        };
    }
    if [
        "--video",
        "--sheet",
        "--frames",
        "--events",
        "--serial",
        "--miniatura",
    ]
    .iter()
    .any(|f| args.iter().any(|a| a == f))
    {
        let c = Command::from_args(args).map_err(|x| (2, x.to_string()))?;
        for f in e.render(&c).map_err(|m| (1, m))? {
            println!("{}", f.display());
        }
        return Ok(());
    }
    let puerto: u16 = valor(args, "--puerto")
        .map(str::parse)
        .transpose()
        .map_err(|_| (2, "--puerto: a port number".to_string()))?
        .unwrap_or(7878);
    let l = TcpListener::bind(("127.0.0.1", puerto))
        .map_err(|x| (1, format!("127.0.0.1:{puerto}: {x}")))?;
    let dir = l.local_addr().map_err(|x| (1, x.to_string()))?;
    println!(
        "guion studio: {} ({}) en http://{dir}/  (Ctrl-C para salir)",
        e.post.display(),
        e.plantilla.nombre
    );
    http::servir(&mut e, &l);
    Ok(())
}
