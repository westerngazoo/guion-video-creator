//! The format of a post and its YouTube deliverables (R-0016).
//!
//! - [`formato`]: `--formato`, else `reel.toml`'s `[reel] formato`, else
//!   `escena.toml`'s `formato`, else the reel; the two files must agree;
//! - [`textos`]: `youtube.md` → `titulo.txt` + `descripcion.txt`, the
//!   chapters' anchors resolved against the voice and `eventos.json`;
//! - [`subtitulos`]: `voz.json` → `<name>.srt`.

use std::path::{Path, PathBuf};

use guion_gadget::{Escena, Tiempos};
use guion_publish::youtube::{self as yt, Capitulo};
use guion_reel::{Formato, Narration};

use super::config::ReelToml;

/// The post's format.
pub fn formato(
    post: &Path,
    reel: Option<&ReelToml>,
    flag: Option<Formato>,
) -> Result<Formato, String> {
    if let Some(f) = flag {
        return Ok(f);
    }
    let de_reel = reel
        .and_then(|r| r.reel.formato.as_deref())
        .map(Formato::de)
        .transpose()
        .map_err(|e| format!("{}: {e}", post.join("reel.toml").display()))?;
    let de_escena = if post.join(guion_gadget::escena::ARCHIVO).is_file() {
        Escena::load(post)?.formato
    } else {
        None
    };
    match (de_reel, de_escena) {
        (Some(a), Some(b)) if a != b => Err(format!(
            "{}: reel.toml says formato = «{a}», escena.toml «{b}»",
            post.display()
        )),
        (a, b) => Ok(a.or(b).unwrap_or(Formato::Reel)),
    }
}

/// The named instants of `eventos.json` and its length.
fn eventos(path: &Path) -> Result<(Vec<(String, f64)>, f64), String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let v: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    let dur = v
        .get("duracion")
        .and_then(|d| d.as_f64())
        .ok_or_else(|| format!("{}: no «duracion»", path.display()))?;
    let evs = v
        .get("eventos")
        .and_then(|e| e.as_object())
        .map(|o| {
            o.iter()
                .filter_map(|(k, t)| t.as_f64().map(|t| (k.clone(), t)))
                .collect()
        })
        .unwrap_or_default();
    Ok((evs, dur))
}

/// The title and the description from `youtube.md`, written into `dir`.
pub fn textos(post: &Path, r: &ReelToml, dir: &Path) -> Result<(PathBuf, PathBuf), String> {
    let md = post.join(
        r.youtube
            .as_ref()
            .map(|y| y.texto.as_str())
            .unwrap_or("youtube.md"),
    );
    let fuente = yt::parse_file(&md).map_err(|e| e.to_string())?;
    let (evs, dur) = eventos(&post.join("eventos.json"))?;
    let vj = post.join("voz.json");
    let voz = if vj.is_file() {
        Some(Narration::load(&vj).map_err(|e| e.to_string())?)
    } else {
        None
    };
    let tiempos = Tiempos::de_voz(voz.as_ref(), dur).con_eventos(&evs);
    let caps = fuente
        .capitulos
        .iter()
        .map(|(ancla, titulo)| {
            Ok(Capitulo {
                inicio: tiempos
                    .resolver(ancla)
                    .map_err(|e| format!("{}: chapter «{titulo}»: {e}", md.display()))?,
                titulo: titulo.clone(),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let ex =
        yt::export(&fuente, &caps, dur, &md.display().to_string()).map_err(|e| e.to_string())?;
    ex.write_to(dir).map_err(|e| e.to_string())
}

/// `<dir>/<name>.srt` from `voz.json`.
pub fn subtitulos(post: &Path, name: &str, dir: &Path) -> Result<PathBuf, String> {
    let voz = Narration::load(&post.join("voz.json")).map_err(|e| e.to_string())?;
    let out = dir.join(format!("{name}.srt"));
    std::fs::write(&out, guion_reel::youtube::srt(&voz))
        .map_err(|e| format!("{}: {e}", out.display()))?;
    Ok(out)
}
