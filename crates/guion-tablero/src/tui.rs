//! The terminal loop: draw, poll keys for 100 ms, drain the jobs' channel,
//! re-read the state in the background when a job ends (R-0013 AC5: the UI
//! never waits on a command).

use std::path::PathBuf;
use std::process::Stdio;
use std::sync::mpsc;
use std::time::Duration;

use ratatui::crossterm::event::{self, Event, KeyEventKind};

use crate::tablero::{duracion_ffmpeg, Tablero};
use crate::trabajos::{self, Evento};
use crate::ui::{self, App, Efecto};

/// Runs the dashboard until `q`.
pub fn correr(workspaces: Vec<PathBuf>, tablero: Tablero, paralelo: usize) -> Result<(), String> {
    let mut app = App::new(tablero, crate::acciones::guion_bin(), crate::fecha::hoy());
    app.registrar(format!(
        "{} cuentas, {} posts · paralelo {paralelo}",
        app.tablero.cuentas.len(),
        app.tablero
            .cuentas
            .iter()
            .map(|c| c.posts.len())
            .sum::<usize>()
    ));
    let (tx, rx) = mpsc::channel::<Evento>();
    let (tx_t, rx_t) = mpsc::channel::<Result<Tablero, String>>();
    let mut leyendo = false;
    let mut pendiente = false;
    let mut terminal = ratatui::init();
    let r = (|| -> std::io::Result<()> {
        while !app.salir {
            terminal.draw(|f| ui::dibujar(f, &app))?;
            let mut releer = false;
            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(k) = event::read()? {
                    if k.kind == KeyEventKind::Press {
                        for ef in app.tecla(k) {
                            match ef {
                                Efecto::Lanzar(t) => trabajos::lanzar(t, paralelo, tx.clone()),
                                Efecto::Abrir(c) => {
                                    let r = c
                                        .command()
                                        .stdin(Stdio::null())
                                        .stdout(Stdio::null())
                                        .stderr(Stdio::null())
                                        .spawn();
                                    if let Err(e) = r {
                                        app.registrar(format!("{}: {e}", c.programa.display()));
                                    }
                                }
                                Efecto::Releer => releer = true,
                            }
                        }
                    }
                }
            }
            while let Ok(e) = rx.try_recv() {
                releer |= app.evento(e);
            }
            if releer {
                pendiente = true;
            }
            if pendiente && !leyendo {
                pendiente = false;
                leyendo = true;
                let (w, tx_t) = (workspaces.clone(), tx_t.clone());
                std::thread::spawn(move || {
                    let _ = tx_t.send(Tablero::leer(&w, &duracion_ffmpeg));
                });
            }
            if let Ok(t) = rx_t.try_recv() {
                leyendo = false;
                match t {
                    Ok(t) => app.reemplazar(t),
                    Err(e) => app.registrar(format!("releer: {e}")),
                }
            }
        }
        Ok(())
    })();
    ratatui::restore();
    r.map_err(|e| e.to_string())
}
