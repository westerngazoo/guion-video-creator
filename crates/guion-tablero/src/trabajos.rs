//! The jobs: commands running in the background, a few at a time, their
//! output streamed line by line (R-0013 AC5). The TUI drains the channel on
//! every tick, so it never waits on a command; `lote` drains it until the
//! last job ends.

use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Read};
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

use crate::acciones::{Accion, Comando};

/// One command on one post.
#[derive(Debug, Clone, PartialEq)]
pub struct Trabajo {
    pub accion: Accion,
    /// The post's name, for the log.
    pub post: String,
    /// The post's folder (the TUI marks it as running).
    pub dir: PathBuf,
    pub comando: Comando,
}

/// What the jobs report.
#[derive(Debug, Clone, PartialEq)]
pub enum Evento {
    Empieza {
        dir: PathBuf,
        post: String,
        accion: Accion,
        comando: String,
    },
    Linea {
        post: String,
        texto: String,
    },
    Fin {
        dir: PathBuf,
        post: String,
        accion: Accion,
        ok: bool,
        codigo: Option<i32>,
    },
}

/// Runs `trabajos` with at most `paralelo` at once, in the background, and
/// returns at once. Every job sends `Empieza`, its lines and `Fin`.
pub fn lanzar(trabajos: Vec<Trabajo>, paralelo: usize, tx: Sender<Evento>) {
    let n = trabajos.len();
    let cola = Arc::new(Mutex::new(trabajos.into_iter().collect::<VecDeque<_>>()));
    for _ in 0..paralelo.max(1).min(n) {
        let cola = Arc::clone(&cola);
        let tx = tx.clone();
        std::thread::spawn(move || loop {
            let siguiente = cola.lock().ok().and_then(|mut c| c.pop_front());
            let Some(t) = siguiente else { break };
            correr(t, &tx);
        });
    }
}

fn leer_lineas<R: Read + Send + 'static>(
    r: R,
    post: String,
    tx: Sender<Evento>,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        for l in BufReader::new(r).lines().map_while(Result::ok) {
            // the pipeline redraws progress with \r: keep the last state
            let texto = l.rsplit('\r').next().unwrap_or("").to_string();
            let _ = tx.send(Evento::Linea {
                post: post.clone(),
                texto,
            });
        }
    })
}

fn correr(t: Trabajo, tx: &Sender<Evento>) {
    let _ = tx.send(Evento::Empieza {
        dir: t.dir.clone(),
        post: t.post.clone(),
        accion: t.accion,
        comando: t.comando.mostrar(),
    });
    let mut cmd = t.comando.command();
    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let (ok, codigo) = match cmd.spawn() {
        Ok(mut hijo) => {
            let lectores: Vec<_> = [
                hijo.stdout
                    .take()
                    .map(|o| leer_lineas(o, t.post.clone(), tx.clone())),
                hijo.stderr
                    .take()
                    .map(|e| leer_lineas(e, t.post.clone(), tx.clone())),
            ]
            .into_iter()
            .flatten()
            .collect();
            let st = hijo.wait();
            for l in lectores {
                let _ = l.join();
            }
            match st {
                Ok(s) => (s.success(), s.code()),
                Err(e) => {
                    let _ = tx.send(Evento::Linea {
                        post: t.post.clone(),
                        texto: e.to_string(),
                    });
                    (false, None)
                }
            }
        }
        Err(e) => {
            let _ = tx.send(Evento::Linea {
                post: t.post.clone(),
                texto: format!("{}: {e}", t.comando.programa.display()),
            });
            (false, None)
        }
    };
    let _ = tx.send(Evento::Fin {
        dir: t.dir,
        post: t.post,
        accion: t.accion,
        ok,
        codigo,
    });
}
