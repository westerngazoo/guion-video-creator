use std::io::{self, Write};
use std::path::Path;

use crate::postfx::process_frame;

/// Encuentra dónde empiezan los pixeles de un P6 binario.
///
/// Una cabecera P6 tiene TRES renglones — `P6`, las dimensiones y el
/// valor máximo — y esta función devolvía el final del SEGUNDO, o sea que
/// se comía `255\n` como si fueran pixeles. Cuatro bytes de corrimiento:
/// la imagen entera se recorría un pixel y, como 4 no es múltiplo de 3,
/// los canales R/G/B quedaban ROTADOS. Todo cuadro que pasara por
/// `postfx_dir` salía con los colores cambiados de lugar.
///
/// Llevaba oculto porque el fondo era negro: (0,0,0) rotado sigue siendo
/// (0,0,0). Se vio el día que el fondo pasó a ser el papel de la marca y
/// el crema (242,230,208) salió como (208,242,230).
fn parse_p6_header(data: &[u8]) -> Option<(usize, u32, u32)> {
    if !data.starts_with(b"P6") {
        return None;
    }
    let mut line = 0;
    let mut start = 0;
    let mut dims = None;
    for (i, b) in data.iter().enumerate() {
        if *b == b'\n' {
            line += 1;
            match line {
                1 => start = i + 1,
                2 => {
                    let txt = std::str::from_utf8(&data[start..i]).ok()?;
                    let mut parts = txt.split_whitespace();
                    let w: u32 = parts.next()?.parse().ok()?;
                    let h: u32 = parts.next()?.parse().ok()?;
                    dims = Some((w, h));
                }
                // El tercero cierra el valor máximo; DESPUÉS de él
                // empiezan los pixeles.
                _ => {
                    let (w, h) = dims?;
                    return Some((i + 1, w, h));
                }
            }
        }
    }
    None
}

/// Read a binary P6 PPM into RGB bytes.
pub fn read_ppm(path: &Path) -> io::Result<(u32, u32, Vec<u8>)> {
    let data = std::fs::read(path)?;
    let (header_end, w, h) = parse_p6_header(&data)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "cabecera PPM inválida"))?;
    let rgb = data[header_end..].to_vec();
    Ok((w, h, rgb))
}

/// Apply theme post-fx to every `frame_%05d.ppm` in `dir`.
pub fn postfx_dir(dir: &Path, count: usize, grain: f64, halftone_alpha: u8) -> io::Result<()> {
    for i in 0..count {
        let path = dir.join(format!("frame_{:05}.ppm", i));
        let (w, h, mut rgb) = read_ppm(&path)?;
        process_frame(&mut rgb, w, h, grain, halftone_alpha);
        write_ppm(&path, w, h, &rgb)?;
    }
    Ok(())
}

pub fn write_ppm(path: &Path, w: u32, h: u32, rgb: &[u8]) -> io::Result<()> {
    let mut f = std::fs::File::create(path)?;
    write!(f, "P6\n{} {}\n255\n", w, h)?;
    f.write_all(rgb)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// R-0008 OQ-1. Lo que se escribe es lo que se lee, byte por byte.
    ///
    /// Ésta es la prueba que faltaba y que habría evitado que todo cuadro
    /// con post-fx saliera con los canales rotados: `parse_p6_header`
    /// devolvía el final del renglón de dimensiones en vez del final del
    /// valor máximo, y `postfx_dir` leía-procesaba-escribía, así que el
    /// corrimiento se horneaba en el archivo en cada pasada.
    #[test]
    fn lo_que_se_escribe_es_lo_que_se_lee() {
        let d = std::env::temp_dir().join("guion-ppm-io");
        std::fs::create_dir_all(&d).expect("mkdir");
        let f = d.join("ida-y-vuelta.ppm");
        // Tres pixeles distintos y asimétricos: un gris uniforme habría
        // pasado con los canales rotados.
        let rgb: Vec<u8> = vec![242, 230, 208, 11, 16, 32, 1, 2, 3];
        write_ppm(&f, 3, 1, &rgb).expect("escribir");
        let (w, h, leido) = read_ppm(&f).expect("leer");
        assert_eq!((w, h), (3, 1));
        assert_eq!(leido, rgb, "el archivo no devolvió lo que se le puso");
    }

    /// Y el post-fx no puede mover un pixel de lugar: aclara, ensucia y
    /// estampa puntos, pero la geometría es la misma. Un corrimiento se
    /// ve aquí como un cambio en el primer pixel muy por encima de lo que
    /// el grano puede explicar.
    #[test]
    fn el_postfx_no_corre_la_imagen() {
        let d = std::env::temp_dir().join("guion-ppm-io");
        std::fs::create_dir_all(&d).expect("mkdir");
        let f = d.join("frame_00000.ppm");
        let papel = [242u8, 230, 208];
        let rgb: Vec<u8> = papel.iter().copied().cycle().take(64 * 64 * 3).collect();
        write_ppm(&f, 64, 64, &rgb).expect("escribir");
        postfx_dir(&d, 1, 0.04, 26).expect("postfx");
        let (_, _, despues) = read_ppm(&f).expect("leer");
        assert_eq!(despues.len(), rgb.len(), "cambió de tamaño");

        // Se mide LEJOS de la trama: los puntos van cada 18 px con radio
        // 2.3, así que (9,9) cae entre cuatro. Sobre un punto el papel
        // baja 23 niveles por diseño, y medir ahí confundiría «el
        // post-fx funciona» con «la imagen se corrió».
        let entre = ((9 * 64) + 9) * 3;
        for c in 0..3 {
            let d = (despues[entre + c] as i16 - papel[c] as i16).abs();
            assert!(d <= 2, "canal {c} se movió {d} niveles entre puntos");
        }

        // Y sobre un punto sí cambia, con el valor que dicta la mezcla:
        // (1-26/255)·papel + (26/255)·20. Si los canales estuvieran
        // rotados, este trío no cuadraría con ESTE papel.
        let esperado: Vec<u8> = papel
            .iter()
            .map(|v| {
                let t = 26.0 / 255.0;
                ((1.0 - t) * f64::from(*v) + t * 20.0).round() as u8
            })
            .collect();
        for c in 0..3 {
            let d = (despues[c] as i16 - esperado[c] as i16).abs();
            assert!(
                d <= 2,
                "el punto de trama dio {} y la mezcla pide {}",
                despues[c],
                esperado[c]
            );
        }
    }
}
