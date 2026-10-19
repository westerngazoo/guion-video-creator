//! The gate profile derived from the contract (R-0014 AC2).
//!
//! `guion reel check` reads a brand profile (R-0012 AC1). An account made with
//! `guion cuenta nueva` does not write it by hand: it is derived from
//! `cuenta.toml` into `brand/reel-perfil.toml`, and `guion cuenta perfil`
//! derives it again (or, with `--check`, fails if it drifted). The text is a
//! pure function of the contract, so the same contract gives the same bytes.

use crate::contrato::Cuenta;
use crate::texto::{toml_lista, toml_str};

/// Where the profile lives, relative to the account's root.
pub const RUTA: &str = "brand/reel-perfil.toml";

/// A path of the contract (relative to the root) as the profile sees it
/// (relative to `brand/`).
fn desde_brand(p: &str) -> String {
    if std::path::Path::new(p).is_absolute() {
        p.to_string()
    } else {
        format!("../{p}")
    }
}

fn num(x: f64) -> String {
    // always a TOML float: 52 → 52.0, -14 → -14.0
    if x.fract() == 0.0 && x.abs() < 1e15 {
        format!("{x:.1}")
    } else {
        format!("{x}")
    }
}

fn par(a: [f64; 2]) -> String {
    format!("[{}, {}]", num(a[0]), num(a[1]))
}

/// The profile's text.
pub fn perfil(c: &Cuenta) -> String {
    let m = &c.marca;
    let s = &m.subtitulo;
    let g = &c.compuerta;
    let mut o = String::new();
    o.push_str(&format!(
        "# The gate profile of {} (`guion reel check`, guion-video-creator R-0012).\n\
# DERIVED from cuenta.toml by `guion cuenta perfil` (R-0014): do not edit it by\n\
# hand; change cuenta.toml and derive it again. `guion cuenta perfil --check`\n\
# fails when this file and the contract disagree.\n\n",
        c.cuenta.nombre
    ));
    o.push_str(&format!(
        "[brand]\nname = {}\n\n",
        toml_str(&c.cuenta.nombre)
    ));
    o.push_str(&format!(
        "[canvas]\nwidth = {}\nheight = {}\nfps = {}\n\n",
        m.lienzo.ancho,
        m.lienzo.alto,
        num(m.lienzo.fps)
    ));
    o.push_str("[tokens]\n");
    for (k, v) in &m.tokens {
        let clave = if crate::contrato::slug_valido(k) {
            k.clone()
        } else {
            toml_str(k)
        };
        o.push_str(&format!("{} = [{}, {}, {}]\n", clave, v[0], v[1], v[2]));
    }
    o.push_str(&format!(
        "\n[roles]\nbackground = {}\nred = {}\nfocus = {}\n\n",
        toml_lista(&m.roles.fondo),
        toml_lista(&m.roles.acento),
        toml_lista(&m.roles.foco)
    ));
    o.push_str(&format!(
        "[subtitle]\nfont = {}\nsize = {}\nsize_unit = \"em\"\nmax_width = {}\ncenter_x = {}\n\
baseline = {}\nband = {}\nmax_words = {}\nforbidden = {}\nforbid_digits = {}\nlong_line = {}\nfade = {}\n\n",
        toml_str(&desde_brand(&m.fuentes.texto)),
        num(s.tam),
        num(s.ancho),
        num(f64::from(m.lienzo.ancho) / 2.0),
        num(s.base),
        par(s.franja),
        s.palabras,
        toml_str(&s.prohibidos),
        s.sin_cifras,
        num(s.linea_larga),
        num(s.fundido)
    ));
    let except: Vec<String> = m
        .seguro
        .excepto
        .iter()
        .map(|r| {
            format!(
                "[{}, {}, {}, {}]",
                num(r[0]),
                num(r[1]),
                num(r[2]),
                num(r[3])
            )
        })
        .collect();
    o.push_str(&format!(
        "[safe]\nx = {}\ny = {}\nexcept = [{}]\n\n",
        par(m.seguro.x),
        par(m.seguro.y),
        except.join(", ")
    ));
    o.push_str(&format!(
        "[motion]\nmax_still_with_voice = {}\nclosing_hold = {}\n\n",
        num(g.quieto_con_voz),
        num(g.cierre_quieto)
    ));
    o.push_str(
        "[scan]\nstride = 5\ntolerance = 30.0\nmerge = 48\nmin_area = 400\nfocus_min_area = 3000\nmin_stray = 40\n\n",
    );
    o.push_str(&format!(
        "[audio]\nlufs = {}\ntolerance = {}\ntrue_peak = {}\nshort_gap = {}\ngap_max_db = {}\nbeat_events = {}\n\n",
        num(g.lufs),
        num(g.tolerancia_lufs),
        num(g.pico),
        num(g.hueco_corto),
        num(g.hueco_max_db),
        toml_lista(&g.eventos_de_beat)
    ));
    let mut interno = g.interno.clone();
    interno.extend(c.voz.prohibido.iter().cloned());
    o.push_str(&format!(
        "[caption]\nmax_hashtags = {}\nmax_chars = {}\nunits = {}\ninternal = {}\n",
        g.max_hashtags,
        g.max_caracteres,
        toml_lista(&g.unidades),
        toml_lista(&interno)
    ));
    o
}
