//! Dónde está ffmpeg, y las banderas de video que comparten todas las salidas.

use std::process::Command;

use crate::EncodeError;

/// El ffmpeg a usar: `GUION_FFMPEG` si está puesto (un binario que no está en
/// PATH, como el de `imageio-ffmpeg`), o el de PATH.
pub fn ffmpeg() -> Result<String, EncodeError> {
    if let Some(p) = std::env::var_os("GUION_FFMPEG").filter(|p| !p.is_empty()) {
        return Ok(p.to_string_lossy().into_owned());
    }
    let out = Command::new("which")
        .arg("ffmpeg")
        .output()
        .map_err(EncodeError::Io)?;
    if !out.status.success() {
        return Err(EncodeError::NoFfmpeg);
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// La duración de un archivo de medios, en segundos, del encabezado de
/// ffmpeg (`Duration: 00:01:27.30`). La usa el tablero (R-0013) para la
/// columna de duración; no decodifica nada.
pub fn duration(path: &std::path::Path) -> Result<f64, EncodeError> {
    let ff = ffmpeg()?;
    // Sin salida, ffmpeg termina con error a propósito: solo se lee su encabezado.
    let out = Command::new(&ff)
        .args(["-hide_banner", "-nostdin", "-i"])
        .arg(path)
        .output()
        .map_err(EncodeError::Io)?;
    parse_duration(&String::from_utf8_lossy(&out.stderr)).ok_or_else(|| {
        EncodeError::InvalidInput(format!("{}: ffmpeg no dio una duración", path.display()))
    })
}

/// `Duration: HH:MM:SS.ss` → segundos (`N/A` → nada).
pub fn parse_duration(log: &str) -> Option<f64> {
    let resto = &log[log.find("Duration:")? + "Duration:".len()..];
    let campo = resto.trim_start().split(',').next()?.trim();
    let mut partes = campo.split(':');
    let h: f64 = partes.next()?.parse().ok()?;
    let m: f64 = partes.next()?.parse().ok()?;
    let s: f64 = partes.next()?.parse().ok()?;
    Some(h * 3600.0 + m * 60.0 + s)
}

/// Las banderas de video de toda salida (R-0005 AC1–AC3).
///
/// - **BT.709 de verdad y etiquetado (AC1, AC2).** Sin `out_color_matrix`,
///   ffmpeg pasa de RGB a YUV con la matriz BT.601; sin etiquetas, el
///   teléfono decodifica un video HD con BT.709. Las dos cosas juntas mueven
///   el color: un oro (227, 169, 58) llegaba como (232, 167, 50). Se
///   convierte con BT.709, en rango de TV, y se etiqueta.
/// - **`+faststart` siempre (AC3).** El índice (`moov`) va antes de los
///   datos (`mdat`), así el video empieza a reproducirse sin descargarse
///   entero. Antes solo se ponía cuando había audio.
pub const VIDEO: &[&str] = &[
    "-c:v",
    "libx264",
    "-pix_fmt",
    "yuv420p",
    "-crf",
    "18",
    "-movflags",
    "+faststart",
    "-vf",
    "scale=out_color_matrix=bt709:out_range=tv,format=yuv420p",
    "-colorspace",
    "bt709",
    "-color_primaries",
    "bt709",
    "-color_trc",
    "bt709",
];

/// Los fps como los quiere ffmpeg: entero si lo es, decimal si no.
pub(crate) fn fps_arg(fps: f64) -> String {
    if (fps - fps.round()).abs() < 1e-9 {
        format!("{}", fps as i64)
    } else {
        format!("{fps}")
    }
}

/// Valida los fps (finitos y positivos).
pub(crate) fn check_fps(fps: f64) -> Result<(), EncodeError> {
    if fps.is_finite() && fps > 0.0 {
        Ok(())
    } else {
        Err(EncodeError::InvalidInput(format!(
            "fps debe ser finito y > 0, no {fps}"
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::parse_duration;

    #[test]
    fn la_duracion_sale_del_encabezado() {
        let log = "Input #0, mov,mp4,m4a,3gp,3g2,mj2, from 'x.mp4':\n  Duration: 00:01:27.30, start: 0.000000, bitrate: 2154 kb/s\n";
        assert_eq!(parse_duration(log), Some(87.3));
        assert_eq!(parse_duration("  Duration: N/A, bitrate: N/A"), None);
        assert_eq!(parse_duration("nada"), None);
    }
}
