//! El audio al nivel de las plataformas: −14 LUFS y −1 dBTP (R-0005 AC6).
//!
//! Instagram, YouTube y TikTok normalizan a unos −14 LUFS integrados (EBU
//! R128). Un reel más bajo suena flojo junto a los demás; uno más alto lo
//! bajan, y si sus picos llegaban a 0 dBFS, distorsiona al pasar a AAC. La
//! meta es la de las plataformas, con un pico real de −1 dBTP como techo.
//!
//! Un `loudnorm` solo no basta con audio de mucho factor de cresta (golpes
//! de tambor de unos 23 dB): en AAC su pico real saltaba entre −1.2 y −0.1
//! dBTP, y el nivel quedaba entre −14.3 y −14.8. Así que se hace por pasos,
//! midiendo cada vez:
//!
//! 1. `loudnorm` en dos pasadas (mide y aplica) lleva el audio cerca de la meta;
//! 2. una ganancia exacta corrige lo que falte, y un limitador corta los
//!    picos a 192 kHz, donde el pico de muestra ya es casi el pico real;
//! 3. se codifica en AAC y se mide. Si el pico pasó del techo, el limitador
//!    baja; si el nivel se salió de la tolerancia, la ganancia se corrige.
//!    Hasta 8 vueltas.
//!
//! Determinista: la misma entrada sigue el mismo camino y da los mismos
//! bytes. Si no converge, falla y no escribe nada.

use std::path::Path;
use std::process::Command;

use crate::ffmpeg::ffmpeg;
use crate::stream::temporal;
use crate::EncodeError;

/// Lo que mide EBU R128 de un archivo.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Loudness {
    /// Sonoridad integrada, LUFS.
    pub integrated_lufs: f64,
    /// Pico real (true peak), dBTP.
    pub true_peak_dbtp: f64,
}

/// A dónde se lleva el audio.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LoudnessTarget {
    pub integrated_lufs: f64,
    /// Cuánto se puede alejar de la meta, en LU, para aceptarse.
    pub tolerance_lu: f64,
    /// El pico real más alto que se acepta, dBTP.
    pub max_true_peak_dbtp: f64,
}

impl LoudnessTarget {
    /// La meta de los reels: −14 LUFS ± 0.5, pico real de −1 dBTP o menos.
    pub const REELS: LoudnessTarget = LoudnessTarget {
        integrated_lufs: -14.0,
        tolerance_lu: 0.5,
        max_true_peak_dbtp: -1.0,
    };

    fn accepts(&self, m: &Loudness) -> bool {
        (m.integrated_lufs - self.integrated_lufs).abs() <= self.tolerance_lu
            && m.true_peak_dbtp <= self.max_true_peak_dbtp
    }
}

const VUELTAS: usize = 8;

fn run(ff: &str, args: &[&str]) -> Result<String, EncodeError> {
    let mut cmd = Command::new(ff);
    cmd.args(["-hide_banner", "-nostats", "-nostdin"])
        .args(args);
    let display = format!("{cmd:?}");
    let out = cmd.output()?;
    if !out.status.success() {
        return Err(EncodeError::Ffmpeg {
            status: out.status.code().unwrap_or(-1),
            cmd: display,
        });
    }
    Ok(String::from_utf8_lossy(&out.stderr).into_owned())
}

fn path_str(p: &Path) -> Result<&str, EncodeError> {
    p.to_str()
        .ok_or_else(|| EncodeError::InvalidInput(format!("ruta que no es UTF-8: {}", p.display())))
}

/// La sonoridad integrada y el pico real de un archivo con audio.
pub fn measure(path: &Path) -> Result<Loudness, EncodeError> {
    let ff = ffmpeg()?;
    let log = run(
        &ff,
        &[
            "-i",
            path_str(path)?,
            "-af",
            "ebur128=peak=true",
            "-f",
            "null",
            "-",
        ],
    )?;
    parse_ebur128(&log).ok_or_else(|| {
        EncodeError::InvalidInput(format!(
            "ebur128 no dio un resumen para {} (¿tiene audio?)",
            path.display()
        ))
    })
}

/// El resumen de `ebur128`: `I: -14.0 LUFS` y, bajo `True peak:`, `Peak: -1.3 dBFS`.
fn parse_ebur128(log: &str) -> Option<Loudness> {
    let resumen = &log[log.rfind("Summary:")?..];
    let integrated_lufs = numero_tras(resumen, "I:")?;
    let pico = &resumen[resumen.find("True peak")?..];
    let true_peak_dbtp = numero_tras(pico, "Peak:")?;
    Some(Loudness {
        integrated_lufs,
        true_peak_dbtp,
    })
}

fn numero_tras(texto: &str, clave: &str) -> Option<f64> {
    let resto = texto[texto.find(clave)? + clave.len()..].trim_start();
    let fin = resto
        .find(|c: char| !(c.is_ascii_digit() || c == '-' || c == '.' || c == '+'))
        .unwrap_or(resto.len());
    resto[..fin].parse().ok()
}

/// El valor de `"clave" : "valor"` en el JSON que imprime `loudnorm`.
fn campo_json(json: &str, clave: &str) -> Option<String> {
    let k = format!("\"{clave}\"");
    let resto = &json[json.find(&k)? + k.len()..];
    let resto = &resto[resto.find('"')? + 1..];
    Some(resto[..resto.find('"')?].to_string())
}

