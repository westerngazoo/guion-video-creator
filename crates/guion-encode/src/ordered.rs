//! Render en paralelo, escritura en orden (R-0005 AC5).
//!
//! Cada cuadro es una función pura de su índice, así que se pueden
//! calcular en varios hilos a la vez. El cuadro `i` lo hace el hilo
//! `i mod n`, y el hilo principal los recibe en orden y los pasa al
//! [`Encoder`]. Cada hilo tiene una cola de dos: la memoria no crece con el
//! largo del video.
//!
//! El MP4 sale **idéntico byte por byte** con uno o con n hilos: ffmpeg
//! recibe los mismos bytes en el mismo orden.

use std::path::Path;
use std::sync::mpsc;

use crate::stream::{Encoder, VideoSpec};
use crate::EncodeError;

/// Escribe `frames` cuadros en `out`, calculando cada uno con
/// `render(i)` en `threads` hilos. Devuelve los cuadros escritos.
///
/// Si `render` falla en el cuadro `i`, el error dice cuál y no queda archivo.
pub fn encode_ordered<F, E>(
    out: &Path,
    spec: VideoSpec,
    frames: usize,
    threads: usize,
    render: F,
) -> Result<usize, EncodeError>
where
    F: Fn(usize) -> Result<Vec<u8>, E> + Sync,
    E: std::fmt::Display + Send,
{
    if frames == 0 {
        return Err(EncodeError::InvalidInput(
            "un video sin cuadros no es un video".into(),
        ));
    }
    let threads = threads.clamp(1, frames);
    let mut enc = Encoder::start(out, spec)?;
    let render = &render;
    std::thread::scope(|s| -> Result<(), EncodeError> {
        let mut colas = Vec::with_capacity(threads);
        for h in 0..threads {
            let (tx, rx) = mpsc::sync_channel::<Result<Vec<u8>, String>>(2);
            colas.push(rx);
            s.spawn(move || {
                for i in (h..frames).step_by(threads) {
                    let r = render(i).map_err(|e| e.to_string());
                    let fallo = r.is_err();
                    if tx.send(r).is_err() || fallo {
                        return; // el principal ya se fue, o ya no hay qué hacer
                    }
                }
            });
        }
        for i in 0..frames {
            let r = colas[i % threads].recv().map_err(|_| EncodeError::Render {
                frame: i,
                message: "el hilo que lo calculaba murió".into(),
            })?;
            let rgb = r.map_err(|message| EncodeError::Render { frame: i, message })?;
            enc.push(&rgb)?;
        }
        Ok(())
        // al salir con error, `colas` se suelta y los hilos dejan de mandar
    })?;
    enc.finish()
}