/// Junta `video` (su pista de video se copia tal cual) con `audio` llevado a
/// `target`, y escribe `out`. Devuelve lo que mide la salida.
pub fn mux_at_loudness(
    video: &Path,
    audio: &Path,
    out: &Path,
    target: LoudnessTarget,
) -> Result<Loudness, EncodeError> {
    let ff = ffmpeg()?;
    let dir = std::env::temp_dir().join(format!(
        "guion-loudness-{}-{}",
        std::process::id(),
        out.file_name()
            .map(|n| n.to_string_lossy())
            .unwrap_or_default()
    ));
    std::fs::create_dir_all(&dir)?;
    let resultado = mezclar(&ff, video, audio, out, target, &dir);
    let _ = std::fs::remove_dir_all(&dir);
    resultado
}

fn mezclar(
    ff: &str,
    video: &Path,
    audio: &Path,
    out: &Path,
    target: LoudnessTarget,
    dir: &Path,
) -> Result<Loudness, EncodeError> {
    let (video_s, audio_s) = (path_str(video)?, path_str(audio)?);
    // 1) loudnorm en dos pasadas, a WAV de 48 kHz; su techo, medio dB abajo del final
    let obj = format!(
        "I={}:TP={}:LRA=11",
        target.integrated_lufs,
        target.max_true_peak_dbtp - 0.5
    );
    let log = run(
        ff,
        &[
            "-i",
            audio_s,
            "-af",
            &format!("loudnorm={obj}:print_format=json"),
            "-f",
            "null",
            "-",
        ],
    )?;
    let json = &log[log.rfind('{').unwrap_or(0)..];
    let campo = |k: &str| {
        campo_json(json, k)
            .ok_or_else(|| EncodeError::InvalidInput(format!("loudnorm no dio {k} para {audio_s}")))
    };
    let filtro = format!(
        "loudnorm={obj}:measured_I={}:measured_TP={}:measured_LRA={}:measured_thresh={}:offset={}:linear=true",
        campo("input_i")?,
        campo("input_tp")?,
        campo("input_lra")?,
        campo("input_thresh")?,
        campo("target_offset")?
    );
    let norm = dir.join("norm.wav");
    let norm_s = path_str(&norm)?;
    run(
        ff,
        &[
            "-y",
            "-i",
            audio_s,
            "-af",
            &filtro,
            "-ar",
            "48000",
            "-c:a",
            "pcm_s16le",
            norm_s,
        ],
    )?;
    let nivel = measure(&norm)?.integrated_lufs;

    // 2) y 3) ganancia exacta y limitador a 192 kHz; se mide ya en AAC y se corrige
    let (mut ganancia, mut techo) = (
        target.integrated_lufs - nivel,
        target.max_true_peak_dbtp - 1.0,
    );
    let prueba = dir.join("prueba.mp4");
    let prueba_s = path_str(&prueba)?;
    let mut ultima = None;
    for _ in 0..VUELTAS {
        let af = format!(
            "volume={ganancia:.3}dB,aresample=192000,alimiter=limit={:.4}:attack=1:release=50:level=disabled,aresample=48000",
            10f64.powf(techo / 20.0)
        );
        run(
            ff,
            &[
                "-y",
                "-i",
                video_s,
                "-i",
                norm_s,
                "-map",
                "0:v",
                "-map",
                "1:a",
                "-c:v",
                "copy",
                "-af",
                &af,
                "-c:a",
                "aac",
                "-b:a",
                "192k",
                "-ar",
                "48000",
                "-shortest",
                "-movflags",
                "+faststart",
                prueba_s,
            ],
        )?;
        let m = measure(&prueba)?;
        if target.accepts(&m) {
            if let Some(parent) = out.parent().filter(|p| !p.as_os_str().is_empty()) {
                std::fs::create_dir_all(parent)?;
            }
            // copia y rename: el temporal puede estar en otro disco
            let tmp = temporal(out);
            std::fs::copy(&prueba, &tmp)?;
            std::fs::rename(&tmp, out)?;
            return Ok(m);
        }
        if m.true_peak_dbtp > target.max_true_peak_dbtp {
            techo -= (m.true_peak_dbtp - target.max_true_peak_dbtp) + 0.2;
        }
        if (m.integrated_lufs - target.integrated_lufs).abs() > target.tolerance_lu {
            ganancia += target.integrated_lufs - m.integrated_lufs;
        }
        ultima = Some(m);
    }
    let m = ultima.expect("al menos una vuelta");
    Err(EncodeError::Loudness {
        reached: m,
        target,
        rounds: VUELTAS,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lee_el_resumen_de_ebur128() {
        let log = "[Parsed_ebur128_0 @ 0x1] t: 1.2 M: -20.0 S: -30.0 I: -25.0 LUFS\n\
                   [Parsed_ebur128_0 @ 0x1] Summary:\n\n  Integrated loudness:\n    I:         -14.2 LUFS\n    Threshold: -24.5 LUFS\n\n  \
                   Loudness range:\n    LRA:         3.1 LU\n\n  True peak:\n    Peak:        -1.3 dBFS\n";
        let m = parse_ebur128(log).expect("resumen");
        assert_eq!(m.integrated_lufs, -14.2);
        assert_eq!(m.true_peak_dbtp, -1.3);
    }

    #[test]
    fn lee_un_campo_del_json_de_loudnorm() {
        let json = "{\n\t\"input_i\" : \"-27.61\",\n\t\"input_tp\" : \"-4.47\",\n\t\"target_offset\" : \"0.40\"\n}";
        assert_eq!(campo_json(json, "input_i").as_deref(), Some("-27.61"));
        assert_eq!(campo_json(json, "target_offset").as_deref(), Some("0.40"));
        assert_eq!(campo_json(json, "nada"), None);
    }
}
